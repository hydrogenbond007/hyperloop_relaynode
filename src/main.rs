use dotenv::dotenv;
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
use ethabi::{decode, encode, ParamType, Token};

mod crypt;
mod txn;

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
        .build();

    let mut stream = web3.eth_subscribe().subscribe_logs(filter).await?;

    let interval_dur = std::time::Duration::from_secs(batch_interval_time);
    let mut interval = tokio::time::interval(interval_dur);

    loop{
        tokio::select! {
            log = stream.next() => {
                if let Some(Ok(log)) = log {
                    println!("Log: {:?}", log);
                    let result = handle_log(log, &mut log_bytes).await?;
                    match result {
                        Some(x) => {
                            let client_dest_clone = client_dest.clone();
                            tokio::spawn(async move {
                                let _ = process_batch(&client_dest_clone, &dest_contract_address, &x).await;
                            });
                        },
                        None => {}
                    }

                    if log_bytes.len() >= batch_size_limit {
                        let log_bytes_clone = log_bytes.clone();
                        log_bytes.clear();

                        let interval_dur = std::time::Duration::from_secs(batch_interval_time);
                        interval = tokio::time::interval(interval_dur);

                        let client_dest_clone = client_dest.clone();
                        tokio::spawn(async move {
                            let _ = process_log(&client_dest_clone, &dest_contract_address, log_bytes_clone).await;
                        });
                    }
                }
            },
            _ = interval.tick() => {
                if !log_bytes.is_empty() {
                    let log_bytes_clone = log_bytes.clone();
                    log_bytes.clear();

                    let client_dest_clone = client_dest.clone();
                    tokio::spawn(async move {
                        let _ = process_log(&client_dest_clone, &dest_contract_address, log_bytes_clone).await;
                    });
                }
                else {
                    println!("~ No Transactions Pending ~\n");
                }
            }
        }
    }
}

async fn handle_log(log: Log, log_bytes: &mut Vec<Bytes>) -> Result< Option<Bytes>, Box<dyn std::error::Error>>{
    let log_string = format!("{:?}", log);
    let log_re = Regex::new(r"topics: \[([^,]+), ([^,]+), ([^,]+), ([^,]+)]").unwrap();
    let msg_re = Regex::new(r#"data: Bytes\("0x([^"]+?)"\)"#).unwrap();
    let mut msg_str: &str = "";

    if let Some(captures) = msg_re.captures(log_string.as_str()) {
        msg_str = captures.get(1).unwrap().as_str();
    } else {
        println!("handle_log: no match found for data_re\n");
    }

    let msg_byte_vec = Vec::from_hex(&msg_str).unwrap();
    let msg_byte_arr = msg_byte_vec.as_slice();
    let msg_decode_params = [ParamType::Array(Box::new(ParamType::Bytes))];
    let msg = decode(&msg_decode_params, &msg_byte_arr)?;
    let txn_array = msg[0].clone().into_array().expect("handle_log: invalid array format");

    if let Some(captures) = log_re.captures(log_string.as_str()){
        let topic1 = captures.get(2).unwrap().as_str().trim();
        let topic2 = captures.get(3).unwrap().as_str().trim();
        let topic3 = captures.get(4).unwrap().as_str().trim();

        let action_id_vec = Vec::from_hex(&topic1[2..]).unwrap();
        let from_vec = Vec::from_hex(&topic2[2..]).unwrap();
        let timestamp_vec = Vec::from_hex(&topic3[2..]).unwrap();

        let action_id = decode(&[ParamType::Uint(256)], action_id_vec.as_slice())?[0].clone();
        let from = decode(&[ParamType::Address], from_vec.as_slice())?[0].clone();
        let timestamp = decode(&[ParamType::Uint(256)], timestamp_vec.as_slice())?[0].clone();

        let now = SystemTime::now();
        let unix_timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs().into_token().into_uint().expect("handle_log: invalid unix timestamp");
        let block_timestamp = timestamp.clone().into_uint().expect("handle_log: invalid block.timestamp\n");
        let time_delay = unix_timestamp - block_timestamp;

        if txn_array.len() == 1 {
            println!("* TXN action_id: {:?} | from: {:?} | source -> node duration (seconds): {:?}", action_id, from, time_delay);
        }
        else {
            println!("* BATCH action_id: {:?} | from: {:?} | source -> node duration (seconds): {:?}", action_id, from, time_delay);
        }
    }

    if txn_array.len() == 1 {
        let txn_vec = txn_array[0].clone().into_bytes().expect("handle_log: invalid bytes format");
        let txn_bytes = Bytes::from(txn_vec);

        log_bytes.push(txn_bytes);
        Ok(None)
    }
    else {
        let msg_bytes = Bytes::from(msg_byte_vec);
        Ok(Some(msg_bytes))
    }
}

async fn process_log(client_dest: &Client, dest_contract_addr: &H160, log_bytes: Vec<Bytes>) -> Result< (), Box<dyn std::error::Error>> {
    println!("-----------------------BATCH CALL BEGIN-----------------------\n");

    //Call pure function to get final message in bytes form
    let msg = txn::get_message_bytes( &client_dest, &dest_contract_addr, &log_bytes).await?;
    println!("- MSG BYTES: {:?}", msg);

    //Sign the message
    let args: Vec<String> = env::args().collect();
    let private_key_hex = &args[1];
    let public_key_hex = &args[2];
    println!("- SIGNER: {}", {public_key_hex});

    let sig = crypt::sign_message(private_key_hex.as_str(), &msg)?;
    println!("- SIG: {}", sig);

    //Send to destination contract
    let timeout: u64 = args[3].parse().unwrap();
    let ten_millis = time::Duration::from_millis(timeout);
    thread::sleep(ten_millis);

    txn::execute_message( &client_dest, &dest_contract_addr, public_key_hex.as_str(), sig, msg, false).await?;
    
    println!("------------------------BATCH CALL END------------------------\n");
    Ok(())
}

async fn process_reverts(client_src: &Client, client_dest: &Client, source_contract_addr: &H160, dest_contract_addr: &H160, revert_bytes: &Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    println!("-------------------------REVERT BEGIN-------------------------\n");

    //Call pure function to get final message in bytes form
    let msg = txn::get_message_bytes( client_dest, dest_contract_addr, revert_bytes).await?;

    //Sign the message
    let args: Vec<String> = env::args().collect();
    let private_key_hex = &args[1];
    let public_key_hex = &args[2];
    println!("- SIGNER: {}", {public_key_hex});

    let sig = crypt::sign_message(private_key_hex.as_str(), &msg)?;
    println!("- SIG: {}", sig);

    //Send to destination contract
    let timeout: u64 = args[3].parse().unwrap();
    let ten_millis = time::Duration::from_millis(timeout);
    thread::sleep(ten_millis);

    //Send to destination contract
    txn::execute_message( &client_src, source_contract_addr, public_key_hex.as_str(), sig, msg, true).await?;
    
    println!("--------------------------REVERT END--------------------------\n");
    Ok(())
}

async fn process_batch(client_dest: &Client, dest_contract_addr: &H160, msg: &Bytes) -> Result< (), Box<dyn std::error::Error>> {
    println!("-----------------------BATCH CALL BEGIN-----------------------\n");
    println!("- BATCH MSG: {}", msg);

    //Sign the message
    let args: Vec<String> = env::args().collect();
    let private_key_hex = &args[1];
    let public_key_hex = &args[2];
    println!("- SIGNER: {}", {public_key_hex});

    let sig = crypt::sign_message(private_key_hex.as_str(), &msg)?;
    println!("- SIG: {}", sig);

    //Send to destination contract
    let timeout: u64 = args[3].parse().unwrap();
    let ten_millis = time::Duration::from_millis(timeout);
    thread::sleep(ten_millis);

    txn::execute_message( &client_dest, &dest_contract_addr, public_key_hex.as_str(), sig, msg.clone(), false).await?;
    
    println!("------------------------BATCH CALL END------------------------\n");
    Ok(())
}