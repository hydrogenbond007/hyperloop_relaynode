#[allow(unused_imports)]
use std::{str::FromStr, env};

use helios::{client::ClientBuilder, config::networks::Network, types::BlockTag};
use ethers::{types::Address, utils};
#[allow(unused_imports)]
use eyre::Result;
#[allow(unused_imports)]
use web3::types::{FilterBuilder, Log};
#[allow(unused_imports)]
use web3::Web3;
#[allow(unused_imports)]
use web3::transports::Http;

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let untrusted_rpc_url = env::var("normal_rpc")?;

            let mut client = ClientBuilder::new()
                .network(Network::MAINNET)
                .consensus_rpc("https://www.lightclientdata.org")
                .execution_rpc(&untrusted_rpc_url)
                .build()?;

            client.start().await?;

            let head_block_num = client.get_block_number().await?;
            let addr = Address::from_str("Eth_host_address")?;
            let block = BlockTag::Latest;
            let balance = client.get_balance(&addr, block).await?;

            let contract_address = env::var("contract_address")?;


            let filter = FilterBuilder::default()
            .address(vec![contract_address.parse().unwrap()])
            // You can specify topics (event signatures) here if you want to filter specific events
            .build();
    
        // Subscribe to the filter
        let sub = web3.eth_subscribe().subscribe_logs(filter).await?;
        let event = web3.eth_get_logs(filter).await?;
    
        println!("Listening for events...");

        fn handle_log(log: Log) {
            println!("Event: {:?}", log);
        }
    
        sub.for_each(|log| {
            match log {
                Ok(log) => handle_log(log),
                Err(e) => println!("Error: {}", e),
            }
    
            futures::future::ready(())
        }).await;
    
        Ok(());

            println!("synced up to block: {}", head_block_num);
            println!("balance of deposit contract: {}", utils::format_ether(balance));

            Ok(())
        });
}
