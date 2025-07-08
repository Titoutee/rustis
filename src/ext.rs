use crate::RedisInt;

pub trait RedisValueInner {}

impl RedisValueInner for &str {}
impl RedisValueInner for String {}
impl RedisValueInner for RedisInt {}
