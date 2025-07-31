use crate::resp::{RedisInt, RedisArray};

pub trait RedisValueInner {}

impl RedisValueInner for RedisInt {}
impl RedisValueInner for RedisArray {}
impl RedisValueInner for &str {}
impl RedisValueInner for String {}