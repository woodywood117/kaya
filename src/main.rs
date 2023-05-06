use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Message {
    #[serde(rename = "Data")]
    data: String,
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            match handle_connection(stream).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error handling connection: {}", e);
                }
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let esize = stream.read_u32().await?;
    let buf = &mut vec![0; esize as usize];
    let asize = stream.read_exact(buf).await?;

    if asize != esize as usize {
        eprintln!("Error reading data from stream!");
        return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Error reading data from stream!"));
    }

    let message = serde_json::from_slice::<Message>(buf)?;
    println!("Received message: {:?}", message);

    stream.shutdown().await?;
    Ok(())
}
