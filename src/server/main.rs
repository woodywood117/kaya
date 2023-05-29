use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use std::collections::HashMap;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use postcard::{from_bytes, to_allocvec};
use tokio::net::tcp::OwnedReadHalf;
use kaya::{Message, ReadError};
use clap::Parser;

type BroadcasterMap = Arc<Mutex<HashMap<String, (i32, Option<broadcast::Sender<Message>>)>>>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Address to listen on
	#[arg(short, long, default_value = "0.0.0.0")]
	addr: String,
	/// Port to listen on
	#[arg(short, long, default_value = "8080")]
	port: String,
}

#[tokio::main]
async fn main() {
	let args = Args::parse();
	let addr = format!("{}:{}", args.addr, args.port);

	println!("Listening on {}", addr);
	let listener = TcpListener::bind(addr).await.unwrap();
	let bmap = BroadcasterMap::default();

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

async fn handle_connection(stream: TcpStream, state: BroadcasterMap) -> std::io::Result<()> {
	let mut sender: broadcast::Sender<Message>;
	let mut receiver: broadcast::Receiver<Message>;
	let mut conntopic: String;

	let (mut reader, mut writer) = stream.into_split();

	// Get the initial topic
	match read_message(&mut reader).await {
		Ok(Message::TopicChange(topic)) => {
			// TODO: Check for topic len being under some size limit
			// Check for a topic change and update the broadcaster
			conntopic = topic.clone();
			let mut bmap = state.lock().await;
			let b = bmap.entry(topic.clone()).or_insert_with(|| {
				let (s, _) = broadcast::channel(1);
				(0, Some(s))
			});
			let s = b.1.take().unwrap();
			sender = s.clone();
			receiver = sender.subscribe();
			b.0 += 1;
			b.1 = Some(s);
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
                        Ok(Message::TopicChange(topic)) => {
							// TODO: Check for topic len being under some size limit
							// Check for a topic change and update the broadcaster
							if conntopic == topic {
								continue;
							}
							conntopic = topic.clone();
							{
								let mut bmap = state.lock().await;
								let b = bmap.entry(topic.clone()).or_insert_with(|| {
									(1, None)
								});
								b.0 -= 1;
							}

							let mut bmap = state.lock().await;
							let b = bmap.entry(topic.clone()).or_insert_with(|| {
								let (s, _) = broadcast::channel(1);
								(1, Some(s))
							});
							let s = b.1.take().unwrap();
							sender = s.clone();
							receiver = sender.subscribe();
							b.0 += 1;
							b.1 = Some(s);
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