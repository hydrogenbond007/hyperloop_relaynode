use dotenv::dotenv;
use std::sync::Arc;
use std::{env, thread, time};
use std::time::{SystemTime, UNIX_EPOCH};
use regex::Regex;
use hex::FromHex;
use hex_literal::hex;
use web3::futures::StreamExt;
use web3::types::{FilterBuilder, Log};
use web3::transports::WebSocket;
use ethers::prelude::*;
use ethers::abi::Tokenizable;
use ethers::providers::{Provider, Http};
use ethabi::{decode, encode, ParamType};
mod ed25519;

type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

abigen!(
    BridgeTx,
    "./abi/bridge_tx.json"
);

abigen!(
    BridgeRx,
    "./abi/bridge_rx.json"
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let batch_interval_time = 20; // (seconds)
    let batch_size_limit = 2;

    let mut log_bytes: Vec<Bytes> = vec![];
    let mut revert_bytes: Vec<Bytes> = vec![];

    let dest_chain_id: u64 = env::var("DEST_CHAIN_ID")?.parse::<u64>().expect("main: invalid conversion to u64");
    let dest_contract_address = env::var("DEST_CONTRACT_ADDR")?.parse::<Address>()?;
    let dest_provider = Provider::<Http>::try_from(env::var("DEST_RPC_URL")?.as_str())?;

    let src_chain_id: u64 = env::var("SRC_CHAIN_ID")?.parse::<u64>().expect("main: invalid conversion to u64");
    let src_provider = Provider::<Http>::try_from(env::var("SRC_RPC_URL")?.as_str())?;

    let args: Vec<String> = env::args().collect();
    let dest_wallet: Wallet<k256::ecdsa::SigningKey> = args[1].parse::<Wallet<k256::ecdsa::SigningKey>>()?.with_chain_id(dest_chain_id);
    let client_dest = SignerMiddleware::new(dest_provider.clone(), dest_wallet.clone());
    let src_wallet: Wallet<k256::ecdsa::SigningKey> = args[1].parse::<Wallet<k256::ecdsa::SigningKey>>()?.with_chain_id(src_chain_id);
    let client_src = SignerMiddleware::new(src_provider.clone(), src_wallet.clone());

    let source_rpc_url = env::var("SOURCE_RPC_SOCKET")?;
    let source_contract_string = env::var("SOURCE_CONTRACT_ADDR")?;
    let source_contract_address = source_contract_string.parse::<Address>()?;
    let transport = WebSocket::new(&source_rpc_url).await?;
    let web3 = web3::Web3::new(transport);

    let filter = FilterBuilder::default()
        .address(vec![source_contract_string.parse().unwrap()])
        .topics(
            Some(vec![hex!(
                "9853b992610853aa2f058d95d0a4868ffcd37470b93a5e6634619467e0d4228c"
            )
            .into()]),
            None,
            None,
            None,
        )
        .build();

    let mut stream = web3.eth_subscribe().subscribe_logs(filter).await?;

    let interval_dur = std::time::Duration::from_secs(batch_interval_time);
    let mut interval = tokio::time::interval(interval_dur);

    loop{
        tokio::select! {
            log = stream.next() => {
                if let Some(Ok(log)) = log {
                    //println!("Log: {:?}", log);
                    handle_log(&client_dest, &dest_contract_address, log, &mut revert_bytes, &mut log_bytes).await?;

                    if log_bytes.len() >= batch_size_limit {
                        let revert_bytes_clone = revert_bytes.clone();
                        let log_bytes_clone = log_bytes.clone();
                        revert_bytes.clear();
                        log_bytes.clear();

                        let interval_dur = std::time::Duration::from_secs(batch_interval_time);
                        interval = tokio::time::interval(interval_dur);

                        let client_src_clone = client_src.clone();
                        let client_dest_clone = client_dest.clone();
                        tokio::spawn(async move {
                            let _ = process_log( &client_src_clone, &client_dest_clone, &source_contract_address, &dest_contract_address, log_bytes_clone, revert_bytes_clone).await;
                        });
                    }
                }
            },
            _ = interval.tick() => {
                if !log_bytes.is_empty() {
                    let revert_bytes_clone = revert_bytes.clone();
                    let log_bytes_clone = log_bytes.clone();
                    revert_bytes.clear();
                    log_bytes.clear();

                    let client_src_clone = client_src.clone();
                    let client_dest_clone = client_dest.clone();
                    tokio::spawn(async move {
                        let _ = process_log( &client_src_clone, &client_dest_clone, &source_contract_address, &dest_contract_address, log_bytes_clone, revert_bytes_clone).await;
                    });
                }
                else {
                    println!("~ No Transactions Pending ~\n");
                }
            }
        }
    }
}

async fn handle_log(client_dest: &Client, contract_addr: &H160, log: Log, revert_bytes: &mut Vec<Bytes>, log_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>>{
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
            println!("handle_log: no match found for data_re\n");
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
        let block_timestamp = event_params[0].clone().into_uint().expect("handle_log: Invalid block.timestamp\n");

        match unix_timestamp {
            Some(y) => println!("* (id: {:?}) Source Contract -> Bridge Node (in seconds): {:?}\n", action_id_val[0], y - block_timestamp),
            None => println!("handle_log: invalid current timestamp\n"),
        }

        let from_vec = encode(&[event_params[1].clone()]);
        let from_bytes = Bytes::from(from_vec);
        println!("- TOPICS: {}, {}, {}\n", action_id, to, amount);

        //Call pure function to get transaction in bytes form
        get_transaction_bytes(client_dest, contract_addr, action_id.clone(), to.clone(), amount.clone(), log_bytes).await?;
        get_transaction_bytes(client_dest, contract_addr, action_id.clone(), from_bytes.clone(), amount.clone(), revert_bytes).await?;
    } else {
        println!("handle_log: no matches found\n");
    }

    Ok(())
}

async fn process_log(client_src: &Client, client_dest: &Client, source_contract_addr: &H160, dest_contract_addr: &H160, log_bytes: Vec<Bytes>, revert_bytes: Vec<Bytes>) -> Result< (), Box<dyn std::error::Error>> {
    println!("-----------------------BATCH CALL BEGIN-----------------------\n");

    //Call pure function to get final message in bytes form
    let msg = get_message_bytes( &client_dest, &dest_contract_addr, &log_bytes).await?;
    println!("- MSG BYTES: {:?}", msg);

    //Sign the message
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
    let timeout_duration = time::Duration::from_secs(args[4].parse().unwrap());
    let execute_message_future = execute_message( &client_dest, &dest_contract_addr, public_key_hex.as_str(), sig, msg, false);

    let success = match tokio::time::timeout(timeout_duration, execute_message_future).await {
        Ok(result) => result?,
        Err(_) => {
            println!("~ BATCH TIME LIMIT EXCEEDED ~\n");
            false
        }
    };

    if !success {
        process_reverts( &client_src, &client_dest, &source_contract_addr, &dest_contract_addr, &revert_bytes).await?;
    }
    
    println!("------------------------BATCH CALL END------------------------\n");
    Ok(())
}

async fn process_reverts(client_src: &Client, client_dest: &Client, source_contract_addr: &H160, dest_contract_addr: &H160, revert_bytes: &Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    println!("-------------------------REVERT BEGIN-------------------------\n");

    //Call pure function to get final message in bytes form
    let msg = get_message_bytes( client_dest, dest_contract_addr, revert_bytes).await?;

    //Sign the message
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
    execute_message( &client_src, source_contract_addr, public_key_hex.as_str(), sig, msg, true).await?;
    
    println!("--------------------------REVERT END--------------------------\n");
    Ok(())
}

async fn execute_message(client: &Client, contract_addr: &H160, pub_key_hex: &str, sig: Bytes, txn: Bytes, is_revert: bool) -> Result< bool, Box<dyn std::error::Error>> {
    let txn_send_timestamp = get_current_time();

    let pub_key_bytes = Vec::from_hex(pub_key_hex).expect("execute_message: Invalid hex string");
    let mut signer: [u8; 32] = [0; 32];
    signer.copy_from_slice(&pub_key_bytes);
    
    let execute;
    if is_revert {
        let contract = BridgeTx::new(contract_addr.clone(), Arc::new(client.clone()));
        execute = contract.execute_message(signer, sig, txn);
    }
    else {
        let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));
        execute = contract.execute_message(signer, sig, txn);
    }

    let result = execute.send().await;

    match result {
        Ok(tx_future) => {
            match tx_future.await {
                Ok(tx) => {
                    println!("\n- TRANSACTION RECEIPT: {}\n", serde_json::to_string(&tx)?);
                    let txn_complete_timestamp = get_current_time();
                    println!("- [Transaction completion timestamp] - [Node txn send timestamp] (in ms): {}\n", txn_complete_timestamp - txn_send_timestamp);
                    Ok(true)
                },
                Err(e) => {
                    println!("\n- TRANSACTION ERROR: {}\n", e);
                    Ok(false)
                }
            }
        },
        Err(e) => {
            let err_string = format!("{:?}", e);
            let re = Regex::new(r"Bytes\(0x.{8}([0-9a-fA-F]+)").unwrap();

            if let Some(capture) = re.captures(err_string.as_str()) {
                let hex_chars = capture.get(1).unwrap().as_str();
                let err_bytes = Vec::from_hex(hex_chars).unwrap();
                let err_byte_array: &[u8] = &err_bytes.as_slice();
                let err_val = decode(&[ParamType::String], err_byte_array)?;
                println!("\n- TRANSACTION FAILED | REVERT: {}\n", err_val[0]);
            } else {
                println!("\n- TRANSACTION FAILED | REVERT: {}\n", e);
            }

            Ok(false)
        }
    }
}

async fn get_transaction_bytes(client_dest: &Client, contract_addr: &H160, action_id: Bytes, to: Bytes, amount: Bytes, txn_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client_dest.clone()));
    let result = contract.get_transaction_bytes(action_id, to, amount).call().await?;

    txn_bytes.push(result);
    Ok(())
}

async fn get_message_bytes(client_dest: &Client, contract_addr: &H160, log_bytes: &Vec<Bytes>) -> Result<Bytes, Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client_dest.clone()));
    let result = contract.get_message_bytes(log_bytes.to_vec()).call().await?;
  
    Ok(result)
}

fn get_current_time() -> u64{
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let unix_timestamp_millis = duration_since_epoch.as_secs() * 1000 + u64::from(duration_since_epoch.subsec_millis());

    unix_timestamp_millis
}