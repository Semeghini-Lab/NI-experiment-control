//! Provides definitions and implementations for instruction-related functionalities.
//!
//! ## Main Structures and Enumerations:
//!
//! - `InstrType`: An enumeration that defines the types of instructions supported, including `CONST` for constant values and `SINE` for sinusoidal waves.
//!
//! - `Instruction`: Represents a general instruction composed of a type (`InstrType`) and a set of arguments (`InstrArgs`). It offers methods for creating specific instruction types conveniently and for evaluating them.
//!
//! - `InstrBook`: Manages an instruction along with its associated metadata during the experiment editing phase, capturing details like the defined interval and whether to retain a value after the defined interval.
//!
//! ## Utilities:
//!
//! - The `InstrArgs` type alias provides a convenient way to define instruction arguments using a dictionary with string keys and float values.
//!
//! - The module makes use of the `maplit` crate to enable easy creation of hashmaps.
//!
//! ## Features:
//!
//! - Easy creation of instruction objects with utility methods such as `new_const` and `new_sine`.
//! - Ability to evaluate instructions and in-place populate given time array views with the resulting float-point values.
//! - Support for default values in instructions, allowing for flexibility and ease of use.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt;

use maplit::hashmap;

/// Type alias for instruction arguments: a dictionary with key-value pairs of
/// string (argument name) and float (value)
pub type InstrArgs = HashMap<String, f64>;

/// Enum type for different instructions. Supported instructions: `CONST`, `SINE`
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

// / This function uses [`other_function`] to ...
// /
// / [`other_function`]: ./path/to/other/function

// Instruction struct consists of instr_type (enumerated type) and argument dictionary
/// Struct for a general instruction, consisting of type and arguments.
///
/// Different instruction types expects different fields in their argument dictionary.
/// Behavior for minimally expected keys are defined in `Instruction::new`, behavior of
/// default values are defined in `Instruction::eval_inplace`.
///
/// ## Implemented instruction types and their expected fields:
/// 1. `InstrType::CONST`:
///    - `const`
/// 2. `InstrType::SINE`:
///    - `freq`
///    - `amplitude`: Default is `1.0`
///    - `offset`: Default is `0.0`
///    - `phase`: Default is `0.0`
///
#[derive(Clone, PartialEq)]
pub struct Instruction {
    pub instr_type: InstrType,
    pub args: InstrArgs,
}
impl Instruction {
    /// Constructs an `Instruction` object.
    ///
    /// This method serves as the foundational constructor upon which custom constructor
    /// wrappers for new instructions should be built. For each instruction type,
    /// it ensures that the `args` dictionary contains the required keys.
    ///
    /// Missing keys will cause a panic.
    ///
    /// # Examples
    ///
    /// Constructing a new `CONST` instruction
    /// (this is effectively the underlying implementation for [`Instruction::new_const`],
    /// the more convenient constructor):
    ///
    /// ```
    /// use nicompiler_backend::instruction::*;
    /// use std::collections::HashMap;
    ///
    /// let mut const_args = InstrArgs::new();
    /// const_args.insert("value".to_string(), 1.0);
    /// let const_instr = Instruction::new(InstrType::CONST, const_args);
    /// ```
    ///
    /// If you fail to provide the required argument fields, it will panic:
    ///
    /// ```should_panic
    /// # use nicompiler_backend::instruction::*;
    /// # use std::collections::HashMap;
    /// let mut const_args = InstrArgs::new();
    /// let const_instr = Instruction::new(InstrType::CONST, const_args);
    /// ```
    ///
    /// The panic message will be:
    /// `thread 'main' panicked at 'Expected instr type CONST to contain key value'`.
    ///
    /// Constructing a new `SINE` instruction:
    ///
    /// ```
    /// # use nicompiler_backend::instruction::*;
    /// # use std::collections::HashMap;

    /// let mut sine_args = InstrArgs::new();
    /// sine_args.insert("freq".to_string(), 10.0);
    /// sine_args.insert("offset".to_string(), 1.0); // amplitude and phase will use default values
    /// let sine_instr = Instruction::new(InstrType::SINE, sine_args);
    /// ```
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

    /// Evaluates the instruction and populates the given array view with float-point values.
    ///
    /// This method takes a mutable array view (`t_arr`) and modifies its values in-place based on the instruction type and its arguments.
    ///
    /// - For `InstrType::CONST`, the array will be filled with the constant value specified by the `value` argument.
    /// - For `InstrType::SINE`, a sinusoidal waveform is generated using the arguments `freq`, `amplitude`, `offset`, and `phase`. Default values are used if certain arguments are not provided.
    ///
    /// # Arguments
    ///
    /// * `t_arr` - A mutable 1D array view that will be populated with the evaluated values.
    ///
    /// # Examples
    ///
    /// Given an instruction set with a constant and a sine instruction, and an array representing time values from 0 to 1:
    ///
    /// ```
    /// use ndarray::{Array2, Array1};
    /// use nicompiler_backend::instruction::*;
    ///
    /// let t_row = ndarray::Array1::linspace(0.0, 1.0, 10);
    /// let mut t_values = ndarray::stack(ndarray::Axis(0), &[t_row.view(), t_row.view()]).unwrap();
    /// // Use wrappers to create sine and constant instructions same as the examples above
    /// let const_instr = Instruction::new_const(1.0);
    /// const_instr.eval_inplace(&mut t_values.row_mut(0));
    ///
    /// let sine_instr = Instruction::new_sine(10.0, None, None, Some(1.0));
    /// sine_instr.eval_inplace(&mut t_values.row_mut(1));
    /// assert!(t_values[[0, 0]] == 1. && t_values[[0, 1]] == 1.);
    /// ```
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

    /// Wrapper for conveniently creating new constant instructions.
    /// Example usage equivalent to the constant example above:
    /// ```
    /// # use nicompiler_backend::instruction::*;
    /// let const_instr = Instruction::new_const(1.0);
    /// ```
    pub fn new_const(value: f64) -> Instruction {
        Instruction::new(InstrType::CONST, hashmap! {String::from("value") => value})
    }

    /// Constructs a new sine instruction with provided parameters.
    ///
    /// Allows for convenient creation of sine instructions by specifying the frequency and optionally, amplitude, phase, and DC offset. Unspecified parameters will not be included in the instruction's argument dictionary, allowing for default values to be used elsewhere if necessary.
    ///
    /// # Arguments
    ///
    /// - `freq`: The frequency of the sine wave.
    /// - `amplitude`: Optional amplitude of the sine wave. If `None`, it will not be set in the instruction.
    /// - `phase`: Optional phase offset of the sine wave in radians. If `None`, it will not be set in the instruction.
    /// - `dc_offset`: Optional DC offset for the sine wave. If `None`, it will not be set in the instruction.
    ///
    /// # Examples
    ///
    /// Constructing a sine instruction with a specified frequency, and DC offset. Amplitude and phase will use any default values defined elsewhere:
    ///
    /// ---
    /// # use nicompiler_backend::instruction::*;
    ///
    /// let sine_instr = Instruction::new_sine(10.0, None, None, Some(1.0));
    /// ---
    ///
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
            ("offset", dc_offset),
        ]
        .iter()
        .for_each(|(key, opt_value)| {
            if let Some(value) = *opt_value {
                instr_args.insert(key.to_string(), value);
            }
        });
        Instruction::new(InstrType::SINE, instr_args)
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

/// Manages an instruction along with its associated metadata during experiment editing.
///
/// The `InstrBook` struct captures the following metadata:
/// - Defined interval using `start_pos` and `end_pos`.
/// - A flag, `keep_val`, to determine whether to retain the value after the defined interval.
///
/// For the instruction interval:
/// - `start_pos` is inclusive.
/// - `end_pos` is exclusive.
///
/// `InstrBook` implements ordering based on `start_pos` to facilitate sorting.
/// editing phase: defined interval, and whether to keep value after defined interval.
/// For instruction interval, `start_pos` is inclusive while `end_pos` is exclusive.
/// We implemented ordering for `InstrBook` to allow sorting based on `start_pos`.
///
pub struct InstrBook {
    pub start_pos: usize,
    pub end_pos: usize,
    pub keep_val: bool,
    pub instr: Instruction,
}
impl InstrBook {
    /// Constructs a new `InstrBook` object.
    ///
    /// Checks that `end_pos` is strictly greater than `start_pos`.
    ///
    /// # Arguments
    ///
    /// - `start_pos`: Starting position (inclusive).
    /// - `end_pos`: Ending position (exclusive).
    /// - `keep_val`: Flag to determine if value should be retained after the interval.
    /// - `instr`: The associated instruction.
    ///
    /// # Examples
    ///
    /// Constructing a valid `InstrBook`:
    ///
    /// ```
    /// # use nicompiler_backend::instruction::*;
    /// let instruction = Instruction::new(InstrType::CONST, [("value".to_string(), 1.0)].iter().cloned().collect());
    /// let book = InstrBook::new(0, 5, true, instruction);
    /// ```
    ///
    /// Attempting to construct an `InstrBook` with `end_pos` not greater than `start_pos` will panic:
    ///
    /// ```should_panic
    /// # use nicompiler_backend::instruction::*;
    /// let instruction = Instruction::new(InstrType::CONST, [("value".to_string(), 1.0)].iter().cloned().collect());
    /// let book = InstrBook::new(5, 5, true, instruction);
    /// ```
    ///
    /// The panic message will be:
    /// `Instruction { /* ... */ } end_pos 5 should be strictly greater than start_pos 5`.
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
