// Some utilities and wide traits implementations

use crate::resp::{RedisArray, RedisInt};

pub struct StackCtr {
    vals: Vec<usize>,
    valid_n: usize,
}

impl StackCtr {
    pub fn init(with_n: usize) -> Self {
        StackCtr {
            vals: (0..with_n).rev().collect(),
            valid_n: with_n,
        }
    }

    pub fn get_new_id(&mut self) -> usize {
        self.valid_n -= 1;
        self.vals
            .pop()
            .expect("Error: Rustis limited to 100 clients at the same time...")
    }

    pub fn release(&mut self, id: usize) {
        self.valid_n += 1;
        self.vals.push(id);
    }
}

// Different ways for a non-headless server
pub enum Notify {
    Info,
    Recv,
    RecvRaw, // For raw RESP input
    Send,
    SendRaw, // For smthg...
}

pub trait RedisValueInner {}

impl RedisValueInner for RedisInt {}
impl RedisValueInner for RedisArray {}
impl RedisValueInner for &str {}
impl RedisValueInner for String {}

pub trait RedisValueInenerToStr {}

impl<T: ToString> RedisValueInenerToStr for T {}
