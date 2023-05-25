use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
	TopicChange(String),
	Send(String),
	CloseConnection,
}

#[derive(Debug)]
pub enum ReadError {
	EndOfStream,
	ReadSizeMismatch,
	ConnectionError(std::io::Error),
	PostcardError(postcard::Error),
}
