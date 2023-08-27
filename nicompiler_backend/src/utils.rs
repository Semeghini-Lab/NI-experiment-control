//! This module contains definition for instructions: `InstrType`, `Instruction`, `Instrbook`,
//! timing utilities (`TickTimer`), and other utility functions.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// Utility class for time-tracking
pub struct TickTimer {
    pub milis: f64,
}

impl TickTimer {
    pub fn new() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self {
            milis: duration.as_secs() as f64 * 1e3 + duration.subsec_nanos() as f64 / 1e6,
        }
    }

    pub fn tick(&mut self) -> f64 {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let milis = duration.as_secs() as f64 * 1e3 + duration.subsec_nanos() as f64 / 1e6;
        let diff = milis - self.milis;
        self.milis = milis;
        diff
    }

    pub fn tick_print(&mut self, msg: &str) -> f64 {
        let diff = self.tick();
        println!("{}: {}", msg, diff);
        diff
    }
}

// Assumes that input string is of form port[number]/line[number] (e.g. port0/line32)
// Returns the port and line numbers. Does not perform checks!!
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
