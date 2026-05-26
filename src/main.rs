use axum::extract::Query;
use axum::http::StatusCode;
use axum::{Json, extract::State};
use axum::{Router, debug_handler, routing::get, routing::post};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use sqlx::QueryBuilder;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = SqlitePool::connect(&database_url).await.unwrap();

    let tcp_pool = pool.clone();
    let http_pool = pool.clone();

    let tcp_server = tokio::spawn(async move {
        run_tcp_server(tcp_pool).await;
    });

    let http_server = tokio::spawn(async move {
        run_http_server(http_pool).await;
    });

    tokio::join!(tcp_server, http_server);

    println!("Listening");
}

async fn run_tcp_server(pool: SqlitePool) {
    let listener = TcpListener::bind("127.0.0.1:1999").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let pool_clone = pool.clone();
        println!("Accepted");
        tokio::spawn(async move {
            process(socket, pool_clone).await;
        });
    }
}

async fn run_http_server(pool: SqlitePool) {
    tracing_subscriber::fmt::init(); // prints to stdout
    let app = Router::new()
        .route("/", post(get_all_logs))
        .with_state(pool)
        .layer(TraceLayer::new_for_http()); // add this

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct LogQuery {
    command: String,
}

#[axum::debug_handler]
async fn get_all_logs(
    State(pool): State<SqlitePool>,
    Query(query_params): Query<HashMap<String, String>>,
    Json(payload): Json<LogQuery>,
) -> Result<String, StatusCode> {
    let mut builder = QueryBuilder::new(
        r#"
      SELECT
          original_msg
      FROM logs
      WHERE 1=1
  "#,
    );
    if let Some(h) = query_params.get("date_gt") {
        builder.push("AND timestamp >");
        builder.push_bind(h);
    }
    if let Some(h) = query_params.get("appname") {
        builder.push("AND appname =");
        builder.push_bind(h);
    }
    if let Some(h) = query_params.get("hostname") {
        builder.push("AND hostname =");
        builder.push_bind(h);
    }

    let rows: Vec<String> = builder
        .build_query_scalar()
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let concateneted_rows = rows.join("\n");
    println!("{:?}", rows);

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("restricted_bin");

    let mut child = Command::new("/bin/bash")
        .arg("--norc")
        .arg("--noprofile")
        .arg("--restricted")
        .arg("-c")
        .env("PATH", &path)
        .arg(&payload.command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| {
            println!("Path: {:?}", &path);
            eprintln!("ERROR executing '{}': {}", &payload.command, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(concateneted_rows.as_bytes())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    println!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn process(socket: TcpStream, db: SqlitePool) {
    let mut framed = Framed::new(socket, LinesCodec::new());
    while let Some(result) = framed.next().await {
        match result {
            Ok(line) => {
                println!("Got log: {}", line);
                match parse_log(&line) {
                    Ok(log) => {
                        println!("Parsed log: {:?}", log);

                        match sqlx::query!("INSERT INTO logs ('prival', 'version' ,'date' ,'hostname' ,'appname' ,'procid' ,'msgid' ,'structureddata' ,'msg' ,'original_msg', 'timestamp') VALUES (?,?, ?, ?, ?, ?, ?, ?, ?, ?,?)", log.prival, log.version, log.date, log.hostname, log.appname, log.procid, log.msgid, log.structureddata, log.msg, log.original_msg, log.timestamp)
                            .execute(&db)
                            .await{
                                Ok(_) => println!("Log inserted to db"),
                                Err(error) => println!("Problems inserting log in db {}", error)
                            }
                    }
                    Err(error) => {
                        println!("Error while parsing log {}", error);
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

// #[derive(Debug)]
#[derive(Serialize, Debug, sqlx::FromRow)]
struct Log {
    original_msg: String,
    version: Option<i64>,
    prival: i64,
    date: String,
    hostname: String,
    appname: String,
    procid: String, // or i64 if you change the migration
    msgid: String,
    structureddata: String,
    msg: String,
    timestamp: DateTime<Utc>,
}

fn parse_log(line: &str) -> Result<Log, String> {
    let Some(caps) = log_pattern().captures(&line) else {
        return Err("sorry".to_string());
    };

    Ok(Log {
        original_msg: line.to_owned(),
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
        timestamp: Utc::now(),
    })
}
