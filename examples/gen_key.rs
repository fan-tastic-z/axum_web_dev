use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use rand::RngCore;

fn main() -> Result<()> {
	let mut key = [0u8; 64];
	rand::thread_rng().fill_bytes(&mut key);
	println!("\nGenrated key for HMAC:\n{key:?}");

	let b64u = general_purpose::URL_SAFE_NO_PAD.encode(key);
	println!("\nKey b64u encoded:\n{b64u}");

	Ok(())
}
