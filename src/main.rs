use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use regex::Regex;
use hex::FromHex;
use web3::futures::StreamExt;
use web3::types::{FilterBuilder, Log};
use web3::transports::WebSocket;
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
mod ed25519;

type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

abigen!(
    BridgeRx,
    "./abi/bridge_rx.json"
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let chain_id: u64 = env::var("DEST_CHAIN_ID")?.parse::<u64>().expect("Invalid conversion to u64");
    let dest_contract_address = env::var("DEST_CONTRACT_ADDR")?.parse::<Address>()?;

    let provider = Provider::<Http>::try_from(env::var("DEST_RPC_URL")?.as_str())?;
    let wallet: Wallet<k256::ecdsa::SigningKey> = env::var("PRIVATE_KEY")?.parse::<Wallet<k256::ecdsa::SigningKey>>()?.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider.clone(), wallet.clone());

    let mut log_bytes: Vec<Bytes> = vec![];

    let source_rpc_url = env::var("SOURCE_RPC_URL")?;
    let source_contract_address = env::var("SOURCE_CONTRACT_ADDR")?;
    let transport = WebSocket::new(&source_rpc_url).await?;
    let web3 = web3::Web3::new(transport);

    let filter = FilterBuilder::default()
        .address(vec![source_contract_address.parse().unwrap()])
        .build();

    let mut stream = web3.eth_subscribe().subscribe_logs(filter).await?;

    while let Some(Ok(log)) = stream.next().await {
        handle_log(&client, &dest_contract_address, &mut log_bytes, log).await?;
    }

    Ok(())
}

async fn handle_log(client: &Client, contract_addr: &H160, log_bytes: &mut Vec<Bytes>, log: Log) -> Result<(), Box<dyn std::error::Error>>{
    let log_string = format!("{:?}", log);
    let re = Regex::new(r"topics: \[([^,]+), ([^,]+), ([^,]+), ([^,]+)]").unwrap();

    if let Some(captures) = re.captures(log_string.as_str()) {
        let topic1 = captures.get(2).unwrap().as_str().trim();
        let topic2 = captures.get(3).unwrap().as_str().trim();
        let topic3 = captures.get(4).unwrap().as_str().trim();

        let action_id = topic1.parse::<Bytes>()?;
        let to = topic2.parse::<Bytes>()?;
        let amount = topic3.parse::<Bytes>()?;

        // ^^ Call pure function to get transaction in bytes form
        get_transaction_bytes(client, contract_addr, action_id, to, amount, log_bytes).await?;
        process_log(client, contract_addr, log_bytes).await?;
    } else {
        println!("No matches found");
    }
    //println!("Received event: {:?}", log);

    Ok(())
}

async fn process_log(client: &Client, contract_addr: &H160, log_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    // ^^ Call pure function to get final message in bytes form
    let msg = get_message_bytes(client, contract_addr, log_bytes).await?;

    // ^^ Sign the message
    let private_key_hex = env::var("PRIVATE_KEY")?;
    let public_key_hex = env::var("PUBLIC_KEY")?;
    let sig = ed25519::sign_message(private_key_hex.as_str(), &msg)?;
    println!("SIG: {}", sig);

    // ^^ send to bridge node
    execute_message(client, contract_addr, public_key_hex.as_str(), sig, msg).await?;

    Ok(())
}

async fn get_transaction_bytes(client: &Client, contract_addr: &H160, action_id: Bytes, to: Bytes, amount: Bytes, log_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    println!("TOPICS: {}, {}, {}", action_id, to, amount);
    
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));
    let result = contract.get_transaction_bytes(action_id, to, amount).call().await?;

    log_bytes.push(result);

    Ok(())
}

async fn get_message_bytes(client: &Client, contract_addr: &H160, log_bytes: &mut Vec<Bytes>) -> Result<Bytes, Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));
    let result = contract.get_message_bytes(log_bytes.to_vec()).call().await?;

    println!("MSG BYTES: {:?}", result);
    log_bytes.clear();

    Ok(result)
}

async fn execute_message(client: &Client, contract_addr: &H160, pub_key_hex: &str, sig: Bytes, txn: Bytes) -> Result<(), Box<dyn std::error::Error>> {
    let pub_key_bytes = Vec::from_hex(pub_key_hex).expect("Invalid hex string");
    let mut signer: [u8; 32] = [0; 32];
    signer.copy_from_slice(&pub_key_bytes);

    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));

    let execute = contract.execute_message(signer, sig, txn);
    let result = execute.send().await;

    match result {
        Ok(tx_future) => {
            match tx_future.await {
                Ok(tx) => {
                    println!("\nTRANSACTION RECEIPT: {}\n", serde_json::to_string(&tx)?);
                    println!("-------------------------------------------------------\n");
                },
                Err(e) => {
                    println!("TRANSACTION ERROR: {}", e);
                }
            }
        },
        Err(e) => {
            println!("\nTRANSACTION FAILED | REVERT: {}\n", e);
        }
    }

    Ok(())
}
