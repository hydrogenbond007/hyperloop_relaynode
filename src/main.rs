use config::File;
use ethers::core::k256::{ecdsa::signature::Keypair, elliptic_curve::scalar};
use schnorr_fun::Schnorr;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let untrusted_rpc_url = env::var("UNTRUSTED_RPC_URL")?;

    
    // need to fix the helios imports here
use ethers::prelude::*;
use tokio::time::{interval,Duration};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;
use std::path::Path;
use std::error::Error;
use sha256::{digest, try_digest};
use ethers::utils::keccak256;
use helios::{ClientBuilder,Network};
use schnorr_fun::{
    fun::{marker::*, Scalar, nonce},
    Schnorr,
    Message
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let untrusted_rpc_url = env::var("UNTRUSTED_RPC_URL")?;

    // Assuming `ClientBuilder` is part of the used Ethereum library and properly set up
    let client = ClientBuilder::new().rpc(&untrusted_rpc_url).build()?;

    let head_block_num = client.get_block_number().await?;
    let addr = Address::from_str("0x00000000219ab540356cBB839Cbe05303d7705Fa")?;
    let balance = client.get_balance(addr, None).await?;

    // Example for setting up a filter and listening to logs
    let event_signature = H256::from_slice(&keccak256("EventName(type1,type2)"));
    let filter = Filter::default()
        .address(vec![addr])
        .topics(Some(vec![event_signature]), None, None, None)
        .build();



    // maybe could work on this to  modif a bit more
    
    let mut event_logs: Vec<Log> = Vec::new();
    let mut interval_timer = interval(Duration::from_secs(60));

    loop {
        tokio::select! {
            // listens to the stream of logs
            logs = client.get_logs(&filter) => {
                match logs {
                    Ok(logs) => {
                        for log in logs {
                            event_logs.push(log);
                        }
                    },
                    Err(e) => println!("Error fetching logs: {}", e),
                }
            },
            _ = interval_timer.tick() => {
                // Process buffered logs
                process_event_logs(&event_logs);
                // clears the buffer for the next one
                event_logs.clear();
            },
        }
    }
    async fn process_event_logs(logs: &[Log]) -> Result<String, Box<dyn Error>> {
        let mut event_logs_string = String::new();
    
        for log in logs {
            event_logs_string.push_str(&log.to_string());
            event_logs_string.push('\n'); // Add a newline after each log
        }
    
        Ok(event_logs_string);
    let hash_event = try_digest(event_logs_string);
    let schnorr = Schnorr::new(0);
    let scalar_value = Scalar::random(&mut);
    let Keypair = schnorr.new_keypair(scalar_value);

    let batch_signature = schnorr.sign(&Keypair, hash_event);



    // createa a txn to send data to the contract
    let private_key = "";
    let destination_address = "";
    let tx = contract.method::<_, H256>("receiveData", (hash_event, batch_signature))?.send().await?;

    async fn send_data_to_contract(
        client: Arc<SignerMiddleware<Provider<Http>, Wallet>>,
        contract_address: &str,
        abi_path: &str,
        data: Vec<u8>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let contract = Contract::from_json(client, contract_address.parse()?, include_bytes!(abi_path))?;
        let tx = contract.method::<_, H256>("receiveData", data)?.send().await?;
        println!("Transaction sent: {:?}", tx);
        Ok(())
    }

    }

    

    println!("Synced up to block: {}", head_block_num);
    println!("Balance of deposit contract: {}", balance);

    Ok(())}
}