use helios::*;
use ethers::types::{FilterBuilder, H160, H256};
use web3::*;

fn main() {
let event_contractaddr:H160 = "Mainnet address".parse()?;
let event_signature: H256 = ethers::utils::keccak256("EventName(type1,type2)").into();

async fn listen_logs(&self) -> Result<(), Error> {
    let filter = FilterBuilder::default()
    .address(vec![event_contractaddr])
    .topics(Some(vec![event_signature]), None, None, None)
    .build();

    loop {
        let logs = self.get_logs(filter.clone()).await?;
        for logs in log {
            // parse the logged data with the use abi
            println!("new event emitted", log)
        }
        tokio::time::sleep(std::time::Duration::from_secs(12)).await;
    }

}

}


#[tokio::main]
async fn helios_rpc() -> Result<()> {
    let untrusted_rpc_url = env::var("UNTRUSTED_RPC_URL")?;

    let mut client = ClientBuilder::new()
        .network(Network::MAINNET)
        .consensus_rpc("https://www.lightclientdata.org")
        .execution_rpc(&untrusted_rpc_url)
        .build()?;

    client.start().await?;

    let head_block_num = client.get_block_number().await?;
    let addr = Address::from_str("0x00000000219ab540356cBB839Cbe05303d7705Fa")?;
    let block = BlockTag::Latest;
    let balance = client.get_balance(&addr, block).await?;

    println!("synced up to block: {}", head_block_num);
    println!("balance of deposit contract: {}", utils::format_ether(balance));

    Ok(())
}
