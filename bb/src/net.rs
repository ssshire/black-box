use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;

pub enum ServerEvent {
    Line(String),
    Disconnected,
}

pub struct ServerConnection {
    writer: TcpStream,
    pub events: mpsc::Receiver<ServerEvent>,
}

pub fn connect(addr: &str) -> io::Result<ServerConnection> {
    let stream = TcpStream::connect(addr)?;
    let reader_stream = stream.try_clone()?;
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut lines = BufReader::new(reader_stream).lines();
        while let Some(Ok(line)) = lines.next() {
            if tx.send(ServerEvent::Line(line)).is_err() {
                return;
            }
        }
        let _ = tx.send(ServerEvent::Disconnected);
    });

    Ok(ServerConnection {
        writer: stream,
        events: rx,
    })
}

impl ServerConnection {
    pub fn send_line(&mut self, line: &str) -> io::Result<()> {
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()
    }
}
