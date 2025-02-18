use net_message::asymmetric::AsymmetricTcpStream;
use rsa::{RsaPrivateKey, RsaPublicKey, rand_core};
use std::{net::TcpStream, time::Duration};
use types::{
    CPacket, Credentials, SPacket,
    enc::{AesData, RsaData},
};

fn main() {
    let client = TcpStream::connect("127.0.0.1:65432").unwrap();
    client.set_nonblocking(false).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(60)))
        .unwrap();
    let mut stream = AsymmetricTcpStream::<CPacket, SPacket>::new_unchecked(client);

    println!("Generating handshake, this may take a while");
    let priv_key = RsaPrivateKey::new(&mut rand_core::OsRng, 2048).unwrap();
    let pub_key = priv_key.to_public_key();

    let conn_info = handshake(&mut stream, priv_key, pub_key).unwrap();
}
struct ConnectionInfo {
    token: u128,
    aes_key: Vec<u8>,
    server_key: RsaPublicKey,
}
fn handshake(
    stream: &mut AsymmetricTcpStream<CPacket, SPacket>,
    priv_key: RsaPrivateKey,
    pub_key: RsaPublicKey,
) -> Option<ConnectionInfo> {
    stream
        .send(CPacket::Handshake {
            client_key: pub_key,
        })
        .unwrap();
    if let Ok(SPacket::Handshake {
        server_key,
        shared_key,
        token,
    }) = stream.read()
    {
        Some(ConnectionInfo {
            token: token.get(&priv_key).unwrap(),
            aes_key: shared_key.get(&priv_key).unwrap(),
            server_key,
        })
    } else {
        None
    }
}
