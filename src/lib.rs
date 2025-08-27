mod ext;
mod resp;

use anyhow::Result;
use core::option::Option::None;
pub use ext::{Notify, RedisValueInner, StackCtr, RediSer};
pub use resp::{Database, RedisArray, RedisInt, RedisValue, RespHandler, ThreadSafeDb};
use std::{collections::VecDeque, fmt::Debug};
pub use std::{
    clone,
    collections::HashMap,
    sync::{Arc, Mutex},
};
pub use tokio::net::{TcpListener, TcpStream};

pub const OK: &'static str = "OK";
pub const EMPTY_ARR: &'static str = "0";
pub const QUEUED: &'static str = "QUEUED";

pub fn notify<T: Debug>(notification: Notify, content: &T) {
    let prefix = String::from(match notification {
        Notify::Info => "(Info)",
        Notify::Recv => "(Recv)",
        Notify::RecvRaw => "(RecvRaw)",
        Notify::Send => "(Send)",
        Notify::SendRaw => "(SendRaw)",
    });

    println!("{}   >>>>>   {:?}", prefix, content);
}

#[derive(Debug)]
struct Transaction {
    in_transaction: bool,
    in_exec: bool,
    queue: VecDeque<Option<RedisValue>>,
}

impl Transaction {
    fn init() -> Self {
        Transaction {
            in_transaction: false,
            in_exec: false,
            queue: VecDeque::new(),
        }
    }

    fn push(&mut self, elt: Option<RedisValue>) {
        self.queue.push_front(elt);
    }

    fn pop(&mut self) -> Option<Option<RedisValue>> {
        self.queue.pop_back()
    }

    fn switch_trans(&mut self) {
        self.in_exec = false; // If for whatever reason it was on
        self.in_transaction = true;
    }

    // After multi
    fn switch_exec(&mut self) {
        self.in_transaction = false;
        self.in_exec = true;
    }

    // After exec
    fn switch_neutral(&mut self) {
        self.in_exec = false;
        self.in_transaction = false; // If for whatever reason it was on
    }
}

// Anything concerning response sending will not be seriously tested, as I know no way to early catch the responses before
// flushing them and still send them through this procedure.
// In the future, the response-crafting

pub async fn handle_connection(
    stream: TcpStream,
    id: usize,
    map: ThreadSafeDb,
    client_id: Arc<Mutex<StackCtr>>,
) {
    notify(Notify::Info, &format!("Client (id)[{}] here!", id));
    let mut handler = RespHandler::new(stream, id, map);
    let mut transaction = Transaction::init();

    loop {
        let val = handler.read_value().await.unwrap_or_else(|e| {
            eprintln!("Error reading value: {}", e);
            return None; // Gracefully return None to break out of the loop
        });

        println!("-------------");

        let response = if let Some(v) = val.clone() {
            notify(Notify::Recv, &v.clone().serialize());
            if transaction.in_transaction {
                transaction.push(val);
                RedisValue::SimpleString(QUEUED.to_string())
            } else {
                let (command, args) = extract_cmd(v).unwrap();
                match command.to_ascii_lowercase().as_str() {
                    "multi" => {
                        handler.handle_multi(&mut transaction).await
                    }
                    "exec" => {
                        handler.handle_exec(&mut transaction).await
                    }

                    any => handler.handle_command(any, args).await,
                }
            }
        } else {
            break;
        };

        let to_send = 
        if let RedisValue::Command(c) = &response {
            notify(Notify::Send, &transaction.queue);
            transaction.switch_neutral();
            transaction.queue.clone().into_iter().map(|o| o.unwrap()).collect::<Vec<RedisValue>>()
        } else {
            notify(Notify::Send, &response.clone().serialize());
            vec![response]
        };
        

        if let Err(e) = handler.write_value(to_send).await {
            eprintln!("Error writing value to to-client handler's buffer: {}", e);
            break; // Stop processing if writing fails
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
