/// TCP server binary — `cargo run --bin tcp`
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use bb::command_handler::Command;
use bb::db::{self, AuthedUser};

use sqlx::PgPool;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

type ClientTx = mpsc::UnboundedSender<String>;

struct ChannelState {
    max_size: usize,
    session_id: Option<i32>,
    members: Vec<(SocketAddr, ClientTx)>,
}

/// channel_name → ChannelState
type ChannelMap = Arc<Mutex<HashMap<String, ChannelState>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pool = db::connect().await?;

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("blackbox tcp server listening on 127.0.0.1:8080");
    let channels: ChannelMap = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("[+] client connected: {}", addr);

        let channels = Arc::clone(&channels);
        let pool = pool.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, addr, channels, pool).await {
                eprintln!("[!] client {} error: {}", addr, e);
            }
            println!("[-] client disconnected: {}", addr);
        });
    }
}

async fn handle_client(
    socket: TcpStream,
    addr: SocketAddr,
    channels: ChannelMap,
    pool: PgPool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (reader, mut writer) = socket.into_split();
    let mut lines = BufReader::new(reader).lines();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = writer.write_all(msg.as_bytes()).await;
        }
    });

    let mut current_channel: Option<String> = None;
    let mut authed_user: Option<AuthedUser> = None;

    let user_id = {
        use rand::Rng;
        format!("user_{}", rand::thread_rng().gen_range(10000..99999))
    };

    send_to(&tx, "welcome to blackbox. type /help for commands.\n");

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let input = if !line.starts_with('/') && current_channel.is_some() {
            format!("/msg {}", line)
        } else {
            line.clone()
        };

        match Command::parse(&input) {
            Err(e) => {
                send_to(&tx, &format!("error: {}\n", e));
            }
            Ok(cmd) => match cmd {
                Command::Help => {
                    send_to(&tx, Command::help_text());
                    send_to(&tx, "\n");
                }

                Command::Exit => {
                    send_to(&tx, "goodbye.\n");
                    break;
                }

                Command::Register { username, email, password } => {
                    match db::register_user(&pool, &username, &email, &password).await {
                        Ok(user) => {
                            send_to(&tx, &format!("registered and logged in as {}\n", user.username));
                            authed_user = Some(user);
                        }
                        Err(e) => send_to(&tx, &format!("error: {}\n", e)),
                    }
                }

                Command::Login { username, password } => {
                    match db::login_user(&pool, &username, &password).await {
                        Ok(user) => {
                            send_to(&tx, &format!("logged in as {}\n", user.username));
                            authed_user = Some(user);
                        }
                        Err(e) => send_to(&tx, &format!("error: {}\n", e)),
                    }
                }

                Command::ListChannels => {
                    if current_channel.is_some() {
                        send_to(&tx, "error: /list is only available before you join a channel.\n");
                    } else {
                        let map = channels.lock().await;
                        if map.is_empty() {
                            send_to(&tx, "no channels yet. create one with /create <name> <size>\n");
                        } else {
                            let mut output = String::from("channels:\n");
                            for (name, state) in map.iter() {
                                output.push_str(&format!(
                                    "  #{:<20} {}/{} members\n",
                                    name,
                                    state.members.len(),
                                    state.max_size
                                ));
                            }
                            send_to(&tx, &output);
                        }
                    }
                }

                Command::CreateChannel { ref name, size } => {
                    let mut map = channels.lock().await;
                    if map.contains_key(name) {
                        send_to(&tx, &format!("error: channel '{}' already exists. use /join.\n", name));
                    } else {
                        let session_id = match &authed_user {
                            Some(user) => match db::create_session(&pool, name, user.user_id, size as i32).await {
                                Ok(id) => Some(id),
                                Err(e) => {
                                    eprintln!("[!] failed to persist session '{}': {}", name, e);
                                    None
                                }
                            },
                            None => None,
                        };
                        map.insert(
                            name.clone(),
                            ChannelState { max_size: size, session_id, members: Vec::new() },
                        );
                        send_to(&tx, &format!("channel '{}' created (max {}). use /join {} to enter.\n", name, size, name));
                    }
                }

                Command::JoinChannel { ref channel_name } => {
                    let mut map = channels.lock().await;

                    match map.get(channel_name) {
                        None => {
                            send_to(&tx, &format!(
                                "error: channel '{}' not found. use /list to see available channels or /create to make one.\n",
                                channel_name
                            ));
                        }
                        Some(state) if state.members.len() >= state.max_size => {
                            send_to(&tx, &format!("error: channel '{}' is full.\n", channel_name));
                        }
                        Some(_) => {
                            if let Some(old) = current_channel.take() {
                                remove_client(&mut map, &old, addr);
                            }

                            let state = map.get_mut(channel_name).unwrap();
                            state.members.push((addr, tx.clone()));
                            current_channel = Some(channel_name.clone());
                            send_to(&tx, &format!("joined #{}\n", channel_name));
                            broadcast(
                                &mut state.members,
                                &format!("*** {} joined #{}\n", display_name(&authed_user, &user_id), channel_name),
                                Some(addr),
                            );
                        }
                    }
                }

                Command::LeaveChannel => {
                    if let Some(ch) = current_channel.take() {
                        let mut map = channels.lock().await;
                        remove_client(&mut map, &ch, addr);
                        if let Some(state) = map.get_mut(&ch) {
                            broadcast(&mut state.members, &format!("*** {} left #{}\n", display_name(&authed_user, &user_id), ch), None);
                        }
                        send_to(&tx, &format!("left #{}\n", ch));
                    } else {
                        send_to(&tx, "error: you are not in a channel.\n");
                    }
                }

                Command::SendMessage { ref content } => {
                    if let Some(ch) = &current_channel {
                        let outgoing = format!("{}: {}\n", display_name(&authed_user, &user_id), content);
                        let session_id = {
                            let mut map = channels.lock().await;
                            let session_id = map.get(ch).and_then(|s| s.session_id);
                            if let Some(state) = map.get_mut(ch) {
                                broadcast(&mut state.members, &outgoing, Some(addr));
                            }
                            session_id
                        };
                        send_to(&tx, &format!("you: {}\n", content));

                        if let (Some(session_id), Some(user)) = (session_id, &authed_user) {
                            if let Err(e) = db::insert_message(&pool, session_id, user.user_id, content).await {
                                eprintln!("[!] failed to persist message in #{}: {}", ch, e);
                            }
                        }
                    } else {
                        send_to(&tx, "error: you are not in a channel. use /list to see channels or /create to make one.\n");
                    }
                }
            },
        }
    }

    if let Some(ch) = &current_channel {
        let mut map = channels.lock().await;
        remove_client(&mut map, ch, addr);
        if let Some(state) = map.get_mut(ch) {
            broadcast(&mut state.members, &format!("*** {} left #{}\n", display_name(&authed_user, &user_id), ch), None);
        }
    }

    Ok(())
}

fn display_name(authed_user: &Option<AuthedUser>, fallback_id: &str) -> String {
    authed_user.as_ref().map(|u| u.username.clone()).unwrap_or_else(|| fallback_id.to_string())
}

fn send_to(tx: &ClientTx, msg: &str) {
    let _ = tx.send(msg.to_string());
}

fn broadcast(
    subscribers: &mut Vec<(SocketAddr, ClientTx)>,
    msg: &str,
    exclude: Option<SocketAddr>,
) {
    subscribers.retain(|(addr, tx)| {
        if Some(*addr) == exclude {
            return true;
        }
        tx.send(msg.to_string()).is_ok()
    });
}

fn remove_client(
    map: &mut HashMap<String, ChannelState>,
    channel: &str,
    addr: SocketAddr,
) {
    if let Some(state) = map.get_mut(channel) {
        state.members.retain(|(a, _)| *a != addr);
    }
}