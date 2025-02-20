use enc::{AesData, RsaData};
use serde::{Deserialize, Serialize};
pub mod enc;

#[derive(Serialize, Deserialize, Debug)]
pub enum CPacket {
    Handshake { client_key: rsa::RsaPublicKey },
    Account(CAccount),
    SendMessage(CSendMessage),
    RecvMessage(CRecvMessage),
}
#[derive(Serialize, Deserialize, Debug)]
pub enum CSendMessage {
    Send {
        token: RsaData<u128>,
        message: AesData<OutboundMessage>,
    },
}
#[derive(Serialize, Deserialize, Debug)]
pub enum CRecvMessage {
    FetchNext { token: RsaData<u128> },
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
    Logout {
        token: RsaData<u128>,
    },
}
#[derive(Serialize, Deserialize, Debug)]
pub enum SPacket {
    Handshake {
        server_key: rsa::RsaPublicKey,
        shared_key: RsaData<Vec<u8>>,
        token: RsaData<u128>,
    },
    Account(SAccount),
    SendMessage(SSendMessage),
    RecvMessage(SRecvMessage),
}
#[derive(Serialize, Deserialize, Debug)]
pub enum SAccount {
    Success,
    AccountExists,
    IncorrectPassword,
    InvalidUsername,
    InvalidToken,
    NotLoggedIn,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum SRecvMessage {
    NextMsg { message: AesData<InboundMessage> },
}
#[derive(Serialize, Deserialize, Debug)]
pub enum SSendMessage {
    Success,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OutboundMessage {
    pub recipients: Vec<String>,
    pub contents: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct InboundMessage {
    pub sender: String,
    pub recipients: Vec<String>,
    pub contents: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Credentials {
    pub username: String,
    pub pw_digest: String,
}
