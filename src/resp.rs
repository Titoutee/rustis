use crate::RedisValueInner;
use crate::RediSer;
use crate::Transaction;
use crate::{EMPTY_ARR, OK};
use anyhow::Result;
use bytes::BytesMut;
use core::option::Option::{self, None};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    vec,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, Instant},
};

pub type RedisInt = i64;
pub type RedisArray = Vec<RedisInt>;
pub type Database = HashMap<String, Set>;
type LockedDb = Mutex<Database>;
pub type ThreadSafeDb = Arc<LockedDb>;
pub type Keys = HashSet<String>;

#[derive(Debug, Clone)]
pub struct Set {
    org: Instant,
    pub exp: Option<Duration>,
    pub val: RedisValue,
}

impl Set {
    pub fn new(val: RedisValue, exp: Option<Duration>) -> Self {
        Self {
            org: Instant::now(),
            exp,
            val,
        }
    }

    pub fn from_other(other: &Self, value: RedisValue) -> Self {
        let mut new = other.clone();
        new.val = value;
        new
    }

    pub fn rtime_valid(&self) -> bool {
        if let Some(e) = self.exp {
            self.org.elapsed().as_millis() <= e.as_millis()
        } else {
            true
        }
    }
}

#[allow(dead_code)]
pub struct RespHandler {
    client_id: usize,
    stream: TcpStream,
    buffer: BytesMut,
    pub map: ThreadSafeDb, // *Database*
    self_keys: Keys,       // Server-side for safety reasons
}

/// Standalone remove_entry procedure
pub fn _remove_entry_stdln(db: &mut ThreadSafeDb, key: &str) {
    db.lock().expect("unlock failed!").remove_entry(key);
}

impl<'a> RespHandler {
    pub fn new(stream: TcpStream, id: usize, map: ThreadSafeDb) -> Self {
        Self {
            client_id: id,
            stream,
            buffer: BytesMut::with_capacity(512),
            map: map,
            self_keys: HashSet::new(),
        }
    }

    pub(crate) async fn handle_multi(&mut self, transaction: &mut Transaction) -> RedisValue {
        transaction.switch_trans();
        RedisValue::SimpleString(OK.to_string())
    }

    pub(crate) async fn handle_exec(&mut self, transaction: &mut Transaction) -> RedisValue {
        if !transaction.in_transaction {
            RedisValue::ErrorMsg(Vec::from("Exec called with empty queue..."));
        } else if transaction.queue.is_empty() {
            transaction.switch_neutral();
            RedisValue::ErrorMsg(Vec::from(EMPTY_ARR));
        }

        transaction.switch_exec();
        RedisValue::Command(RedisCommand::EXEC)
    }

    pub async fn handle_command(&mut self, command: &str, args: Vec<RedisValue>) -> RedisValue {
        match command {
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
                                let milli = d.unpack_str_variant().unwrap().parse::<u64>().unwrap();
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
                self.insert(key.unwrap(), value.unwrap().clone(), exp).await;
                RedisValue::SimpleString(OK.to_string())
            }
            "get" => {
                if let Some(a) = self.get_val(args.first().unwrap()).await {
                    a
                } else {
                    RedisValue::NullBulkString
                }
            }
            "incr" => {
                let key = &args.iter().next().unwrap().clone();
                let val = self.get_val(&key).await;

                let response = match val {
                    Some(RedisValue::Int(_)) | None => {
                        if let Some(n) = self.incr(key).await {
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
        }
    }

    fn keyize(&self, key: &RedisValue) -> String {
        format!("{}{}", key.keyize(), self.client_id)
    }

    pub async fn read_value(&mut self) -> Result<Option<RedisValue>> {
        let bytes_read = self.stream.read_buf(&mut self.buffer).await?;

        if bytes_read == 0 {
            return Ok(None);
        }

        // Here ignore the length result
        let (v, _) = parse_msg(self.buffer.split())?;
        Ok(Some(v))
    }

    pub async fn write_value<T: RediSer>(&mut self, value: T) -> Result<usize> {
        Ok(self.stream.write(value.serialize().as_bytes()).await?)
    }

    pub async fn insert(&mut self, redval: &RedisValue, value: RedisValue, exp: Option<Duration>) {
        let key = self.keyize(redval);
        self.add_entry(key, value, exp);
    }

    pub fn remove_entry(&mut self, key: &String) {
        // Remove both from the database and the client handle's inner keys memory
        self.map.lock().unwrap().remove(key);
        self.self_keys.remove(key);
    }

    pub fn add_entry(&mut self, key: String, value: RedisValue, exp: Option<Duration>) {
        self.map
            .lock()
            .expect("unlock failed!")
            .insert(key.clone(), Set::new(value, exp));
        self.self_keys.insert(key);
    }

    pub fn cleanup(&mut self) {
        let old_map_len = self.map.lock().expect("unlock failed!").len();
        println!("Cleanup performs on client (id)[{}]", self.client_id);
        self.self_keys
            .iter()
            .for_each(|s| _remove_entry_stdln(&mut self.map, s));
        println!(
            "map length went from {} to {} entries",
            old_map_len,
            self.map.lock().expect("unlock failed!").len()
        );
    }

    /// Increments the key-corresponding value up to the current value added with `add`.
    /// As it signatures enforces, matching the `RedisValue::Int` variants apart from the others is done beforehand.
    ///
    /// If the Set is not key-present when incrementing, it is inserted with a default value of 1.
    pub async fn incr(
        &mut self,
        key: &RedisValue, /* Should be RedisValue::Int() */
    ) -> Option<RedisValue> {
        // let key = self.keyize(key);
        // let res = self.map.lock().unwrap().get(key)?.val.clone();
        let (res, new_set) = if let Some(r) = self.map.lock().unwrap().get_mut(&key.keyize()) {
            let val = RedisValue::Int(r.val.unpack_int_variant()? + 1);
            (
                RedisValue::Int(r.val.unpack_int_variant()?),
                Set::from_other(r, val),
            )
        } else {
            (RedisValue::Int(0), Set::new(RedisValue::Int(1), None)) // If key is not present, default is val 1 and no expiry (as one cannot conceptually be decided)
            // and preceding value is 0.
        };

        self.insert(key, new_set.val, new_set.exp).await;
        Some(res)
    }

    pub async fn get_set(&mut self, key: &RedisValue) -> Option<Set> {
        let key = self.keyize(key);
        let set = self
            .map
            .lock()
            .unwrap()
            .get(&key)
            .map(|inner| inner.clone())?; // Clone happens here but could happen in `handle_connection` under "GET"
        if set.rtime_valid() {
            Some(set)
        } else {
            self.remove_entry(&key);
            None
        }
    }

    // Returns owned RedisValue instead (or call `get_set` and access `.val`)
    pub async fn get_val(&mut self, key: &RedisValue) -> Option<RedisValue> {
        let key = self.keyize(key);
        let set = self
            .map
            .lock()
            .unwrap()
            .get(&key)
            .map(|inner| inner.clone())?; // Clone happens here but could happen in `handle_connection` under "GET"
        if set.rtime_valid() {
            Some(set.val)
        } else {
            self.remove_entry(&key);
            None
        }
    }
}

fn parse_msg(buffer: BytesMut) -> Result<(RedisValue, usize)> {
    match buffer[0] as char {
        '+' => parse_simple_string(buffer),
        '$' => parse_bulk_string(buffer),
        '*' => parse_array(buffer),
        ':' => parse_int(buffer),
        _ => Err(anyhow::anyhow!(
            "Not a valid RESP type: {:?}, starting with prefix: '{}' [byte: {}]",
            buffer,
            buffer[0] as char,
            buffer[0]
        )),
    }
}

fn parse_simple_string(buffer: BytesMut) -> Result<(RedisValue, usize)> {
    if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
        let string = String::from_utf8(line.to_vec()).unwrap();
        return Ok((RedisValue::SimpleString(string), len + 1));
    }

    Err(anyhow::anyhow!("Not a valid simple string: {:?}", buffer))
}

fn parse_bulk_string(buffer: BytesMut) -> Result<(RedisValue, usize)> {
    let (bulk_str_len, bytes_consumed) = if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
        let bulk_str_len = _parse_int(line)?;

        (bulk_str_len, len + 1)
    } else {
        return Err(anyhow::anyhow!("Invalid bulk string format: {:?}", buffer));
    };

    let end_of_bulk_str = bytes_consumed + bulk_str_len as usize;
    let parsed = end_of_bulk_str + 2;

    Ok((
        RedisValue::BulkString(String::from_utf8(
            buffer[bytes_consumed..end_of_bulk_str].to_vec(),
        )?),
        parsed,
    ))
}

fn parse_array(buffer: BytesMut) -> Result<(RedisValue, usize)> {
    let (array_length, mut bytes_consumed) =
        if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
            let array_length = _parse_int(line)?;

            (array_length, len + 1)
        } else {
            return Err(anyhow::anyhow!("Invalid array format {:?}", buffer));
        };

    let mut items = vec![];
    for _ in 0..array_length {
        let (array_item, len) = parse_msg(BytesMut::from(&buffer[bytes_consumed..]))?;

        items.push(array_item);
        bytes_consumed += len;
    }

    return Ok((RedisValue::Array(items), bytes_consumed));
}

// /!\ Call with buffer[..], not buffer[1..]

pub fn parse_int(buffer: BytesMut) -> Result<(RedisValue, usize)> {
    let parsed = _parse_int(&buffer[1..])?;
    Ok((RedisValue::Int(parsed), 0))
}

fn _parse_int(buffer: &[u8]) -> Result<RedisInt> {
    Ok(String::from_utf8(buffer.to_vec())?
        .trim()
        .parse::<RedisInt>()?)
}

// (segment, length_of_segment)
fn read_until_crlf(buffer: &[u8]) -> Option<(&[u8], usize)> {
    for i in 1..buffer.len() {
        if buffer[i - 1] == b'\r' && buffer[i] == b'\n' {
            return Some((&buffer[0..(i - 1)], i + 1));
        }
    }

    return None;
}

// Mostly useful in the main (server-side) command handling routine when the MULTI's queue should be flushed
// rather than getting new commands from the TCP pipe on a call to EXEC.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum RedisCommand {
    EXEC, // On EXEC call
}

/// RedisValue represents any object passing through a Redis client or server, may it be an integer, a bulk string or
/// any other main Redis, part of the RESP documentation which can be found [here](https://redis.io/docs/latest/develop/reference/protocol-spec/).
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub enum RedisValue {
    SimpleString(String),
    // Error(Bytes),
    BulkString(String),
    Array(Vec<RedisValue>),

    #[allow(unused)]
    Int(RedisInt),
    // NullArray,
    NullBulkString,
    ErrorMsg(Vec<u8>),

    // Server private
    Command(RedisCommand),
}

// Only RedisValue and Vec<RedisValue> really need to be serialized
impl RediSer for RedisValue {
    fn serialize(&self) -> String {
        match self {
            RedisValue::SimpleString(s) => format!("+{}\r\n", s),
            RedisValue::BulkString(s) => format!("${}\r\n{}\r\n", s.chars().count(), s),
            RedisValue::Int(n) => format!(":{}", n),
            RedisValue::NullBulkString => "$-1\r\n".to_string(),
            RedisValue::Array(v) => {
                // Heavy many clones
                format!(
                    "*{}\r\n{}",
                    v.len(),
                    v.iter()
                        .map(|rv| rv.clone().serialize()) // /!\
                        .collect::<String>()
                )
            }
            RedisValue::ErrorMsg(v) => {
                format!("-{}\r\n{}\r\n", v.len(), String::from_utf8(v.clone()).unwrap())
            } // `v`` is expected to be correctly created at source => safer implementation could be wanted though

            _ => unimplemented!(), // Server internal commands should not leak to clients (what's the point?)
        }
    }
}

impl RediSer for Vec<RedisValue> {
    fn serialize(&self) -> String {
        let mut res = String::new();
        for i in self {
            res.push_str(&i.serialize());
        }
        res
    }
}
impl RedisValue {
    pub fn keyize(&self) -> String {
        self.unpack_for_str().to_string()
    }

    /// Unpacks only variants that hold string types
    pub fn unpack_str_variant(&self) -> Option<&str> {
        match self {
            RedisValue::SimpleString(s) => Some(s),
            RedisValue::BulkString(s) => Some(s),
            _ => None,
        }
    }

    /// Unpacks only variants that hold int types
    pub fn unpack_int_variant(&self) -> Option<RedisInt> {
        match self {
            RedisValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn unpack(&self) -> Option<&dyn RedisValueInner> {
        match self {
            RedisValue::SimpleString(s) => Some(s),
            RedisValue::BulkString(s) => Some(s),
            RedisValue::Int(n) => Some(n),
            _ => unimplemented!(),
        }
    }

    pub fn unpack_for_str(&self) -> &dyn ToString {
        match self {
            RedisValue::SimpleString(s) => s,
            RedisValue::BulkString(s) => s,
            RedisValue::Int(n) => n,
            _ => panic!(
                "String key cannot be constructed from any other value than RedisValue::SimpleString/BulkString/Int"
            ),
        }
    }
}