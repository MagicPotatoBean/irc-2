use std::{io::Write, net::ToSocketAddrs, sync::mpsc};

use connection::Connection;
use types::InboundMessage;

mod connection;

fn main() {
    print!("Enter username: ");
    std::io::stdout().flush().unwrap();
    let mut username = String::default();
    std::io::stdin().read_line(&mut username).unwrap();
    username = username
        .trim()
        .chars()
        .filter(|chr| chr.is_alphanumeric())
        .collect();

    print!("Enter password: ");
    std::io::stdout().flush().unwrap();
    let mut password = String::default();
    std::io::stdin().read_line(&mut password).unwrap();
    password = password.trim().to_owned();

    println!("Connecting");
    let mut conn = Connection::new("zoe.soutter.com:65432").unwrap();
    let _ = conn.create_account(username.to_string(), &password);
    message_reciever("zoe.soutter.com:65432", username.to_string(), &password);
    if conn.login(username.to_string(), &password).is_ok() {
        println!("Logging in");
    } else {
        println!("Failed to log in; shutting down.");
        return;
    }

    loop {
        print!("Recipients: ");
        std::io::stdout().flush().unwrap();
        let mut input = String::default();
        std::io::stdin().read_line(&mut input).unwrap();
        let recipients: Vec<_> = input
            .split(",")
            .map(|name| name.trim().to_owned())
            .collect();
        print!("Message: ");
        std::io::stdout().flush().unwrap();
        let mut input = String::default();
        std::io::stdin().read_line(&mut input).unwrap();
        input = input.trim().to_owned();

        conn.send_message(recipients, input).unwrap()
    }
}

fn message_reciever<A: ToSocketAddrs>(
    addr: A,
    username: String,
    password: &str,
) -> mpsc::Receiver<InboundMessage> {
    let mut conn = Connection::new(addr).unwrap();
    conn.login(username, password).unwrap();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        loop {
            let msg = conn.recv_message().unwrap();
            let recievers = msg
                .recipients
                .clone()
                .into_iter()
                .reduce(|acc, new| format!("{acc}, {new}"))
                .unwrap();
            println!("\n{} -> {}: {}", msg.sender, recievers, msg.contents);
            let _ = tx.send(msg);
        }
    });

    rx
}
