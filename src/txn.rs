use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use hex::FromHex;
use regex::Regex;
use ethers::prelude::*;
use ethabi::{decode, ParamType};

type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

abigen!(
    BridgeTx,
    "./abi/bridge_tx.json"
);

abigen!(
    BridgeRx,
    "./abi/bridge_rx.json"
);

pub async fn execute_message(client: &Client, contract_addr: &H160, pub_key_hex: &str, sig: Bytes, txn: Bytes, is_revert: bool) -> Result< bool, Box<dyn std::error::Error>> {
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

pub async fn get_transaction_bytes(client_dest: &Client, contract_addr: &H160, action_id: Bytes, to: Bytes, amount: Bytes, txn_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client_dest.clone()));
    let result = contract.get_transaction_bytes(action_id, to, amount).call().await?;

    txn_bytes.push(result);
    Ok(())
}

pub async fn get_message_bytes(client_dest: &Client, contract_addr: &H160, log_bytes: &Vec<Bytes>) -> Result<Bytes, Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client_dest.clone()));
    let result = contract.get_message_bytes(log_bytes.to_vec()).call().await?;
  
    Ok(result)
}

pub fn get_current_time() -> u64{
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let unix_timestamp_millis = duration_since_epoch.as_secs() * 1000 + u64::from(duration_since_epoch.subsec_millis());

    unix_timestamp_millis
}