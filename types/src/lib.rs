use enc::{AesData, RsaData};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey, rand_core, traits::PaddingScheme};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use soft_aes::aes::AES_BLOCK_SIZE;
use std::{error::Error, fmt::Write, marker::PhantomData};
pub mod enc;

#[derive(Serialize, Deserialize, Debug)]
pub enum CPacket {
    Handshake { client_key: rsa::RsaPublicKey },
    Account(CAccount),
}
#[derive(Serialize, Deserialize, Debug)]
pub enum CAccount {
    Login {
        token: RsaData<u128>,
        creds: AesData<Credentials>,
    },
    Create {
        token: RsaData<u128>,
        creds: AesData<Credentials>,
    },
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Credentials {
    pub username: String,
    pub pw_digest: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum SPacket {
    Handshake {
        server_key: rsa::RsaPublicKey,
        shared_key: RsaData<Vec<u8>>,
        token: RsaData<u128>,
    },
    LoginResult(bool),
    InvalidToken,
}
