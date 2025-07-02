mod resp;

use anyhow::Result;
use resp::{RedisValue, RespHandler};
use tokio::net::{TcpListener, TcpStream};

async fn handle_connection(stream: TcpStream) {
    let mut handler = RespHandler::new(stream);
    loop {
        let val = handler.read_value().await.unwrap_or_else(|e| {
            eprintln!("Error reading value: {}", e);
            return None; // Gracefully return None to break out of the loop
        });

        let response = if let Some(v) = val {
            // If there is a valid value: read from the buffer
            let (command, args) = extract_cmd(v).unwrap();
            println!("dsdfs");
            match command.to_lowercase().as_str() {
                "ping" => {
                    println!("Received PING call");
                    RedisValue::SimpleString("PONGO".to_string())
                }
                "echo" => {
                    println!("Received ECHO call");
                    args.first().unwrap().clone()
                }
                c => panic!("Erroneous command to handle: {}", c),
            }
        } else {
            break;
        };

        println!("Sending value {:?}", response);

        if let Err(e) = handler.write_value(response).await { // Serialization happens here
            eprintln!("Error writing value: {}", e);
            break; // Stop processing if writing fails
        }
    }
}

fn extract_cmd(val: RedisValue) -> Result<(String, Vec<RedisValue>)> {
    match val {
        RedisValue::SimpleString(s) => {
            Ok((s, vec![]))
        }
        RedisValue::Array(a) => Ok((
            unpack_bulk_str(a.first().unwrap().clone())?,
            a.into_iter().skip(1).collect(),
        )),
        _ => Err(anyhow::anyhow!(
            "Command is not formed properly (not an array of Redis values?)(V): {:?}",
            val
        )),
    }
}

fn unpack_bulk_str(val: RedisValue) -> Result<String> {
    match val {
        RedisValue::BulkString(s) => Ok(s),
        _ => Err(anyhow::anyhow!(
            "Unpacking invalid bulk string(V): {:?}",
            val
        )),
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6378").await.unwrap();
    loop {
        let stream = listener.accept().await;

        match stream {
            Ok((stream, _)) => {
                tokio::spawn(async move {handle_connection(stream).await});
            }
            Err(e) => {
                println!("Stream error: {}", e);
            }
        }
    }
}
