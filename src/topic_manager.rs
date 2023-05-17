use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::Receiver;
use tokio::io::{AsyncWriteExt};
use tokio::sync::Mutex;

pub struct TopicManager {
    topics: Arc<Mutex<HashMap<String, Vec<OwnedWriteHalf>>>>,
}

impl TopicManager {
    pub fn new(
    ) -> Self {
        Self {
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run(
        &mut self,
        mut receiver: Receiver<(String, OwnedWriteHalf)>,
        mut messages: Receiver<(String, String)>,
        mut closer: Receiver<(String, SocketAddr)>
    ) {
        let mut handles = vec![];

        let receiver_handle = {
            let tclone = self.topics.clone();
            tokio::spawn(async move {
                loop {
                    if let Some((topic, writer)) = receiver.recv().await {
                        println!("Adding client {:?} to topic {:?}", writer.peer_addr().unwrap(), topic);

                        // New scope to drop the lock on self.topics
                        {
                            let mut topics = tclone.lock().await;

                            if let Some(writers) = topics.get_mut(&topic) {
                                writers.push(writer);
                            } else {
                                topics.insert(topic, vec![writer]);
                            }
                        }
                    }
                }
            })
        };
        handles.push(receiver_handle);

        let messages_handle = {
            let tclone = self.topics.clone();
            tokio::spawn(async move {
                loop {
                    if let Some((topic, message)) = messages.recv().await {
                        // New scope to drop the lock on self.topics
                        {
                            let mut topics = tclone.lock().await;
                            if let Some(writers) = topics.get_mut(&topic) {
                                for writer in writers {
                                    println!("Sending message \"{:?}\" to: {:?}", message, writer.peer_addr().unwrap());
                                    writer.write_u32(message.len() as u32).await.unwrap();
                                    writer.write_all(message.as_bytes()).await.unwrap();
                                }
                            }
                        }
                    }
                }
            })
        };
        handles.push(messages_handle);

        let closer_handle = {
            let tclone = self.topics.clone();
            tokio::spawn(async move {
                loop {
                    if let Some((topic, addr)) = closer.recv().await {
                        // New scope to drop the lock on self.topics
                        {
                            let mut topics = tclone.lock().await;
                            if let Some(writers) = topics.get_mut(&topic) {
                                println!("Removing client: {:?}", addr);
                                writers.retain(|writer| {
                                    let peer_addr = writer.peer_addr().unwrap();
                                    peer_addr != addr
                                });
                            }
                        }
                    }
                }
            })
        };
        handles.push(closer_handle);

        for handle in handles {
            handle.await.unwrap();
        }
    }
}