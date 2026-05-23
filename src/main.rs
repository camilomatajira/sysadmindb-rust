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
                match parse_log(&line) {
                    Ok(log) => {
                        println!("Parsed log: {:?}", log);
                    }
                    Err(error) => {
                        println!("Error while parsing log");
                    }
                }
            }
            Err(e) => {
                println!("Error reading frame: {}", e);
                break;
            }
        }
    }
}

use regex::Regex;
use std::sync::OnceLock;

fn log_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<(?<prival>[0-9]+)>(?<version>[0-9])?\s?(?<date>([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]+(Z|[+-][0-9]{2}:[0-9]{2})|\w{3}\s[0-9]{2}\s[0-9]{2}:[0-9]{2}:[0-9]{2}))\s(?<hostname>[\w.]+)\s(?<appname>[\w.]+)\s?\[?(?<procid>[0-9-]+)?\]?\:?\s?(?<msgid>(-|\w{2}[0-9]{2}))?\s?(?<structureddata>(\[.+\]|-))?\s?(BOM)?(?<msg>.+)?").unwrap())
}

#[derive(Debug)]
struct Log {
    version: Option<u32>,
    prival: u32,
    date: String,
    hostname: String,
    appname: String,
    procid: String,
    msgid: String,
    structureddata: String,
    msg: String,
}
fn parse_log(line: &str) -> Result<Log, String> {
    let Some(caps) = log_pattern().captures(&line) else {
        return Err("sorry".to_string());
    };

    Ok(Log {
        prival: caps["prival"].parse().unwrap(),
        version: caps.name("version").map(|m| m.as_str().parse().unwrap()),
        date: caps["date"].to_owned(),
        hostname: caps["hostname"].to_owned(),
        appname: caps["appname"].to_owned(),
        procid: caps["procid"].to_owned(),
        msgid: caps["msgid"].to_owned(),
        structureddata: caps["structureddata"].to_owned(),
        msg: caps
            .name("msg")
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default(),
    })
}
