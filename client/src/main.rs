use std::{
    io::Write,
    net::ToSocketAddrs,
    sync::mpsc::{self, channel, Sender},
    time::Duration,
};

use connection::Connection;
use cursive::{
    event::Event,
    theme::Palette,
    view::Nameable,
    views::{
        self, Button, Dialog, HideableView, LinearLayout, ListView, ResizedView, TextArea, TextView,
    },
    CbSink, View,
};
use net_message::asymmetric::AsymmetricTcpStream;
use types::{CPacket, InboundMessage, SPacket};

mod connection;

fn main() {
    let mut c = cursive::default();
    c.add_global_callback('q', |s| s.quit());
    c.add_global_callback(Event::Key(cursive::event::Key::Esc), |s| s.quit());
    c.set_theme(cursive::theme::Theme {
        shadow: true,
        borders: cursive::theme::BorderStyle::Simple,
        palette: Palette::terminal_default(),
    });

    let conn = Connection::new("zoe.soutter.com:65432").unwrap();
    let conn2 = Connection::new("zoe.soutter.com:65432").unwrap();
    let (tx, rx) = channel();
    c.set_user_data(AppState {
        msg_tx: Some(tx),
        main_connection: Some(conn),
        secondary_connection: Some(conn2),
    });

    let main_app = cursive::views::Dialog::around(
        LinearLayout::vertical()
            .child(ListView::new().with_name("message_list"))
            .child(
                LinearLayout::horizontal()
                    .child(ResizedView::new(
                        cursive::view::SizeConstraint::AtLeast(20),
                        cursive::view::SizeConstraint::Free,
                        TextArea::new().with_name("msg_box"),
                    ))
                    .child(Button::new("Send", |s| {
                        let contents = s
                            .find_name::<TextArea>("msg_box")
                            .unwrap()
                            .get_content()
                            .to_string();
                        s.find_name::<TextArea>("msg_box").unwrap().set_content("");
                        s.with_user_data(|dat: &mut AppState| {
                            dat.main_connection
                                .as_mut()
                                .unwrap()
                                .send_message(
                                    vec!["zoe".to_string(), "thomas".to_string()],
                                    contents,
                                )
                                .unwrap();
                        })
                        .unwrap()
                    })),
            ),
    );
    let login_dialog = HideableView::new(cursive::views::Dialog::around(
        cursive::views::LinearLayout::vertical()
            .child(TextView::new("Login").center())
            .child(
                LinearLayout::horizontal()
                    .child(views::TextView::new("Username"))
                    .child(ResizedView::new(
                        cursive::view::SizeConstraint::AtLeast(8),
                        cursive::view::SizeConstraint::Fixed(1),
                        TextArea::new().with_name("un_dialog"),
                    )),
            )
            .child(
                LinearLayout::horizontal()
                    .child(views::TextView::new("Password"))
                    .child(ResizedView::new(
                        cursive::view::SizeConstraint::AtLeast(8),
                        cursive::view::SizeConstraint::Fixed(1),
                        TextArea::new().with_name("pw_dialog"),
                    )),
            )
            .child(Button::new("Confirm", |e| {
                let (username, password) = (
                    e.find_name::<TextArea>("un_dialog").unwrap(),
                    e.find_name::<TextArea>("pw_dialog").unwrap(),
                );
                e.pop_layer();
                let AppState {
                    msg_tx,
                    main_connection: pri,
                    secondary_connection: sec,
                } = e.take_user_data().unwrap();
                let (username, password) = (username.get_content(), password.get_content());
                let mut pri = pri.unwrap();
                let _ = pri.create_account(username.to_string(), password);
                message_reciever(
                    sec.unwrap(),
                    username.to_string(),
                    password,
                    msg_tx.unwrap(),
                );
                pri.login(username.to_string(), password).unwrap();
                e.set_user_data(AppState {
                    msg_tx: None,
                    main_connection: Some(pri),
                    secondary_connection: None,
                });
            })),
    ))
    .with_name("login_dialog");
    c.add_layer(main_app);
    c.add_layer(login_dialog);

    let sink = c.cb_sink().to_owned();
    std::thread::Builder::new()
        .name("ReaderThread".to_owned())
        .spawn(move || {
            let data = rx;
            while let Ok(msg) = data.recv() {
                sink.send(Box::new(|s| {
                    s.call_on_name("message_list", |e: &mut ListView| {
                        e.add_child(
                            "msg",
                            TextView::new(format!(
                                "[{}->{}]: {}",
                                msg.sender,
                                msg.recipients
                                    .into_iter()
                                    .reduce(|acc, e| format!("{acc},{e}"))
                                    .unwrap(),
                                msg.contents
                            )),
                        );
                    })
                    .unwrap()
                }))
                .unwrap()
            }
        })
        .unwrap();

    c.run();
}
struct AppState {
    msg_tx: Option<Sender<InboundMessage>>,
    main_connection: Option<Connection>,
    secondary_connection: Option<Connection>,
}

fn message_reciever(
    mut conn: Connection,
    username: String,
    password: &str,
    tx: Sender<InboundMessage>,
) {
    conn.login(username, password).unwrap();

    std::thread::spawn(move || loop {
        let msg = conn.recv_message().unwrap();
        let recievers = msg
            .recipients
            .clone()
            .into_iter()
            .reduce(|acc, new| format!("{acc}, {new}"))
            .unwrap();
        let _ = tx.send(msg);
    });
}
