use connection::Connection;
use cursive::{
    event::Event,
    theme::Palette,
    view::Nameable,
    views::{self, Button, EditView, LinearLayout, ListView, ResizedView, TextView},
    Cursive,
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
                        LinearLayout::horizontal()
                            .child(ResizedView::new(
                                cursive::view::SizeConstraint::Fixed(13),
                                cursive::view::SizeConstraint::Fixed(1),
                                TextView::new("Destination: "),
                            ))
                            .child(ResizedView::new(
                                cursive::view::SizeConstraint::AtLeast(20),
                                cursive::view::SizeConstraint::Fixed(1),
                                EditView::new()
                                    .on_submit(|s, text| {
                                        s.focus_name("msg_box").unwrap();
                                    })
                                    .with_name("dest_box"),
                            )),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(ResizedView::new(
                                cursive::view::SizeConstraint::Fixed(13),
                                cursive::view::SizeConstraint::Fixed(1),
                                TextView::new("Message: "),
                            ))
                            .child(ResizedView::new(
                                cursive::view::SizeConstraint::AtLeast(20),
                                cursive::view::SizeConstraint::Fixed(1),
                                EditView::new()
                                    .on_submit(|s, text| {
                                        let recipients = s
                                            .find_name::<EditView>("dest_box")
                                            .unwrap()
                                            .get_content()
                                            .clone();
                                        s.with_user_data(|dat: &mut AppState| {
                                            let recipients: Vec<_> = recipients
                                                .split(",")
                                                .map(|usr| {
                                                    let x = usr
                                                        .trim()
                                                        .chars()
                                                        .filter(|chr| chr.is_alphanumeric())
                                                        .collect();
                                                    x
                                                })
                                                .collect();
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
                        EditView::new()
                            .on_submit(|s, text| {
                                if !text.is_empty() {
                                    let _ = s.focus_name("pw_dialog");
                                }
                            })
                            .with_name("un_dialog"),
                    )),
            )
            .child(
                LinearLayout::horizontal()
                    .child(views::TextView::new("Password"))
                    .child(ResizedView::new(
                        cursive::view::SizeConstraint::AtLeast(8),
                        cursive::view::SizeConstraint::Fixed(1),
                        EditView::new()
                            .on_submit(|s, text| {
                                if !text.is_empty() {
                                    login(s);
                                }
                            })
                            .with_name("pw_dialog"),
                    )),
            )
            .child(Button::new("Confirm", login).with_name("submit_btn")),
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
fn login(s: &mut Cursive) {
    let (username, password) = (
        s.find_name::<EditView>("un_dialog")
            .unwrap()
            .get_content()
            .trim()
            .to_owned(),
        s.find_name::<EditView>("pw_dialog")
            .unwrap()
            .get_content()
            .trim()
            .to_owned(),
    );
    if username.is_empty() || password.is_empty() {
        return;
    }
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
                let msg = conn.recv_message().unwrap();
                sink.send(Box::new(|s| {
                    s.call_on_name("message_list", |e: &mut ListView| {
                        e.add_child(
                            format!(
                                "[{}->{}]:",
                                msg.sender,
                                msg.recipients
                                    .into_iter()
                                    .reduce(|acc, e| format!("{acc},{e}"))
                                    .unwrap(),
                            ),
                            TextView::new(msg.contents),
                        );
                    })
                    .unwrap()
                }))
                .unwrap()
            }
        })
        .unwrap();
}
