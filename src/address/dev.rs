use super::{AddressBundle, AddressProvider};
use anyhow::Result;
use bech32::{ToBase32, Variant};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
use rand::rngs::OsRng;


#[derive(Clone, Default)]
pub struct DevAddressProvider;


impl DevAddressProvider { pub fn new() -> Self { Self } }


impl AddressProvider for DevAddressProvider {
fn new_address(&self) -> Result<AddressBundle> {
let mut csprng = OsRng;
let signing = SigningKey::generate(&mut csprng);
let verifying: VerifyingKey = (&signing).into();
let pk = verifying.to_bytes();
let sk = signing.to_bytes();
// DEV bech32 address: "addr_dev1" + pk hash (not a real Cardano address!)
let hrp = "addr_dev";
let data = pk.to_vec().to_base32();
let addr = bech32::encode(hrp, data, Variant::Bech32)?;
Ok(AddressBundle { address: addr, pubkey: pk, privkey: sk })
}


fn sign_utf8(&self, privkey: &[u8;32], message: &str) -> Result<Vec<u8>> {
let signing = SigningKey::from_bytes(privkey);
let sig = signing.sign(message.as_bytes());
Ok(sig.to_bytes().to_vec())
}
}