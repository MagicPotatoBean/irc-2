use std::net::{TcpStream, ToSocketAddrs};

use anyhow::{anyhow, bail};
use net_message::asymmetric::AsymmetricTcpStream;
use rsa::{rand_core::OsRng, RsaPrivateKey, RsaPublicKey};
use thiserror::Error;
use types::{
    enc::{AesData, RsaData},
    CPacket, Credentials, InboundMessage, OutboundMessage, SAccount, SPacket,
};

pub struct Connection {
    stream: AsymmetricTcpStream<CPacket, SPacket>,
    username: Option<String>,
    token: u128,
    aes_key: Vec<u8>,
    server_key: RsaPublicKey,
    client_key: RsaPrivateKey,
}
impl Connection {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Option<Self> {
        let client = TcpStream::connect(addr).unwrap();
        client.set_nonblocking(false).unwrap();
        client.set_read_timeout(None).unwrap();
        let mut stream: AsymmetricTcpStream<CPacket, SPacket> =
            AsymmetricTcpStream::new_unchecked(client);
        let priv_key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        stream
            .send(CPacket::Handshake {
                client_key: priv_key.to_public_key(),
            })
            .unwrap();
        if let Ok(SPacket::Handshake {
            server_key,
            shared_key,
            token,
        }) = stream.read()
        {
            Some(Self {
                stream,
                username: None,
                token: token.get(&priv_key).unwrap(),
                aes_key: shared_key.get(&priv_key).unwrap(),
                server_key,
                client_key: priv_key,
            })
        } else {
            None
        }
    }
    pub fn login(&mut self, username: String, password: &str) -> Result<(), LoginError> {
        self.stream
            .send(CPacket::Account(types::CAccount::Login {
                token: RsaData::new(self.token, &self.server_key).unwrap(),
                creds: AesData::new(
                    Credentials {
                        username: username.clone(),
                        pw_digest: sha256::digest(password),
                    },
                    &self.aes_key,
                )
                .unwrap(),
            }))
            .unwrap();
        match self.stream.read() {
            Ok(SPacket::Account(SAccount::Success)) => {
                self.username = Some(username);
                Ok(())
            }
            Ok(SPacket::Account(SAccount::InvalidToken)) => {
                self.username = None;
                Err(LoginError::InvalidToken)
            }
            Ok(SPacket::Account(SAccount::IncorrectPassword)) => Err(LoginError::IncorrectPassword),
            Ok(_) => Err(LoginError::InvalidPacket),
            Err(_) => Err(LoginError::Disconnected),
        }
    }
    pub fn create_account(
        &mut self,
        username: String,
        password: &str,
    ) -> Result<(), CreateAccountError> {
        self.stream
            .send(CPacket::Account(types::CAccount::Create {
                token: RsaData::new(self.token, &self.server_key).unwrap(),
                creds: AesData::new(
                    Credentials {
                        username,
                        pw_digest: sha256::digest(password),
                    },
                    &self.aes_key,
                )
                .unwrap(),
            }))
            .unwrap();
        match self.stream.read() {
            Ok(SPacket::Account(SAccount::Success)) => Ok(()),
            Ok(SPacket::Account(SAccount::AccountExists)) => Err(CreateAccountError::AccountExists),
            Ok(SPacket::Account(SAccount::InvalidUsername)) => {
                Err(CreateAccountError::InvalidUsername)
            }
            Ok(SPacket::Account(SAccount::InvalidToken)) => {
                self.username = None;
                Err(CreateAccountError::InvalidToken)
            }
            Ok(_) => Err(CreateAccountError::InvalidPacket),
            Err(_) => Err(CreateAccountError::Disconnected),
        }
    }
    pub fn send_message(
        &mut self,
        recipients: Vec<String>,
        contents: String,
    ) -> Result<(), SendMessageError> {
        self.stream
            .send(CPacket::SendMessage(types::CSendMessage::Send {
                token: RsaData::new(self.token, &self.server_key).unwrap(),
                message: AesData::new(
                    OutboundMessage {
                        recipients,
                        contents,
                    },
                    &self.aes_key,
                )
                .unwrap(),
            }))
            .unwrap();
        match self.stream.read() {
            Ok(SPacket::SendMessage(types::SSendMessage::Success)) => Ok(()),
            Ok(SPacket::Account(SAccount::InvalidToken)) => {
                self.username = None;
                Err(SendMessageError::InvalidToken)
            }
            Ok(_) => Err(SendMessageError::InvalidPacket),
            Err(_) => Err(SendMessageError::Disconnected),
        }
    }
    pub fn recv_message(&mut self) -> Result<InboundMessage, RecvMessageError> {
        self.stream
            .send(CPacket::RecvMessage(types::CRecvMessage::FetchNext {
                token: RsaData::new(self.token, &self.server_key).unwrap(),
            }))
            .unwrap();
        match self.stream.read() {
            Ok(SPacket::RecvMessage(types::SRecvMessage::NextMsg { message })) => {
                match message.get(&self.aes_key) {
                    Ok(msg) => Ok(msg),
                    Err(_) => Err(RecvMessageError::DeserializationError),
                }
            }
            Ok(SPacket::Account(SAccount::InvalidToken)) => {
                self.username = None;
                Err(RecvMessageError::InvalidToken)
            }
            Ok(_) => Err(RecvMessageError::InvalidPacket),
            Err(_) => Err(RecvMessageError::Disconnected),
        }
    }
}
#[derive(Debug, Clone, Error)]
pub enum CreateAccountError {
    #[error("Account already exists")]
    AccountExists,
    #[error("Username contains invalid characters")]
    InvalidUsername,
    #[error("Invalid session token")]
    InvalidToken,
    #[error("Disconnected from server")]
    Disconnected,
    #[error("Server sent an invalid packet")]
    InvalidPacket,
}
#[derive(Debug, Clone, Error)]
pub enum LoginError {
    #[error("Incorrect password provided, or incorrect username")]
    IncorrectPassword,
    #[error("Invalid session token")]
    InvalidToken,
    #[error("Disconnected from server")]
    Disconnected,
    #[error("Server sent an invalid packet")]
    InvalidPacket,
}
#[derive(Debug, Clone, Error)]
pub enum SendMessageError {
    #[error("Invalid session token")]
    InvalidToken,
    #[error("Disconnected from server")]
    Disconnected,
    #[error("Server sent an invalid packet")]
    InvalidPacket,
}
#[derive(Debug, Clone, Error)]
pub enum RecvMessageError {
    #[error("Failed to parse AES data")]
    DeserializationError,
    #[error("Invalid session token")]
    InvalidToken,
    #[error("Disconnected from server")]
    Disconnected,
    #[error("Server sent an invalid packet")]
    InvalidPacket,
}
