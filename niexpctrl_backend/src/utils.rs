use std::cmp::min;

/// Helps keep track of a position within a defined range. Used for repetitions during streaming.
pub struct StreamCounter {
    pos: usize,
    end_pos: usize,
    interval: usize,
}

/// `StreamCounter` is a utility struct that keeps track of a position within a defined range.
/// This position advances in intervals, and when it reaches the end of the range, it wraps around
/// to the beginning.
///
/// # Examples
///
/// ```ignore
/// use niexpctrl_backend::StreamCounter;
/// let mut counter = StreamCounter::new(10, 3);
///
/// assert_eq!(counter.tick_next(), (0, 3));
/// assert_eq!(counter.tick_next(), (3, 6));
/// assert_eq!(counter.tick_next(), (6, 9));
/// assert_eq!(counter.tick_next(), (9, 10));
/// assert_eq!(counter.tick_next(), (0, 3));
/// ```
impl StreamCounter {
    /// Creates a new `StreamCounter` with a specified end position and interval.
    ///
    /// # Parameters
    ///
    /// * `end_pos`: The end position of the range. This is exclusive.
    /// * `interval`: The interval at which the position advances.
    ///
    /// # Returns
    ///
    /// A new `StreamCounter`.
    pub fn new(end_pos: usize, interval: usize) -> Self {
        Self {
            pos: 0,
            end_pos,
            interval,
        }
    }

    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Gets the current position.
    ///
    /// # Returns
    ///
    /// The current position.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Calculates the next position based on the current position and the interval.
    /// If the calculated position exceeds or equals `end_pos`, it returns `end_pos`.
    ///
    /// # Returns
    ///
    /// The next position.
    pub fn next_pos(&mut self) -> usize {
        min(self.pos + self.interval, self.end_pos)
    }

    pub fn reached_end(&self) -> bool {
        self.pos == self.end_pos
    }

    /// Advances the position by the interval and returns a tuple of the current position and the next position.
    /// If the position reaches `end_pos`, it wraps around to the beginning.
    ///
    /// # Returns
    ///
    /// A tuple of the form `(current_position, next_position)`.
    pub fn tick_next(&mut self) -> Option<(usize, usize)> {
        if self.reached_end() {
            return None;
        }
        let result = Some((self.pos(), self.next_pos()));
        self.pos = self.next_pos();
        result
    }
}

use std::sync::{Condvar, Mutex};

/// `Semaphore` is a synchronization primitive that controls access to a shared resource
/// by multiple threads. It maintains a count, which is decremented by the `acquire` method
/// and incremented by the `release` method. When the count is 0, the `acquire` method will block
/// until another thread calls `release`.
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
