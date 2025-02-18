use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey, rand_core, traits::PaddingScheme};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use soft_aes::aes::AES_BLOCK_SIZE;
use std::{error::Error, fmt::Write, marker::PhantomData};

#[derive(Serialize, Deserialize, Debug)]
pub struct RsaData<T: Serialize + DeserializeOwned> {
    data: Vec<u8>,
    pd: PhantomData<T>,
}
impl<T: Serialize + DeserializeOwned> RsaData<T> {
    pub fn new(data: T, key: &RsaPublicKey) -> Result<Self, rsa::Error> {
        let encrypted = key.encrypt(
            &mut rand_core::OsRng,
            Pkcs1v15Encrypt,
            &bincode::serialize(&data).unwrap(),
        );
        Ok(Self {
            data: encrypted?,
            pd: PhantomData,
        })
    }
    pub fn get(&self, key: &RsaPrivateKey) -> Result<T, rsa::Error> {
        Ok(bincode::deserialize(&key.decrypt(Pkcs1v15Encrypt, &self.data)?).unwrap())
    }
    pub fn set(&mut self, key: &RsaPublicKey, data: T) -> Result<(), rsa::Error> {
        *self = Self::new(data, key)?;
        Ok(())
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct AesData<T: Serialize + DeserializeOwned> {
    data: Vec<u8>,
    pd: PhantomData<T>,
}
impl<T: Serialize + DeserializeOwned> AesData<T> {
    pub fn new(data: T, key: &[u8]) -> Result<Self, Box<dyn Error>> {
        let data = &bincode::serialize(&data).unwrap();
        let encrypted = soft_aes::aes::aes_enc_cbc(data, key, &[0; AES_BLOCK_SIZE], Some("PKCS7"));
        Ok(Self {
            data: encrypted.unwrap(),
            pd: PhantomData,
        })
    }
    pub fn get(&self, key: &[u8]) -> Result<T, Box<dyn Error>> {
        Ok(bincode::deserialize(
            &soft_aes::aes::aes_dec_cbc(&self.data, key, &[0; AES_BLOCK_SIZE], Some("PKCS7"))
                .unwrap(),
        )
        .unwrap())
    }
    pub fn set(&mut self, key: &[u8], data: T) -> Result<(), Box<dyn Error>> {
        *self = Self::new(data, key)?;
        Ok(())
    }
}
