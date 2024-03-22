#[allow(unused_imports)]
use std::{str::FromStr, env};

use helios::client::ClientBuilder;
use helios::config::networks::Network;
use helios::types::BlockTag;
use helios::client::*;
use helios::rpc::*;
use ethers::types::{Address, Filter};
use ethers::utils::*;
#[allow(unused_imports)]
use ethers::{types::Address, utils, FilterBuilder, H160, H256};
#[allow(unused_imports)]
use eyre::Result;
#[allow(unused_imports)]
use web3::types::{FilterBuilder, Log};
#[allow(unused_imports)]
use web3::Web3;
#[allow(unused_imports)]
use web3::transports::Http;

async fn main() -> Result<()> {
    let untrusted_rpc_url = env::var("private_rpc")?;

    let mut client = ClientBuilder::new()
        .network(Network::MAINNET)
        .consensus_rpc("https://www.lightclientdata.org")
        .execution_rpc(&untrusted_rpc_url)
        .build()?;

    client.start().await?;

    let head_block_num = client.get_block_number().await?;
    let block = BlockTag::Latest;
    let balance = client.get_balance(&addr, block).await?;
    let contract_address: H160 = "your_contract_address".parse()?;
    let event_signature: H256 = ethers::utils::keccak256("EventName(type1,type2)").into();

    println!("synced up to block: {}", head_block_num);

    Ok(())
}



async fn event_emitted(rpc: &rpc, filter:Filter) -> Result<()> {
    let logs = rpc.get_logs(filter).await?;
    let filter = FilterBuilder::default()
    .address(vec![contract_address])
    .topics(Some(vec![event_signature]), None, None, None)
    .build();
    for log in logs {
        println!("Log found: {:?}", log);
        
    }
    Ok(())
}

