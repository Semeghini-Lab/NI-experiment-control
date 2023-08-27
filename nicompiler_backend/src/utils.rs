// The "utils" module defines InstructionType enum (to be)
use std::cmp::Ordering;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use maplit::hashmap;

pub type InstrArgs = HashMap<String, f64>;

// Enumeration type for different instructions
// To be modified if adding new instructions
#[derive(Clone, PartialEq)]
pub enum InstrType {
    CONST,
    SINE,
}
impl fmt::Display for InstrType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InstrType::CONST => "CONST",
                InstrType::SINE => "SINE",
            }
        )
    }
}

// Instruction struct consists of instr_type (enumerated type) and argument dictionary
#[derive(Clone, PartialEq)]
pub struct Instruction {
    pub instr_type: InstrType,
    pub args: InstrArgs,
}
// Constructor: checks for declaring new instructions
impl Instruction {
    // Provides a uniform constructor for instructions
    // For new instructions, should check for required dictionary fields
    pub fn new(instr_type: InstrType, args: InstrArgs) -> Self {
        let panic_key = |key| {
            if !args.contains_key(key) {
                panic!("Expected instr type {} to contain key {}", instr_type, key)
            }
        };
        match instr_type {
            InstrType::CONST => panic_key("value"),
            InstrType::SINE => panic_key("freq"),
        };
        Instruction { instr_type, args }
    }

    pub fn new_const(value: f64) -> Instruction {
        Instruction::new(InstrType::CONST, hashmap! {String::from("value") => value})
    }

    pub fn new_sine(
        freq: f64,
        amplitude: Option<f64>,
        phase: Option<f64>,
        dc_offset: Option<f64>,
    ) -> Instruction {
        let mut instr_args: InstrArgs = hashmap! {"freq".to_string() => freq};
        // For each optional argument, if specified, insert into dictionary
        [
            ("amplitude", amplitude),
            ("phase", phase),
            ("dc_offset", dc_offset),
        ]
        .iter()
        .for_each(|(key, opt_value)| {
            if let Some(value) = *opt_value {
                instr_args.insert(key.to_string(), value);
            }
        });
        Instruction::new(InstrType::SINE, instr_args)
    }

    pub fn eval_inplace(&self, t_arr: &mut ndarray::ArrayViewMut1<f64>) {
        match self.instr_type {
            InstrType::CONST => {
                let value = *self.args.get("value").unwrap();
                t_arr.fill(value);
            }
            InstrType::SINE => {
                let freq = *self.args.get("freq").unwrap();
                // Default values can be set by default with unwrap_or
                let amplitude = *self.args.get("amplitude").unwrap_or(&1.0);
                let offset = *self.args.get("offset").unwrap_or(&0.0);
                let phase = *self.args.get("phase").unwrap_or(&0.0);

                t_arr.map_inplace(|t| {
                    *t = (2.0 * PI * freq * (*t) + phase).sin() * amplitude + offset
                });
            }
        }
    }
}
impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let args_string = self
            .args
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<String>>()
            .join(", ");
        write!(f, "[{}, {{{}}}]", self.instr_type, args_string)
    }
}

// Wrapper around instruction to include start_pos, end_pos, and keep_val
pub struct InstrBook {
    pub start_pos: usize,
    pub end_pos: usize,
    pub keep_val: bool,
    pub instr: Instruction,
}
impl InstrBook {
    pub fn new(start_pos: usize, end_pos: usize, keep_val: bool, instr: Instruction) -> Self {
        assert!(
            end_pos > start_pos,
            "Instruction {} end_pos {} should be strictly greater than start_pos {}",
            instr,
            end_pos,
            start_pos
        );
        InstrBook {
            start_pos,
            end_pos,
            keep_val,
            instr,
        }
    }
}
// Support total ordering for InstrBook
impl Ord for InstrBook {
    fn cmp(&self, other: &Self) -> Ordering {
        // We reverse the order to make BinaryHeap a min-heap based on start_pos
        self.start_pos.cmp(&other.start_pos)
    }
}
impl PartialOrd for InstrBook {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for InstrBook {
    fn eq(&self, other: &Self) -> bool {
        self.start_pos == other.start_pos
    }
}
impl fmt::Display for InstrBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InstrBook({}, {}-{}, {})",
            self.instr, self.start_pos, self.end_pos, self.keep_val
        )
    }
}
impl Eq for InstrBook {}

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
    let port_str = &port_part[4..];  // Skip the "port" prefix
    let line_part = parts[1];
    let line_str = &line_part[4..];
    (port_str.parse::<usize>().unwrap(), line_str.parse::<usize>().unwrap())
}