mod topic_manager;

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::Sender;

#[tokio::main]
async fn main() {
    let (conn_sender, conn_receiver) = tokio::sync::mpsc::channel::<(String, OwnedWriteHalf)>(1);
    let (mesg_sender, mesg_receiver) = tokio::sync::mpsc::channel::<(String, String)>(1);
    let (close_sender, close_receiver) = tokio::sync::mpsc::channel::<(String, SocketAddr)>(1);
    let mut topic_manager = topic_manager::TopicManager::new(conn_receiver, mesg_receiver, close_receiver);
    tokio::spawn(async move {
        topic_manager.run().await;
    });

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let conn_sender = conn_sender.clone();
        let mesg_sender = mesg_sender.clone();
        let close_sender = close_sender.clone();
        tokio::spawn(async move {
            match handle_connection(stream, conn_sender, mesg_sender, close_sender).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error handling connection: {}", e);
                }
            }
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    conn: Sender<(String, OwnedWriteHalf)>,
    mesg: Sender<(String, String)>,
    close: Sender<(String, SocketAddr)>,
) -> std::io::Result<()> {
    let esize = stream.read_u32().await?;
    let buf = &mut vec![0; esize as usize];
    let asize = stream.read_exact(buf).await?;
    if asize != esize as usize {
        eprintln!("Error reading data from stream!");
        stream.shutdown().await?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "Error reading data from stream!",
        ));
    }

    let (mut reader, writer) = stream.into_split();
    let topic = String::from_utf8_lossy(buf);
    println!("New client on topic: {:?}, {:?}", topic, writer.peer_addr().unwrap());
    conn.send((topic.clone().into(), writer)).await.unwrap();

    loop {
        match reader.read_u32().await {
            Ok(esize) => {
                let buf = &mut vec![0; esize as usize];
                if let Ok(asize) = reader.read_exact(buf).await {
                    if asize != esize as usize {
                        eprintln!("Error reading data from stream!");
                        close.send((topic.clone().into(), reader.peer_addr().unwrap())).await.unwrap();
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::BrokenPipe,
                            "Error reading data from stream!",
                        ));
                    }
                } else {
                    close.send((topic.clone().into(), reader.peer_addr().unwrap())).await.unwrap();
                    return Ok(());
                }

                let message = String::from_utf8_lossy(buf);
                println!("Received new message: {:?}, {:?}", message, reader.peer_addr().unwrap());
                mesg.send((topic.clone().into(), message.into())).await.unwrap();
            }
            Err(_) => {
                close.send((topic.clone().into(), reader.peer_addr().unwrap())).await.unwrap();
                return Ok(());
            }
        };
    }
}
