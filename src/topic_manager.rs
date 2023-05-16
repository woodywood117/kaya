use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::Receiver;
use tokio::io::{AsyncWriteExt};

pub struct TopicManager {
    topics: HashMap<String, Vec<OwnedWriteHalf>>,
    receiver: Receiver<(String, OwnedWriteHalf)>,
    messages: Receiver<(String, String)>,
    closer: Receiver<(String, SocketAddr)>
}

impl TopicManager {
    pub fn new(
        receiver: Receiver<(String, OwnedWriteHalf)>,
        messages: Receiver<(String, String)>,
        closer: Receiver<(String, SocketAddr)>
    ) -> Self {
        Self {
            topics: HashMap::new(),
            receiver,
            messages,
            closer
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Some((topic, writer)) = self.receiver.recv() => {
                    println!("Adding client {:?} to topic {:?}", writer.peer_addr().unwrap(), topic);
                    if let Some(writers) = self.topics.get_mut(&topic) {
                        writers.push(writer);
                    } else {
                        self.topics.insert(topic, vec![writer]);
                    }
                }
                Some((topic, message)) = self.messages.recv() => {
                    if let Some(writers) = self.topics.get_mut(&topic) {
                        for writer in writers {
                            println!("Sending message \"{:?}\" to: {:?}", message, writer.peer_addr().unwrap());
                            writer.write_u32(message.len() as u32).await.unwrap();
                            writer.write_all(message.as_bytes()).await.unwrap();
                        }
                    }
                }
                Some((topic, addr)) = self.closer.recv() => {
                    if let Some(writers) = self.topics.get_mut(&topic) {
                        println!("Removing client: {:?}", addr);
                        writers.retain(|writer| {
                            let peer_addr = writer.peer_addr().unwrap();
                            peer_addr != addr
                        });
                    }
                }
            }
        }
    }
}