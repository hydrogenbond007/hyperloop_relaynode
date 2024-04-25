use web3::futures::StreamExt;
use web3::types::{FilterBuilder, Log};
use web3::transports::WebSocket;
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use std::sync::Arc;
use hex::{FromHex, encode};
use regex::Regex;
mod ed25519;

type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

abigen!(
    BridgeRx,
    "./abi/bridge_rx.json"
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let private_key = "47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a";

    let chain_id: u64 = 31337;
    let dest_rpc_url = format!("http://127.0.0.1:8545");
    let dest_contract_address = "0x98eDDadCfde04dC22a0e62119617e74a6Bc77313".parse::<Address>()?;

    let provider = Provider::<Http>::try_from(dest_rpc_url.as_str())?;
    let wallet: Wallet<k256::ecdsa::SigningKey> = private_key.parse::<Wallet<k256::ecdsa::SigningKey>>()?.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider.clone(), wallet.clone());

    // --------------------------------------------------------------------------------------------

    let private_key_hex = "B583220215144D856030DCA19F5F800B6908F1F3E0D9168E1D3FFA73017FB2CA";
    let public_key_hex = "292C70EBBBD20F278DB008B93A76D39AD5D87299883E59BC2CD5900F2EB849C2";

    let private_key_bytes = Vec::from_hex(private_key_hex).expect("Invalid private key hex string");
    let private_key_array: [u8; 32] = private_key_bytes.as_slice().try_into().unwrap();

    // --------------------------------------------------------------------------------------------

    let mut log_bytes: Vec<Bytes> = vec![];

    let source_rpc_url = "ws://127.0.0.1:8545";
    let source_contract_address = "0x95bD8D42f30351685e96C62EDdc0d0613bf9a87A";
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
    let private_key_hex = "B583220215144D856030DCA19F5F800B6908F1F3E0D9168E1D3FFA73017FB2CA";
    let sig = ed25519::sign_message(private_key_hex, &msg)?;
    println!("Sig: {}", sig);

    // ^^ send to bridge node

    Ok(())
}

async fn get_transaction_bytes(client: &Client, contract_addr: &H160, action_id: Bytes, to: Bytes, amount: Bytes, log_bytes: &mut Vec<Bytes>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Topics: {}, {}, {}", action_id, to, amount);
    
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));
    let result = contract.get_transaction_bytes(action_id, to, amount).call().await?;

    log_bytes.push(result);

    Ok(())
}

async fn get_message_bytes(client: &Client, contract_addr: &H160, log_bytes: &mut Vec<Bytes>) -> Result<Bytes, Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));
    let result = contract.get_message_bytes(log_bytes.to_vec()).call().await?;

    println!("Msg Bytes: {:?}", result);

    Ok(result)
}
