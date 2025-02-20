use net_message::asymmetric::AsymmetricTcpStream;
use rand::Rng;
use rsa::{RsaPrivateKey, RsaPublicKey, rand_core};
use std::{
    collections::{HashMap, VecDeque},
    net::TcpListener,
    sync::{LazyLock, RwLock},
    time::Duration,
};
use types::{
    CPacket, Credentials, InboundMessage, OutboundMessage, SAccount, SPacket, SSendMessage,
    enc::{AesData, RsaData},
};

static TOKEN_MAP: LazyLock<RwLock<HashMap<u128, TokenData>>> =
    LazyLock::new(|| HashMap::new().into());
static ACCOUNT_MAP: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| HashMap::new().into());
static MESSAGE_MAP: LazyLock<RwLock<HashMap<String, VecDeque<InboundMessage>>>> =
    LazyLock::new(|| HashMap::new().into());

#[derive(Clone, Debug)]
struct TokenData {
    username: Option<String>,
    rsa_key: RsaPublicKey,
    aes_key: Vec<u8>,
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:65432").unwrap();
    for client in listener.incoming().flatten() {
        std::thread::spawn(|| {
            client.set_nonblocking(false).unwrap();
            client.set_read_timeout(None).unwrap();
            let mut stream = AsymmetricTcpStream::<SPacket, CPacket>::new_unchecked(client);

            println!("Generating server RSA keys, please allow a few seconds for this to happen");
            let priv_key = RsaPrivateKey::new(&mut rand_core::OsRng, 2048).unwrap();
            println!("Generated keys");

            while let Ok(pack) = stream.read() {
                println!("{pack:?}");
                match pack {
                    CPacket::Handshake { client_key } => {
                        handshake(&mut stream, &client_key, &priv_key)
                    }
                    CPacket::Account(c_account) => match c_account {
                        types::CAccount::Login { token, creds } => {
                            login(&mut stream, &priv_key, token, creds)
                        }
                        types::CAccount::Create { token, creds } => {
                            create_account(&mut stream, &priv_key, token, creds)
                        }
                        types::CAccount::Logout { token } => logout(&mut stream, &priv_key, token),
                    },
                    CPacket::SendMessage(csend_message) => match csend_message {
                        types::CSendMessage::Send { token, message } => {
                            send_msg(&mut stream, &priv_key, token, message)
                        }
                    },
                    CPacket::RecvMessage(crecv_message) => match crecv_message {
                        types::CRecvMessage::FetchNext { token } => {
                            recv_msg(&mut stream, &priv_key, token)
                        }
                    },
                }
            }
        });
    }
}
fn handshake(
    stream: &mut AsymmetricTcpStream<SPacket, CPacket>,
    client_key: &RsaPublicKey,
    priv_key: &RsaPrivateKey,
) {
    println!("handshake");
    let aes_key: [u8; 16] = rand::rng().random(); // 1024 bit key
    let user = TokenData {
        username: None,
        rsa_key: client_key.clone(),
        aes_key: aes_key.to_vec(),
    };
    let read_lock = TOKEN_MAP.read().unwrap();
    let token = loop {
        let token: u128 = rand::rng().random();
        if !read_lock.contains_key(&token) {
            drop(read_lock);
            TOKEN_MAP.write().unwrap().insert(token, user);
            break token;
        }
    };
    stream
        .send(SPacket::Handshake {
            server_key: priv_key.into(),
            shared_key: RsaData::new(aes_key.to_vec(), client_key).unwrap(),
            token: RsaData::new(token, client_key).unwrap(),
        })
        .unwrap();
    println!("Sent off handshake");
}
fn login(
    stream: &mut AsymmetricTcpStream<SPacket, CPacket>,
    priv_key: &RsaPrivateKey,
    token: RsaData<u128>,
    creds: AesData<Credentials>,
) {
    let token = token.get(priv_key).unwrap();
    match TOKEN_MAP.write().unwrap().entry(token) {
        std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
            let creds = creds.get(&occupied_entry.get().aes_key).unwrap();

            if Some(&creds.pw_digest) == ACCOUNT_MAP.read().unwrap().get(&creds.username) {
                occupied_entry.get_mut().username = Some(creds.username);
                stream
                    .send(SPacket::Account(types::SAccount::Success))
                    .unwrap()
            } else {
                stream
                    .send(SPacket::Account(types::SAccount::IncorrectPassword))
                    .unwrap()
            }
        }
        std::collections::hash_map::Entry::Vacant(_) => {
            stream
                .send(SPacket::Account(types::SAccount::InvalidToken))
                .unwrap();
        }
    }
}
fn create_account(
    stream: &mut AsymmetricTcpStream<SPacket, CPacket>,
    priv_key: &RsaPrivateKey,
    token: RsaData<u128>,
    creds: AesData<Credentials>,
) {
    let token = token.get(priv_key).unwrap();
    match TOKEN_MAP.read().unwrap().get(&token) {
        Some(user) => {
            let creds = creds.get(&user.aes_key).unwrap();
            if creds.username.chars().any(|chr| !chr.is_alphanumeric()) {
                stream
                    .send(SPacket::Account(SAccount::InvalidUsername))
                    .unwrap();
                return;
            }

            if ACCOUNT_MAP.read().unwrap().contains_key(&creds.username) {
                println!("Account already exists");
                stream
                    .send(SPacket::Account(types::SAccount::AccountExists))
                    .unwrap();
            } else {
                ACCOUNT_MAP
                    .write()
                    .unwrap()
                    .insert(creds.username, creds.pw_digest);
                stream
                    .send(SPacket::Account(types::SAccount::Success))
                    .unwrap();
            }
        }
        None => {
            stream
                .send(SPacket::Account(SAccount::InvalidToken))
                .unwrap();
        }
    }
}
fn send_msg(
    stream: &mut AsymmetricTcpStream<SPacket, CPacket>,
    priv_key: &RsaPrivateKey,
    token: RsaData<u128>,
    message: AesData<OutboundMessage>,
) {
    let token = token.get(priv_key).unwrap();
    let Some(usr) = TOKEN_MAP.read().unwrap().get(&token).cloned() else {
        stream
            .send(SPacket::Account(SAccount::InvalidToken))
            .unwrap();
        return;
    };
    if let Some(username) = usr.username {
        let message = message.get(&usr.aes_key).unwrap();
        for recipient in &message.recipients {
            match MESSAGE_MAP.write().unwrap().entry(recipient.to_string()) {
                std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().push_back(InboundMessage {
                        sender: username.clone(),
                        recipients: message.recipients.clone(),
                        contents: message.contents.clone(),
                    });
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(
                        vec![InboundMessage {
                            sender: username.clone(),
                            recipients: message.recipients.clone(),
                            contents: message.contents.clone(),
                        }]
                        .into(),
                    );
                }
            }
        }
        stream
            .send(SPacket::SendMessage(SSendMessage::Success))
            .unwrap()
    }
}
fn recv_msg(
    stream: &mut AsymmetricTcpStream<SPacket, CPacket>,
    priv_key: &RsaPrivateKey,
    token: RsaData<u128>,
) {
    let token = token.get(priv_key).unwrap();
    let Some(usr) = TOKEN_MAP.read().unwrap().get(&token).cloned() else {
        stream
            .send(SPacket::Account(SAccount::InvalidToken))
            .unwrap();
        return;
    };
    if let Some(username) = usr.username {
        loop {
            if let Some(vec) = MESSAGE_MAP.read().unwrap().get(&username) {
                if !vec.is_empty() {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(250))
        }
        let next_msg = MESSAGE_MAP
            .write()
            .unwrap()
            .get_mut(&username)
            .unwrap()
            .pop_front();
        stream
            .send(SPacket::RecvMessage(types::SRecvMessage::NextMsg {
                message: AesData::new(next_msg.unwrap(), &usr.aes_key).unwrap(),
            }))
            .unwrap();
    }
}
fn logout(
    stream: &mut AsymmetricTcpStream<SPacket, CPacket>,
    priv_key: &RsaPrivateKey,
    token: RsaData<u128>,
) {
    let token = token.get(priv_key).unwrap();
    match TOKEN_MAP.write().unwrap().entry(token) {
        std::collections::hash_map::Entry::Occupied(occupied_entry) => {
            occupied_entry.remove();
            stream
                .send(SPacket::Account(types::SAccount::Success))
                .unwrap()
        }
        std::collections::hash_map::Entry::Vacant(_) => {
            stream
                .send(SPacket::Account(types::SAccount::InvalidToken))
                .unwrap();
        }
    }
}
