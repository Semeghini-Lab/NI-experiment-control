//! Provides a collection of utility functions and structures primarily focused on
//! time-tracking and extracting specific data from string patterns.
//!
//! Main features:
//! - [`TickTimer`]: A utility struct that allows easy time-tracking and measures
//! elapsed time in milliseconds since the UNIX epoch.
//!   It's particularly useful for performance measurement and debugging.
//! - [`extract_port_line_numbers`]: A function that extracts port and line numbers from a
//! specific string pattern (`port[number]/line[number]`).
//!   It's a quick way to parse such patterns when the exact format is known in advance.
//!
//! # Examples
//!
//! Using `TickTimer` for time-tracking:
//!
//! ```
//! # use nicompiler_backend::utils::TickTimer;
//! let mut timer = TickTimer::new(); // Starts the timer
//! // ... some operations ...
//! let elapsed = timer.tick(); // Measures time since last tick.
//! println!("Elapsed time since last tick: {}ms", elapsed);
//! ```
//!
//! Extracting port and line numbers from a string:
//!
//! ```
//! # use nicompiler_backend::utils::extract_port_line_numbers;
//! let input_str = "port0/line32";
//! let (port, line) = extract_port_line_numbers(input_str);
//! assert_eq!(port, 0);
//! assert_eq!(line, 32);
//! ```
//!
//! Refer to individual function and struct documentation for detailed usage and examples.

use std::time::{SystemTime, UNIX_EPOCH};

/// A utility class for time-tracking.
///
/// `TickTimer` provides a simple way to measure elapsed time in milliseconds since the UNIX epoch
/// and the time differences between successive calls.
///
/// # Example
///
/// ```
/// # use nicompiler_backend::utils::TickTimer;
/// let mut timer = TickTimer::new(); // Starts the timer
///
/// // Some code here...
///
/// let elapsed = timer.tick(); // Returns milliseconds since last tick.
/// println!("Elapsed time since last tick: {}ms", elapsed);
///
/// // Some more code here.
///
/// timer.tick_print("Time since last tick"); // Prints the elapsed time with a message, and returns duration
/// ```
pub struct TickTimer {
    pub milis: f64,
}

impl TickTimer {
    /// Constructs a new `TickTimer` and initializes it with the current time.
    ///
    /// # Returns
    ///
    /// Returns an instance of `TickTimer` with the current time in milliseconds since the UNIX epoch.
    pub fn new() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self {
            milis: duration.as_secs() as f64 * 1e3 + duration.subsec_nanos() as f64 / 1e6,
        }
    }

    /// Updates the timer and returns the time elapsed since the last tick in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a floating-point value representing the number of milliseconds elapsed since the last tick.
    pub fn tick(&mut self) -> f64 {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let milis = duration.as_secs() as f64 * 1e3 + duration.subsec_nanos() as f64 / 1e6;
        let diff = milis - self.milis;
        self.milis = milis;
        diff
    }

    /// Updates the timer, prints the elapsed time since the last tick with a provided message,
    /// and returns the elapsed time.
    ///
    /// # Arguments
    ///
    /// * `msg` - A string slice that holds a custom message to be printed alongside the elapsed time.
    ///
    /// # Returns
    ///
    /// Returns a floating-point value representing the number of milliseconds elapsed since the last tick.
    pub fn tick_print(&mut self, msg: &str) -> f64 {
        let diff = self.tick();
        println!("{}: {}", msg, diff);
        diff
    }
}

/// Extracts the port and line numbers from a given channel string.
///
/// This function assumes that the input string is of the form `port[number]/line[number]`
/// (e.g., `port0/line32`). It returns the port and line numbers as a tuple.
/// Note: The function does not perform any checks on the input format, so ensure that
/// the input string adheres to the expected format before calling this function.
///
/// # Arguments
///
/// * `chan` - A string slice representing the channel, expected to be in the format `port[number]/line[number]`.
///
/// # Returns
///
/// Returns a tuple containing the port and line numbers extracted from the input string.
///
/// # Examples
///
/// ```
/// # use nicompiler_backend::extract_port_line_numbers;
/// let input_str = "port0/line32";
/// let (port, line) = extract_port_line_numbers(input_str);
/// assert_eq!(port, 0);
/// assert_eq!(line, 32);
/// ```
pub fn extract_port_line_numbers(chan: &str) -> (usize, usize) {
    let parts: Vec<&str> = chan.split('/').collect();
    let port_part = parts[0];
    let port_str = &port_part[4..]; // Skip the "port" prefix
    let line_part = parts[1];
    let line_str = &line_part[4..];
    (
        port_str.parse::<usize>().unwrap(),
        line_str.parse::<usize>().unwrap(),
    )
}
