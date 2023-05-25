use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use std::collections::HashMap;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use postcard::{from_bytes, to_allocvec};
use tokio::net::tcp::OwnedReadHalf;

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Message {
    TopicChange(String),
    Send(String),
    CloseConnection,
}

struct Broadcaster {
    sender: broadcast::Sender<Message>,
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let bmap: Arc<Mutex<HashMap<String, Broadcaster>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let state = bmap.clone();
        tokio::spawn(async move {
            match handle_connection(stream, state).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error handling connection: {}", e);
                }
            }
        });
    }
}

#[derive(Debug)]
enum ReadError {
    EndOfStream,
    ReadSizeMismatch,
    ConnectionError(std::io::Error),
    PostcardError(postcard::Error),
}

async fn read_message(stream: &mut OwnedReadHalf) -> Result<Message, ReadError> {
    let size: u32;
    match stream.read_u32().await {
        Ok(s) => {
            if s == 0 {
                return Err(ReadError::EndOfStream);
            }
            size = s;
        }
        Err(e) => {
            return Err(ReadError::ConnectionError(e));
        }
    }

    let buf = &mut vec![0; size as usize];
    match stream.read_exact(buf).await {
        Ok(s) => {
            if s == 0 {
                return Err(ReadError::EndOfStream);
            } else if s != size as usize {
                return Err(ReadError::ReadSizeMismatch);
            }
            match from_bytes::<Message>(buf) {
                Ok(m) => Ok(m),
                Err(e) => Err(ReadError::PostcardError(e)),
            }
        }
        Err(e) => {
            Err(ReadError::ConnectionError(e))
        }
    }
}

async fn handle_connection(stream: TcpStream, state: Arc<Mutex<HashMap<String, Broadcaster>>>) -> std::io::Result<()> {
    let sender: broadcast::Sender<Message>;
    let mut receiver: broadcast::Receiver<Message>;

    let (mut reader, mut writer) = stream.into_split();

    // Get the initial topic
    match read_message(&mut reader).await {
        Ok(Message::TopicChange(topic)) => {
            // Check for a topic change and update the broadcaster
            let mut bmap = state.lock().await;
            let b = bmap.entry(topic.clone()).or_insert_with(|| {
                let (s, _) = broadcast::channel(1);
                Broadcaster {
                    sender: s,
                }
            });
            sender = b.sender.clone();
            receiver = b.sender.subscribe();
        }
        _ => {
            // Close the connection if we get anything other than a TopicChange
            return Ok(());
        }
    }

    // Start listening for messages on the receiver and messages from other clients on the same topic
    loop {
        let handle = read_message(&mut reader);
        tokio::pin!(handle);
        loop {
            tokio::select! {
                res = &mut handle => {
                    match res {
                        Ok(Message::CloseConnection) => {
                            println!("Got a close connection");
                            return Ok(());
                        }
                        Ok(Message::TopicChange(_topic)) => {
                            println!("Got a topic change");
                        }
                        Ok(msg) => {
                            sender.send(msg).unwrap();
                            break;
                        }
                        Err(e) => {
                            eprintln!("Error reading message: {:?}", e);
                            return Ok(());
                        }
                    }
                }
                res = receiver.recv() => {
                    match res {
                        Ok(Message::Send(msg)) => {
                            let msg = Message::Send(msg);
                            let mut buf = to_allocvec(&msg).unwrap();
                            let size = buf.len() as u32;
                            let mut size_buf = size.to_be_bytes().to_vec();
                            size_buf.append(&mut buf);
                            writer.write_all(&size_buf).await.unwrap();
                        }
                        _ => {
                            println!("Got something from the broadcaster")
                        }
                    }
                }
            }
        }
    }
}

// Testing tokio select logic
#[cfg(test)]
mod tests {
    async fn test1() {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
    async fn test2() {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn test_read_message() {
        let mut count = 0;
        let mut handle1 = test1();
        tokio::pin!(handle1);

        loop {
            tokio::select! {
                _ = &mut handle1 => {
                    println!("handle1 done");
                    count += 1;
                    if count == 2 {
                        break;
                    }

                    handle1.set(test1());
                }
                _ = test2() => {
                    println!("handle2 done");
                }
            }
        }
    }
}