use net_message::asymmetric::AsymmetricTcpStream;
use rand::Rng;
use rsa::{RsaPrivateKey, RsaPublicKey, rand_core};
use std::{
    collections::{HashMap, VecDeque},
    net::TcpListener,
    sync::{LazyLock, RwLock},
    time::Duration,
};
use types::{CPacket, SPacket, enc::RsaData};

static TOKEN_MAP: LazyLock<RwLock<HashMap<u128, TokenData>>> =
    LazyLock::new(|| HashMap::new().into());
static ACCOUNT_MAP: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| HashMap::new().into());

#[derive(Clone, Debug)]
struct TokenData {
    username: Option<String>,
    rsa_key: RsaPublicKey,
    aes_key: Vec<u8>,
    incoming_messages: VecDeque<u128>,
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:65432").unwrap();
    let client = listener.accept().unwrap().0;
    client.set_nonblocking(false).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(60)))
        .unwrap();
    let mut stream = AsymmetricTcpStream::<SPacket, CPacket>::new_unchecked(client);

    println!("Generating server RSA keys, please allow a few seconds for this to happen");
    let priv_key = RsaPrivateKey::new(&mut rand_core::OsRng, 2048).unwrap();
    let pub_key = priv_key.to_public_key();
    println!("Generated keys");

    println!("Waiting");

    while let Ok(pack) = stream.read() {
        println!("{pack:?}");
        match pack {
            CPacket::Handshake { client_key } => handshake(&mut stream, client_key, priv_key),
            CPacket::Account(c_account) => match c_account {
                types::CAccount::Login { token, creds } => {
                    let token = token.get(&priv_key).unwrap();
                    match TOKEN_MAP.write().unwrap().entry(token) {
                        std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
                            let creds = creds.get(&occupied_entry.get().aes_key).unwrap();

                            if Some(&creds.pw_digest)
                                == ACCOUNT_MAP.read().unwrap().get(&creds.username)
                            {
                                occupied_entry.get_mut().username = Some(creds.username);
                                println!("Logged in");
                            } else {
                                println!("invalid creds");
                            }
                        }
                        std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                            stream.send(SPacket::InvalidToken).unwrap();
                        }
                    }
                }
                types::CAccount::Create { token, creds } => {
                    let token = token.get(&priv_key).unwrap();
                    match TOKEN_MAP.write().unwrap().entry(token) {
                        std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                            let creds = creds.get(&occupied_entry.get().aes_key).unwrap();

                            if ACCOUNT_MAP.read().unwrap().contains_key(&creds.username) {
                                println!("Account already exists");
                            } else {
                                ACCOUNT_MAP
                                    .write()
                                    .unwrap()
                                    .insert(creds.username, creds.pw_digest);
                            }
                        }
                        std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                            stream.send(SPacket::InvalidToken).unwrap();
                        }
                    }
                }
            },
        }
    }
}
fn handshake(
    stream: &mut AsymmetricTcpStream<SPacket, CPacket>,
    client_key: RsaPublicKey,
    priv_key: RsaPrivateKey,
) {
    println!("handshake");
    let aes_key: [u8; 16] = rand::rng().random(); // 1024 bit key
    let user = TokenData {
        username: None,
        rsa_key: client_key.clone(),
        aes_key: aes_key.to_vec(),
        incoming_messages: VecDeque::new(),
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
            shared_key: RsaData::new(aes_key.to_vec(), &client_key).unwrap(),
            token: RsaData::new(token, &client_key).unwrap(),
        })
        .unwrap();
    println!("Sent off handshake");
}
fn handle(stream: &mut AsymmetricTcpStream<SPacket, CPacket>) {
    while let Ok(packet) = stream.read() {
        match packet {
            CPacket::Handshake { client_key } => todo!(),
            CPacket::Account(c_account) => match c_account {
                types::CAccount::Login { token, creds } => todo!(),
                types::CAccount::Create { token, creds } => todo!(),
            },
        }
    }
}
