use std::cmp::min;

pub struct StreamCounter {
    pos: usize,
    end_pos: usize,
    interval: usize,
}

// Keeps track of marching interval at specified "interval".
// Upon reaching the end, starts from the beginning again
impl StreamCounter {
    pub fn new(end_pos: usize, interval: usize) -> Self {
        Self {
            pos: 0,
            end_pos,
            interval,
        }
    }

    pub fn next_pos(&mut self) -> usize {
        min(self.pos + self.interval, self.end_pos)
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn tick_next(&mut self) -> (usize, usize) {
        let result = (self.pos(), self.next_pos());
        self.pos = self.next_pos();
        if self.pos == self.end_pos {
            self.pos = 0
        }
        result
    }
}

use std::sync::{Condvar, Mutex};

pub struct Semaphore {
    count: Mutex<i32>,
    condition: Condvar,
}

impl Semaphore {
    pub fn new(init_count: i32) -> Self {
        Semaphore {
            count: Mutex::new(init_count),
            condition: Condvar::new(),
        }
    }

    pub fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count < 1 {
            count = self.condition.wait(count).unwrap();
        }
        *count -= 1;
    }

    pub fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.condition.notify_one();
    }
}
