mod ext;
mod resp;

use anyhow::Result;
use core::{option::Option::None, time::Duration};
pub use ext::{Notify, RedisValueInner, StackCtr};
pub use resp::{Database, RedisArray, RedisInt, RedisValue, RespHandler, ThreadSafeDb};
use std::collections::VecDeque;
pub use std::{
    clone,
    collections::HashMap,
    sync::{Arc, Mutex},
};
pub use tokio::net::{TcpListener, TcpStream};

pub fn notify(notification: Notify, content: &str) {
    let prefix = String::from(match notification {
        Notify::Info => "(Info)",
        Notify::Recv => "(Recv)",
        Notify::RecvRaw => "(RecvRaw)",
        Notify::Send => "(Send)",
        Notify::SendRaw => "(SendRaw)",
    });

    println!("{}   >>>>>   {}", prefix, content);
}

#[derive(Debug)]
struct Transaction {
    in_transaction: bool,
    queue: VecDeque<Option<RedisValue>>,
}

impl Transaction {
    fn init() -> Self {
        Transaction {
            in_transaction: false,
            queue: VecDeque::new(),
        }
    }

    fn push(&mut self, elt: Option<RedisValue>) {
        self.queue.push_front(elt);
    }

    fn pop(&mut self) -> Option<Option<RedisValue>> {
        self.queue.pop_back()
    }
}

pub async fn handle_connection(
    stream: TcpStream,
    id: usize,
    map: ThreadSafeDb,
    client_id: Arc<Mutex<StackCtr>>,
) {
    notify(Notify::Info, &format!("Client (id)[{}] here!", id));
    let mut handler = RespHandler::new(stream, id, map);
    let mut transaction = Transaction::init();

    let ok = "+OK\r\n";
    let empty_arr = "*0\r\n";

    loop {
        let val = handler.read_value().await.unwrap_or_else(|e| {
            eprintln!("Error reading value: {}", e);
            return None; // Gracefully return None to break out of the loop
        });

        if transaction.in_transaction {
            transaction.push(val);
        } else {
            let response = if let Some(v) = val {
                notify(Notify::Recv, &v.clone().serialize());
                let (command, args) = extract_cmd(v).unwrap();
                match command.to_ascii_lowercase().as_str() {
                    "multi" => {
                        transaction.in_transaction = true;
                        RedisValue::SimpleString(ok.to_string())
                    }
                    "exec" => {
                        if !transaction.in_transaction {
                            RedisValue::ErrorMsg(Vec::from("Exec called with empty queue..."));
                        } else if transaction.queue.is_empty() {
                            transaction.in_transaction = false;
                            RedisValue::ErrorMsg(Vec::from(empty_arr));
                        }
                        RedisValue::NullBulkString
                    }

                    any => match any {
                        "ping" => RedisValue::SimpleString("PONG".to_string()),
                        "echo" => args.first().unwrap().clone(),
                        "set" => {
                            println!("{:?}", args);
                            let mut args_iter = args.iter();
                            let (key, value, sub1, sub2) = (
                                args_iter.next(),
                                args_iter.next(),
                                args_iter.next(),
                                args_iter.next(),
                            );

                            let exp = if let Some(cmd) = sub1 {
                                match cmd
                                    .unpack_str_variant()
                                    .unwrap()
                                    .to_ascii_lowercase()
                                    .as_str()
                                {
                                    "px" => {
                                        if let Some(d) = sub2 {
                                            let milli = d
                                                .unpack_str_variant()
                                                .unwrap()
                                                .parse::<u64>()
                                                .unwrap();
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
                                .insert(key.unwrap(), value.unwrap().clone(), exp)
                                .await;
                            RedisValue::SimpleString("Ok".to_string())
                        }
                        "get" => {
                            if let Some(a) = handler.get_val(args.first().unwrap()).await {
                                a
                            } else {
                                RedisValue::NullBulkString
                            }
                        }
                        "incr" => {
                            let key = &args.iter().next().unwrap().clone();
                            let val = handler.get_val(&key).await;

                            let response = match val {
                                Some(RedisValue::Int(_)) | None => {
                                    if let Some(n) = handler.incr(key).await {
                                        n
                                    } else {
                                        RedisValue::NullBulkString
                                    }
                                }
                                _ => RedisValue::ErrorMsg(Vec::from(
                                    "(error) value is not an integer or out of range",
                                )),
                            };
                            response
                        }

                        c => panic!("Erroneous command to handle: {}", c),
                    },
                }
            } else {
                break;
            };

            notify(Notify::Send, &response.clone().serialize());

            if let Err(e) = handler.write_value(response).await {
                eprintln!("Error writing value to to-client handler's buffer: {}", e);
                break; // Stop processing if writing fails
            }
        }
    }
    handler.cleanup();
    client_id.lock().expect("unlock failed!").release(id);
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

pub fn unpack_bulk_str(val: RedisValue) -> Result<String> {
    match val {
        RedisValue::BulkString(s) => Ok(s),
        _ => Err(anyhow::anyhow!(
            "Unpacking invalid bulk string(V): {:?}",
            val
        )),
    }
}

#[cfg(test)]
mod test {
    use crate::RedisValue;
    use crate::unpack_bulk_str;

    #[test]
    fn _unpack_bulk_str() {
        let val = RedisValue::BulkString("Should unwrap".to_string());
        assert_eq!(unpack_bulk_str(val).unwrap(), "Should unwrap".to_string());
    }

    #[test]
    #[should_panic]
    fn unpack_bulk_str_panic_unwrap() {
        let val = RedisValue::Int(4);
        unpack_bulk_str(val).unwrap();
    }

    #[test]
    #[should_panic]
    fn unpack_bulk_str_panic_eq() {
        let val = RedisValue::BulkString("Should unwrap".to_string());
        assert_eq!(unpack_bulk_str(val).unwrap(), "Shouldunwrap".to_string());
    }
}