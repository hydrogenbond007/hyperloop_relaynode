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
    async fn process_event_logs(logs: &[Log]) -> Result<(), Box<dyn Error>> {
        // saving the event logs to a file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open("event_logs.txt")
            .unwrap();
    
        for log in logs {
            if let Err(e) = writeln!(file, "{}", log.to_string()) {
                eprintln!("Could not write log to file: {}", e);
            }
        }
    
        Ok(())
    }

    let hash_file = Path::new("event_logs.txt");
    let hash_event = try_digest(hash_file).unwrap(); 
    let schnorr = Schnorr::new(0);
    let scalar_value = Scalar::random(&mut);
    let Keypair = schnorr.new_keypair(scalar_value);

    let batch_signature = schnorr.sign(&Keypair, hash_event);




    

    println!("Synced up to block: {}", head_block_num);
    println!("Balance of deposit contract: {}", balance);

    Ok(())}
}