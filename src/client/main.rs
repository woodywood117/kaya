use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use serde::{Serialize, Deserialize};
use postcard::{from_bytes, to_allocvec};
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    topic: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Message {
    TopicChange(String),
    Send(String),
    CloseConnection,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();

    // Send topic change
    let message = Message::TopicChange(args.topic);
    let buf = to_allocvec(&message).unwrap();
    stream.write_u32(buf.len() as u32).await.unwrap();
    stream.write_all(&buf).await.unwrap();

    // Main event loop
    let mut input = String::new();
    let (mut reader, mut writer) = stream.into_split();
    tokio::spawn(async move {
        loop {
            let esize = reader.read_u32().await.unwrap();
            let buf = &mut vec![0; esize as usize];
            let asize = reader.read_exact(buf).await.unwrap();
            if asize != esize as usize {
                eprintln!("Error reading data from stream!");
                return;
            }

            let message: Message = from_bytes(buf).unwrap();
            println!("{message:?}");
        }
    });

    loop {
        // Read string from stdin
        std::io::stdin().read_line(&mut input).unwrap();
        let message = input.trim().to_string().clone();

        // Create message
        let message = Message::Send(message);
        let message = to_allocvec(&message).unwrap();

        // Write message size
        writer.write_u32(message.len() as u32).await.unwrap();
        writer.write_all(message.as_slice()).await.unwrap();

        // Clear input
        input.clear();
    }
}
