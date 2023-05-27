use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
	TopicChange(String),
	Send(String),
	CloseConnection,
	Acknowledge,
	Error(String),
	Custom(Vec<u8>),
	Ping,
	Pong,
}

#[derive(Debug)]
pub enum ReadError {
	EndOfStream,
	ReadSizeMismatch,
	ConnectionError(std::io::Error),
	PostcardError(postcard::Error),
}
