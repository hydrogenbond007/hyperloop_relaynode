#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let untrusted_rpc_url = env::var("UNTRUSTED_RPC_URL")?;

    // Assuming `ClientBuilder` is part of the used Ethereum library and properly set up
    // Assume necessary imports are here
use ethers::prelude::*;
use std::env;
use std::str::FromStr;
use std::error::Error;
use ethers::utils::keccak256;
use helios::{ClientBuilder,Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let untrusted_rpc_url = env::var("private rpc endpoint")?;
    let client = ClientBuilder::new().rpc(&untrusted_rpc_url).build()?;

    let head_block_num = client.get_block_number().await?;
    let addr = Address::from_str("your address")?;
    let balance = client.get_balance(addr, None).await?;

    let event_signature = H256::from_slice(&keccak256("EventName(type1,type2)"));
    let filter = Filter::default()
        .address(vec![addr])
        .topics(Some(vec![event_signature]), None, None, None)
        .build();

    // fetching logs with ethers and helios
    let mut client = ClientBuilder::new()
        .network(Network::MAINNET)
        .consensus_rpc("https://www.lightclientdata.org")
        .execution_rpc(&untrusted_rpc_url)
        .build()?;
    let logs = client.get_logs(&filter).await?;
    for log in logs {
        println!("New event emitted: {:?}", log);
    }

    println!("Synced up to block: {}", head_block_num);
    println!("Balance of deposit contract: {}", balance);

    Ok(())}
}
