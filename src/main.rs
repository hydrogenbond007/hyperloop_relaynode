use dotenv::dotenv;
use std::sync::Arc;
use std::{env, thread, time};
use std::time::{SystemTime, UNIX_EPOCH};
use regex::Regex;
use hex::FromHex;
use web3::futures::StreamExt;
use web3::types::{FilterBuilder, Log};
use web3::transports::WebSocket;
use ethers::prelude::*;
use ethers::abi::Tokenizable;
use ethers::providers::{Provider, Http};
use ethabi::{decode, ParamType};
mod ed25519;

type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

abigen!(
    BridgeRx,
    "./abi/bridge_rx.json"
);

abigen!(
    BridgeTx,
    "./abi/bridge_tx.json"
);

struct TxnMeta {
    from: Address,
    amount: U256
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let batch_interval_time = 30; // (seconds)

    let mut log_bytes: Vec<Bytes> = vec![];
    let mut txn_meta: Vec<TxnMeta> = vec![];

    let chain_id: u64 = env::var("DEST_CHAIN_ID")?.parse::<u64>().expect("Invalid conversion to u64");
    let dest_contract_address = env::var("DEST_CONTRACT_ADDR")?.parse::<Address>()?;

    let provider = Provider::<Http>::try_from(env::var("DEST_RPC_URL")?.as_str())?;
    // ^ let wallet: Wallet<k256::ecdsa::SigningKey> = env::var("PRIVATE_KEY")?.parse::<Wallet<k256::ecdsa::SigningKey>>()?.with_chain_id(chain_id);

    // CMD line args 
    let args: Vec<String> = env::args().collect();
    let wallet: Wallet<k256::ecdsa::SigningKey> = args[1].parse::<Wallet<k256::ecdsa::SigningKey>>()?.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider.clone(), wallet.clone());

    let source_rpc_url = env::var("SOURCE_RPC_URL")?;
    let source_contract_address = env::var("SOURCE_CONTRACT_ADDR")?;
    let transport = WebSocket::new(&source_rpc_url).await?;
    let web3 = web3::Web3::new(transport);

    let filter = FilterBuilder::default()
        .address(vec![source_contract_address.parse().unwrap()])
        .build();

    let mut stream = web3.eth_subscribe().subscribe_logs(filter).await?;

    let interval = std::time::Duration::from_secs(batch_interval_time);
    let mut interval = tokio::time::interval(interval);

    loop{
        tokio::select! {
            log = stream.next() => {
                if let Some(Ok(log)) = log {
                    //println!("Received event: {:?}", log);
                    handle_log(&client, &dest_contract_address, log, &mut txn_meta, &mut log_bytes).await?;
                }
            },
            _ = interval.tick() => {
                if !log_bytes.is_empty() {
                    process_log(&client, &source_contract_address.parse::<Address>()?, &dest_contract_address, &mut txn_meta, &mut log_bytes).await?;
                }
                else {
                    println!("~ No Transactions Pending ~\n");
                }
            }
        }
    }
}

async fn handle_log(client: &Client, contract_addr: &H160, log: Log, txn_meta: &mut Vec<TxnMeta>, log_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>>{
    let log_string = format!("{:?}", log);
    let re1 = Regex::new(r"topics: \[([^,]+), ([^,]+), ([^,]+), ([^,]+)]").unwrap();

    if let Some(captures) = re1.captures(log_string.as_str()) {
        let topic1 = captures.get(2).unwrap().as_str().trim();
        let topic2 = captures.get(3).unwrap().as_str().trim();
        let topic3 = captures.get(4).unwrap().as_str().trim();

        let action_id = topic1.parse::<Bytes>()?;
        let to = topic2.parse::<Bytes>()?;
        let amount = topic3.parse::<Bytes>()?;

        //Below code parses event data and calculates curr timestamp - block.timestamp

        let data_re = Regex::new(r#"data: Bytes\("0x([^"]+?)"\)"#).unwrap();
        let mut data_str: &str = "";

        if let Some(captures) = data_re.captures(log_string.as_str()) {
            data_str = captures.get(1).unwrap().as_str();
        } else {
            println!("handle_log: no match found for data_re");
        }

        let action_id_bytes = Vec::from_hex(&topic1[2..]).unwrap();
        let action_id_byte_array: &[u8] = &action_id_bytes.as_slice();
        let action_id_val = decode(&[ParamType::Uint(256)], action_id_byte_array)?;

        let event_bytes = Vec::from_hex(data_str).unwrap();
        let event_byte_array: &[u8] = event_bytes.as_slice();
        let decode_params = [ParamType::Uint(256), ParamType::Address, ParamType::Uint(256), ParamType::Uint(256), ParamType::Uint(256)];
        let event_params = decode(&decode_params, event_byte_array)?;

        let now = SystemTime::now();
        let unix_timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs().into_token().into_uint();
        let block_timestamp = event_params[0].clone().into_uint();

        match block_timestamp{
            Some(x) => {
                match unix_timestamp {
                    Some(y) => println!("* (id: {:?}) Source Contract -> Bridge Node (in seconds): {:?}\n", action_id_val[0], y - x),
                    None => println!("handle_log: invalid current timestamp"),
                }
            },
            None => println!("handle_log: Invalid block.timestamp"),
        }

        //Store in txn_meta
        let amount_bytes = Vec::from_hex(&topic3[2..]).unwrap();
        let amount_byte_array: &[u8] = &amount_bytes.as_slice();
        let amount_val = decode(&[ParamType::Uint(256)], amount_byte_array)?;

        let curr_txn_meta = TxnMeta {
            from: event_params[1].clone().into_address().expect("handle_log: invalid address type"),
            amount: amount_val[0].clone().into_uint().expect("handle_log: invalid uint type"),
        };
        txn_meta.push(curr_txn_meta);

        //Call pure function to get transaction in bytes form
        get_transaction_bytes(client, contract_addr, action_id, to, amount, log_bytes).await?;
    } else {
        println!("handle_log: no matches found");
    }
    //println!("Received event: {:?}", log);

    Ok(())
}

async fn process_log(client: &Client, source_contract_addr: &H160, dest_contract_addr: &H160, txn_meta: &mut Vec<TxnMeta>, log_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    println!("-----------------------BATCH CALL BEGIN-----------------------\n");

    //Call pure function to get final message in bytes form
    let msg = get_message_bytes(client, dest_contract_addr, log_bytes).await?;

    //Sign the message
    // ^ let private_key_hex = env::var("PRIVATE_KEY")?;
    // ^ let public_key_hex = env::var("PUBLIC_KEY")?;
    
    let args: Vec<String> = env::args().collect();
    let private_key_hex = &args[1];
    let public_key_hex = &args[2];
    println!("- SIGNER: {}", {public_key_hex});

    let sig = ed25519::sign_message(private_key_hex.as_str(), &msg)?;
    println!("- SIG: {}", sig);

    //Send to destination contract
    let timeout: u64 = args[3].parse().unwrap();
    let ten_millis = time::Duration::from_millis(timeout);
    thread::sleep(ten_millis);

    //Send to destination contract
    let mut success = false;

    let timeout_duration = time::Duration::from_secs(args[4].parse().unwrap());
    let execute_message_future = execute_message(client, dest_contract_addr, public_key_hex.as_str(), sig, msg);

    match tokio::time::timeout(timeout_duration, execute_message_future).await {
        Ok(result) => {
            success = result?;
        }
        Err(_) => {
            println!("~ BATCH TIME LIMIT EXCEEDED ~");
            success = false;
        }
    }


    if !success {
        revert_txns(client, source_contract_addr, txn_meta).await?;
    }
    
    println!("\n------------------------BATCH CALL END------------------------\n");
    Ok(())
}

async fn get_transaction_bytes(client: &Client, contract_addr: &H160, action_id: Bytes, to: Bytes, amount: Bytes, log_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    println!("- TOPICS: {}, {}, {}\n", action_id, to, amount);
    
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));
    let result = contract.get_transaction_bytes(action_id, to, amount).call().await?;

    log_bytes.push(result);

    Ok(())
}

async fn get_message_bytes(client: &Client, contract_addr: &H160, log_bytes: &mut Vec<Bytes>) -> Result<Bytes, Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));
    let result = contract.get_message_bytes(log_bytes.to_vec()).call().await?;

    println!("- MSG BYTES: {:?}", result);
    log_bytes.clear();

    Ok(result)
}

async fn execute_message(client: &Client, contract_addr: &H160, pub_key_hex: &str, sig: Bytes, txn: Bytes) -> Result<(bool), Box<dyn std::error::Error>> {
    let txn_send_timestamp = get_current_time();

    let pub_key_bytes = Vec::from_hex(pub_key_hex).expect("execute_message: Invalid hex string");
    let mut signer: [u8; 32] = [0; 32];
    signer.copy_from_slice(&pub_key_bytes);

    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));

    let execute = contract.execute_message(signer, sig, txn);
    let result = execute.send().await;

    match result {
        Ok(tx_future) => {
            match tx_future.await {
                Ok(tx) => {
                    println!("\n- TRANSACTION RECEIPT: {}\n", serde_json::to_string(&tx)?);
                    let txn_complete_timestamp = get_current_time();
                    println!("- [Transaction completion timestamp] - [Node txn send timestamp] (in ms): {}", txn_complete_timestamp - txn_send_timestamp);
                    Ok(true)
                },
                Err(e) => {
                    println!("- TRANSACTION ERROR: {}", e);
                    Ok(false)
                }
            }
        },
        Err(e) => {
            println!("\n- TRANSACTION FAILED | REVERT: {}", e);
            Ok(false)
        }
    }
}

fn get_current_time() -> u64{
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let unix_timestamp_millis = duration_since_epoch.as_secs() * 1000 + u64::from(duration_since_epoch.subsec_millis());

    unix_timestamp_millis
}

async fn revert_txns(client: &Client, contract_addr: &H160, txn_meta: &mut Vec<TxnMeta>) -> Result<(),Box<dyn std::error::Error>> {
    println!("\n------------------------REVERT BEGIN-------------------------\n");
    let ten_millis = time::Duration::from_millis(100);

    let contract = BridgeTx::new(contract_addr.clone(), Arc::new(client.clone()));

    for i in 0..txn_meta.len() {
        let execute = contract.revert_txn(txn_meta[i].from, txn_meta[i].amount);
        let result = execute.send().await;

        match result {
            Ok(tx_future) => {
                match tx_future.await {
                    Ok(tx) => {
                        //println!("\nTRANSACTION RECEIPT: {}\n", serde_json::to_string(&tx)?);
                        println!("(To: {:?}) REVERT SUCCESS", txn_meta[i].from);
                    },
                    Err(e) => {
                        println!("REVERT TRANSACTION ERROR: {}", e);
                    }
                }
            },
            Err(e) => {
                println!("\nREVERT TRANSACTION FAILED | REVERT: {}\n", e);
            }
        }
        
        thread::sleep(ten_millis);
    }

    txn_meta.clear();
    Ok(())
}