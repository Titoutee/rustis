mod resp;

use resp::{RespHandler, RedisValue};
use std::{thread};
use anyhow::Result;
use tokio::net::{TcpListener};

async fn handle_connection(mut handler: RespHandler) {
    loop {
        let val = handler.read_value().await.unwrap();
        let response = if let Some(v) = val { // If there is a valid value read from the buffer
            let (command, args) = extract_cmd(v).unwrap();

            match command.to_lowercase().as_str() {
                "ping" => RedisValue::SimpleString("PONG".to_string()),
                "echo" => args.first().unwrap().clone(),
                c => panic!("Erroneous command to handle: {}", c),
            }
        } else {
            break;
        };

        handler.write_value(response).await.unwrap();
    }
}

fn extract_cmd(val: RedisValue) -> Result<(String, Vec<RedisValue>)> {
    match val {
        RedisValue::Array(a) => {
            Ok((
                unpack_bulk_str(a.first().unwrap().clone())?,
                a.into_iter().skip(1).collect(),
            ))
        },
        _ => Err(anyhow::anyhow!("Command is not formed properly (not an array of Redis values?)(V): {:?}", val)),
    }
}

fn unpack_bulk_str(val: RedisValue) -> Result<String> {
    match val {
        RedisValue::BulkString(s) => Ok(s),
        _ => Err(anyhow::anyhow!("Unpacking invalid bulk string(V): {:?}", val)),
    }
}
#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    loop {
        let stream = listener.accept().await;

        match stream {
            Ok((stream, _)) => {
                let handler = RespHandler::new(stream);
                thread::spawn(|| handle_connection(handler));
            }
            Err(e) => {
                println!("Stream error: {}", e);
            }
        }
    }
}
