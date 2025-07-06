mod resp;

use anyhow::Result;
use core::{option::Option::None, time::Duration};
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
            println!("Received \"{}\" call", command.to_ascii_lowercase());
            match command.to_ascii_lowercase().as_str() {
                "ping" => {
                    //println!("Received PING call");
                    RedisValue::SimpleString(format!("PONG"))
                }
                "echo" => {
                    //println!("Received ECHO call");²
                    args.first().unwrap().clone()
                }
                "set" => {
                    println!("{:?}", args);
                    let mut args_iter = args.iter();
                    // #[warn(soft_unstable)]
                    let (key, value, sub1, sub2) = (
                        args_iter.next(),
                        args_iter.next(),
                        args_iter.next(),
                        args_iter.next(),
                    );


                    //println!("{:?}, {:?}, {:?}, {:?}", key.unwrap(), value.unwrap(), sub1.unwrap(), sub2.unwrap());

                    let exp = if let Some(cmd) = sub1 {
                        // println!("out of PX");
                        println!("{}", cmd.unpack_str_variant().unwrap());
                        match cmd.unpack_str_variant().unwrap() {
                            "PX" => {
                                println!("in PX");
                                if let Some(d) = sub2 {
                                    let milli =
                                        d.unpack_str_variant().unwrap().parse::<u64>().unwrap();

                                    Some(Duration::from_millis(milli))
                                } else {
                                    None
                                }
                            }

                            _ => None,
                        }
                    } else {
                        None
                    };
                    handler
                        .insert(key.unwrap().clone(), value.unwrap().clone(), exp)
                        .await;
                    RedisValue::SimpleString("Ok".to_string())
                }
                "get" => {
                    if let Some(a) = handler.get(args.first().unwrap().clone()).await {
                        a
                    } else {
                        RedisValue::NullBulkString
                    }
                }
                c => panic!("Erroneous command to handle: {}", c),
            }
        } else {
            break;
        };

        println!("Sending value {:?}", response);

        if let Err(e) = handler.write_value(response).await {
            // Serialization happens here
            eprintln!("Error writing value: {}", e);
            break; // Stop processing if writing fails
        }
    }
}

fn extract_cmd(val: RedisValue) -> Result<(String, Vec<RedisValue>)> {
    match val {
        RedisValue::SimpleString(s) => Ok((s, vec![])),
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
                tokio::spawn(async move { handle_connection(stream).await });
            }
            Err(e) => {
                println!("Stream error: {}", e);
            }
        }
    }
}
