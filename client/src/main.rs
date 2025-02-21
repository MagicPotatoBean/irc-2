use connection::Connection;
use cursive::{
    event::Event,
    theme::Palette,
    view::Nameable,
    views::{
        self, Button, EditView, HideableView, LinearLayout, ListView, ResizedView, TextArea,
        TextView,
    },
};

mod connection;

fn main() {
    let mut c = cursive::default();
    c.add_global_callback(Event::Key(cursive::event::Key::Esc), |s| s.quit());
    c.set_theme(cursive::theme::Theme {
        shadow: true,
        borders: cursive::theme::BorderStyle::Simple,
        palette: Palette::terminal_default(),
    });

    let conn = Connection::new("zoe.soutter.com:65432").unwrap();
    let conn2 = Connection::new("zoe.soutter.com:65432").unwrap();
    c.set_user_data(AppState {
        main_connection: Some(conn),
        secondary_connection: Some(conn2),
        ..Default::default()
    });

    let main_app = cursive::views::Dialog::around(
        LinearLayout::vertical()
            .child(ListView::new().with_name("message_list"))
            .child(
                LinearLayout::vertical()
                    .child(
                        EditView::new()
                            .on_submit(|s, text| {
                                s.focus_name("msg_box").unwrap();
                            })
                            .with_name("dest_box"),
                    )
                    .child(ResizedView::new(
                        cursive::view::SizeConstraint::AtLeast(20),
                        cursive::view::SizeConstraint::Free,
                        EditView::new()
                            .on_submit(|s, text| {
                                let recipients = s
                                    .find_name::<EditView>("dest_box")
                                    .unwrap()
                                    .get_content()
                                    .clone();
                                s.with_user_data(|dat: &mut AppState| {
                                    eprintln!("recipients: {:?}", recipients);
                                    let recipients: Vec<_> = recipients
                                        .split(",")
                                        .map(|usr| {
                                            eprintln!("ATTEMPTING TO ADD USER: {usr:?}");
                                            let x = usr
                                                .trim()
                                                .chars()
                                                .filter(|chr| chr.is_alphanumeric())
                                                .collect();
                                            eprintln!("ADD USER: {x:?}");
                                            x
                                        })
                                        .collect();
                                    eprintln!("SENT MESSAGE: {recipients:?}");
                                    dat.main_connection
                                        .as_mut()
                                        .unwrap()
                                        .send_message(recipients, text.to_string())
                                        .unwrap();
                                })
                                .unwrap();
                                s.find_name::<EditView>("msg_box").unwrap().set_content("");
                            })
                            .with_name("msg_box"),
                    )),
            ),
    );
    let login_dialog = cursive::views::Dialog::around(
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
            .child(Button::new("Confirm", |s| {
                let (username, password) = (
                    s.find_name::<TextArea>("un_dialog")
                        .unwrap()
                        .get_content()
                        .trim()
                        .to_owned(),
                    s.find_name::<TextArea>("pw_dialog")
                        .unwrap()
                        .get_content()
                        .trim()
                        .to_owned(),
                );
                eprintln!("Logged in as {username:?}");
                s.pop_layer();
                let AppState {
                    main_connection: main_conn,
                    secondary_connection: recv_conn,
                    ..
                } = s.take_user_data().unwrap();
                let mut main_conn = main_conn.unwrap();
                let _ = main_conn.create_account(username.clone(), &password);
                let sink = s.cb_sink().to_owned();

                main_conn.login(username.to_string(), &password).unwrap();
                s.set_user_data(AppState {
                    main_connection: Some(main_conn),
                    ..Default::default()
                });

                std::thread::Builder::new()
                    .name("Message handler".to_string())
                    .spawn(move || {
                        let mut conn = recv_conn.unwrap();
                        conn.login(username.to_string(), &password).unwrap();
                        loop {
                            eprintln!("Started reciever loop");
                            let msg = conn.recv_message().unwrap();
                            eprintln!("Message incoming: {msg:?}");
                            sink.send(Box::new(|s| {
                                s.call_on_name("message_list", |e: &mut ListView| {
                                    e.add_child(
                                        "",
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
            })),
    )
    .with_name("login_dialog");
    c.add_layer(main_app);
    c.add_layer(login_dialog);

    c.run();
}
#[derive(Default)]
struct AppState {
    main_connection: Option<Connection>,
    secondary_connection: Option<Connection>,
}
