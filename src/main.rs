use tokio::time::{interval,Duration};
use config::File;
use std::str::FromStr;
use std::error::Error;
use std::sync::Arc;
use eyre::Result;
use std::path::Path;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

use helios::prelude::*;
use helios::{client::ClientBuilder, config::networks::Network, client::Client};

use ethers::prelude::*;
use ethers::prelude::{Address, U256};
use ethers::types::{Filter, Log, H256, SyncingStatus, Transaction, TransactionReceipt};
use ethers::providers::{Provider, Http};
use ethers::utils::keccak256;

//use ethers::core::k256::{ecdsa::signature::Keypair, elliptic_curve::scalar};
use sha256::{digest, try_digest};
use schnorr_fun::Schnorr;
use schnorr_fun::{
    fun::{marker::*, Scalar, nonce},
    Message
};

abigen!(
    BridgeRx,
    "./abi/bridge_rx.json"
);

type EthClient = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

async fn execute_message(client: &EthClient, contract_addr: &H160, signer: [u8; 32], sig: Bytes, txn: Bytes) -> Result<(), Box<dyn std::error::Error>> {
    let contract = BridgeRx::new(contract_addr.clone(), Arc::new(client.clone()));

    let execute = contract.execute_message(signer, sig, txn);
    let result = execute.send().await;

    match result {
        Ok(tx_future) => {
            match tx_future.await {
                Ok(tx) => {
                    println!("\nTRANSACTION RECEIPT: {}\n", serde_json::to_string(&tx)?);
                },
                Err(e) => {
                    println!("TRANSACTION ERROR: {}", e);
                }
            }
        },
        Err(e) => {
            println!("\nSIGNATURE INVALID | REVERT: {}\n", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // --- TRANSACTION RELATED CODE ---

    let chain_id: u64 = 31337;
    let rpc_url = format!("http://127.0.0.1:8545");

    let private_key = "47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a";
    let contract_address = "0x9CB7a24C844afd220f05EB1359691E6A4DB80e3a".parse::<Address>()?;

    let pub_key = "292C70EBBBD20F278DB008B93A76D39AD5D87299883E59BC2CD5900F2EB849C2";
    let sig: Bytes = "0xCA69906813B443FAC1D047DA4E73472DA34B873D2B8C6698C185859F3DDB860C8C5E871FDA0510C9B37ADA883B85ED09B42A141817C4652B3AB4A67D2D8C1708".parse::<Bytes>()?;
    let txn: Bytes = "0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000002b5ad5c4795c026514f8317c7a215e218dccd6cf0000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000002b5ad5c4795c026514f8317c7a215e218dccd6cf00000000000000000000000000000000000000000000000000000000000f4240".parse::<Bytes>()?;

    let provider = Provider::<Http>::try_from(rpc_url.as_str())?;
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider.clone(), wallet.clone());

    // --------------------------------

    //let untrusted_rpc_url = env::var("UNTRUSTED_RPC_URL")?;
    let untrusted_rpc_url = "http://128.0.0.16";
    let addr = Address::from_str("0x00000000219ab540356cBB839Cbe05303d7705Fa")?;

    // Assuming `ClientBuilder` is part of the used Ethereum library and properly set up
    // ^^ client variable type was unknown so specified to Client<ConfigDB> (not sure if that is right) + need to set the arguments properly
    let mut client: Client<ConfigDB>= ClientBuilder::new()
        .network(Network::GOERLI)
        .consensus_rpc("https://www.lightclientdata.org")
        .execution_rpc(&untrusted_rpc_url)
        .build()?;

    let head_block_num = client.get_block_number().await?;
    //let balance = client.get_balance(&addr, None).await?;

    // Example for setting up a filter and listening to logs
    let event_signature = H256::from_slice(&keccak256("EventName(type1,type2)"));
    let filter = Filter::new()
        .address(addr)
        .event("EventName(type1,type2)");

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
        let mut event_logs_string = String::new();
    
        for log in logs {
            // ^^ log does not have direct string conversion so need to do manually for each field
            event_logs_string.push_str(&log.address.to_string()); 
            event_logs_string.push('\n'); // Add a newline after each log
        }
    
        //Ok(event_logs_string)
/* 
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

        }*/
    

        //println!("Synced up to block: {}", head_block_num);
        //println!("Balance of deposit contract: {}", balance);

        Ok(())
    }
}