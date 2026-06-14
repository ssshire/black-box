/// TCP server binary — `cargo run --bin tcp`
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use bb::command_handler::Command;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

type ClientTx = mpsc::UnboundedSender<String>;

/// channel_name → (max_size, list of (addr, ClientTx))
type ChannelMap = Arc<Mutex<HashMap<String, (usize, Vec<(SocketAddr, ClientTx)>)>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("blackbox tcp server listening on :8080");

    let channels: ChannelMap = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("[+] client connected: {}", addr);

        let channels = Arc::clone(&channels);
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, addr, channels).await {
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

                Command::ListChannels => {
                    let map = channels.lock().await;
                    if map.is_empty() {
                        send_to(&tx, "no channels yet. create one with /create <name> <size>\n");
                    } else {
                        let mut output = String::from("channels:\n");
                        for (name, (max_size, members)) in map.iter() {
                            output.push_str(&format!(
                                "  #{:<20} {}/{} members\n",
                                name,
                                members.len(),
                                max_size
                            ));
                        }
                        send_to(&tx, &output);
                    }
                }

                Command::CreateChannel { ref name, size } => {
                    let mut map = channels.lock().await;
                    if map.contains_key(name) {
                        send_to(&tx, &format!("channel '{}' already exists. use /join.\n", name));
                    } else {
                        map.insert(name.clone(), (size, Vec::new()));
                        send_to(&tx, &format!("channel '{}' created (max {}). use /join {} to enter.\n", name, size, name));
                    }
                }

                Command::JoinChannel { ref channel_name } => {
                    let mut map = channels.lock().await;

                    match map.get(channel_name) {
                        None => {
                            send_to(&tx, &format!(
                                "channel '{}' not found. use /list to see available channels or /create to make one.\n",
                                channel_name
                            ));
                        }
                        Some((max_size, subscribers)) if subscribers.len() >= *max_size => {
                            send_to(&tx, &format!("channel '{}' is full.\n", channel_name));
                        }
                        Some(_) => {
                            if let Some(old) = current_channel.take() {
                                remove_client(&mut map, &old, addr);
                            }

                            let (_, subscribers) = map.get_mut(channel_name).unwrap();
                            subscribers.push((addr, tx.clone()));
                            current_channel = Some(channel_name.clone());
                            send_to(&tx, &format!("joined #{}\n", channel_name));
                            broadcast(
                                subscribers,
                                &format!("*** {} joined #{}\n", user_id, channel_name),
                                Some(addr),
                            );
                        }
                    }
                }

                Command::SendMessage { ref content } => {
                    if let Some(ch) = &current_channel {
                        let outgoing = format!("[{}] {}: {}\n", ch, user_id, content);
                        let mut map = channels.lock().await;
                        if let Some((_, subscribers)) = map.get_mut(ch) {
                            broadcast(subscribers, &outgoing, Some(addr));
                        }
                        send_to(&tx, &format!("[{}] you: {}\n", ch, content));
                    } else {
                        send_to(&tx, "you are not in a channel. use /list to see channels or /create to make one.\n");
                    }
                }
            },
        }
    }

    if let Some(ch) = &current_channel {
        let mut map = channels.lock().await;
        remove_client(&mut map, ch, addr);
        if let Some((_, subs)) = map.get_mut(ch) {
            broadcast(subs, &format!("*** {} left #{}\n", user_id, ch), None);
        }
    }

    Ok(())
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
    map: &mut HashMap<String, (usize, Vec<(SocketAddr, ClientTx)>)>,
    channel: &str,
    addr: SocketAddr,
) {
    if let Some((_, subs)) = map.get_mut(channel) {
        subs.retain(|(a, _)| *a != addr);
    }
}