use std::{collections::HashMap, sync::{Mutex}};

use anyhow::Result;
use bytes::{BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

// pub type Bytes = Vec<u8>;
// type RedisResult = Result<Option<(usize, RedisBufSplit)>, RESPError>;

pub struct RespHandler {
    stream: TcpStream,
    buffer: BytesMut,
    map: Mutex<HashMap<RedisValue, RedisValue>>, 
}

impl RespHandler {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: BytesMut::with_capacity(512),
            map: Mutex::new(HashMap::new()),
        }
    }

    pub async fn read_value(&mut self) -> Result<Option<RedisValue>> {
        let bytes_read = self.stream.read_buf(&mut self.buffer).await?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let (v, _) = parse_msg(self.buffer.split())?;
        Ok(Some(v))
    }

    pub async fn write_value(&mut self, value: RedisValue) -> Result<usize> {
        Ok(self.stream.write(value.serialize().as_bytes()).await?)
    }

    pub async fn insert(&mut self, key: RedisValue, value: RedisValue) {
        self.map.lock().unwrap().insert(key, value); // Previous state and old value are of no use
    }

    pub async fn get(&mut self, key: RedisValue) -> Option<RedisValue> {
        self.map.lock().unwrap().get(&key).map(|inner| inner.clone())
    }
}

fn parse_msg(buffer: BytesMut) -> Result<(RedisValue, usize)> {
    match buffer[0] as char {
        '+' => parse_simple_string(buffer),
        '$' => parse_bulk_string(buffer),
        '*' => parse_array(buffer),
        ':' => // parse_int(&buffer[1..]),
        unimplemented!(),
        _ => Err(anyhow::anyhow!("Not a valid RESP type: {:?}", buffer)),
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

    Ok((RedisValue::BulkString(String::from_utf8(buffer[bytes_consumed..end_of_bulk_str].to_vec())?), parsed))
}

fn parse_array(buffer: BytesMut) -> Result<(RedisValue, usize)> {
    let (array_length, mut bytes_consumed) = if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
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

    return Ok((RedisValue::Array(items), bytes_consumed))
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

fn _parse_int(buffer: &[u8]) -> Result<i64> {
    Ok(String::from_utf8(buffer.to_vec())?.parse::<i64>()?)
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
    Int(i64),
    // NullArray,
    NullBulkString,
    // ErrorMsg(Vec<u8>), // This is not a RESP type. This is an redis-oxide internal error type.
}


impl RedisValue {
    pub fn serialize(self) -> String {
        match self {
            RedisValue::SimpleString(s) => format!("+{}\r\n", s),
            RedisValue::BulkString(s) => format!("${}\r\n{}\r\n",s.chars().count(), s),
            RedisValue::NullBulkString => "$-1\r\n".to_string(),
            _ => unimplemented!(), // RedisValue::Array(v) =>
        }
    }
}
