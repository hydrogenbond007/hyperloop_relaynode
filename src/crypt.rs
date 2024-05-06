use ethers::prelude::*;
use hex::{FromHex, encode};
use ed25519_dalek::{SigningKey, Signer};

pub fn sign_message(private_key_hex: &str, message_bytes: &Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let private_key_bytes = Vec::from_hex(private_key_hex).expect("Invalid private key hex string");
    let private_key_array: [u8; 32] = private_key_bytes.as_slice().try_into().unwrap();

    let keypair = SigningKey::from_bytes(&private_key_array);
    let signature = keypair.sign(&message_bytes);

    let hex_sig = format!("0x{}", encode(&signature.to_bytes()));
    let sig: Bytes = hex_sig.parse::<Bytes>()?;

    Ok(sig)
}