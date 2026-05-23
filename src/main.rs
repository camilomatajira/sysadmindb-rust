use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:1999").await.unwrap();

    println!("Listening");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        println!("Accepted");
        tokio::spawn(async move {
            process(socket).await;
        });
    }
}

async fn process(socket: TcpStream) {
    let mut framed = Framed::new(socket, LinesCodec::new());
    while let Some(result) = framed.next().await {
        match result {
            Ok(line) => {
                println!("Got log: {}", line);
            }
            Err(e) => {
                println!("Error reading frame: {}", e);
                break;
            }
        }
    }
}
