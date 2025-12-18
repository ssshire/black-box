// import necessary modules from the rust libraries
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener,TcpStream};
use std::error::Error;
 // handling a single client (single messages)
async fn process(mut socket: TcpStream) {
    let mut buffer = [0; 1024]; // initalizes the cap on the buffer
 
    loop {
        let n = match socket.read(&mut buffer).await {
            Ok(0) => {
                println!("Client disconnected");
                return; // client will close the connection 
            }
            Ok(n) => n,
            Err(e) => {
                println!("Failed to read from the client: {}", e);
                return;
            }
        };
        let msg = String::from_utf8_lossy(&buffer[..n]); // initalizes the variable to read each byte
        println!("Server received: {:?}", msg); // print for debugging 
    }
}
// establishing an open connection via tokio
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
// spawn a future task -> awaiting incoming connections
    let listener = TcpListener::bind("0.0.0.0:8080").await?; // listener variable 
    println!("Opening path for incoming connections..."); // message for ui 

    loop { // loops each client interacting with the single state machine 
        let (socket, addr) = listener.accept().await?; // the listener variable will wait for a
                                                       // client to connect
        println!("Client connected: {}", addr); // print for debugging/monitoring active clients

        tokio::spawn(async move {
            process(socket).await;
        });
    }
}
