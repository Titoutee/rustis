// Some utilities and implementing wide traits

use crate::resp::{RedisInt, RedisArray};

pub struct ShuffleCtr {
    vals: Vec<usize>,
    valid_n: usize,
}

// Render async?

impl ShuffleCtr {
    pub fn init(with_n: usize) -> Self {
        ShuffleCtr { vals: (0..with_n).collect(), valid_n: with_n}
    }

    pub fn get_new_id(&mut self) -> usize {
        self.valid_n -= 1;
        self.vals.pop().expect("Error: Rustis limited to 100 clients at the same time...")
    }

    pub fn release(&mut self, id: usize) {
        self.valid_n+=1;
        self.vals.push(id);
    }
}

pub trait RedisValueInner {}

impl RedisValueInner for RedisInt {}
impl RedisValueInner for RedisArray {}
impl RedisValueInner for &str {}
impl RedisValueInner for String {}



