//! Struct and methods corresponding to NI-hardware channels. See [`BaseChannel`] for
//! implementation details.
//!
//! Channels constitute the fundamental unit of interaction with NI devices, and between NI
//! devices and controlled hardware. A `Channel` instance, trivially implementing the [`BaseChannel`]
//! trait, corresponds to a physical channel on a NI device and, by extension,
//! a controllable physical quantity (e.g. laser on/off, coil current).
//!
//! ## Editing behavior
//! During editing, the user effectively adds [`InstrBook`] instances (instructions with associated
//! intervals) into the `instr_list` field through wrapper methods.
//! The `instr_list` field functions as an edit cache and  maintains a sorted list of newly added instruction books.
//!
//! ## Compilation behavior
//! Compilation is analogous to "flushing" the edit cache of an experiment.
//! During compilation, instructions within the edit cache via `instr_list` — which could
//! be disjointed — are expanded according to their `keep_val` property and combined to
//! produce a continuous stream of [`Instruction`], which is stored in `instr_end` and `instr_val`.
//!
//! Properties of a channel include:
//! - `samp_rate`: The sampling rate at which the parent device operates.
//! - `name`: Denotes the channel's identifier as seen by the NI driver. For instance,
//!    this could be 'ao0' or 'port0/line0'. This name can be viewed using tools like NI-MAX on
//!    Windows or the NI hardware configuration utilities on Linux.
//!  - `instr_list`: An edit-cache for the channel. Internally, this uses a `BTreeSet` to guarantee
//!    the sorted ordering of non-overlapping instruction intervals.
//!  - `task_type`: Specifies the task type associated with the channel. This affects the behavior
//!    of certain methods within the channel.
//!  - `fresh_compiled`: An internal boolean value that indicates whether the compiled results
//!    (stored in `instr_end` and `instr_val`) are up-to-date with the content of the edit cache.
//!
//! ## Channel property: "editable" and "streamable"
//!
//! For AO (Analog Output) channels, each edited channel corresponds directly to a NI-DAQmx channel.
//! However, the situation becomes nuanced when we consider DO (Digital Output) channels.
//! In DAQmx, digital channels can be of type "line" or "port".
//!
//! - Learn more about [lines and ports](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/mxcncpts/linesports.html).
//! - Dive deeper into their [corresponding data organization](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/mxcncpts/dataformats.html).
//!
//! A single port can encompass anywhere from 8 to 32 lines.
//! Importantly, each of these lines can produce an arbitrary output.
//! In this library, the unit of independent digital triggers, which users interact with,
//! corresponds to DAQmx "lines". These lines accept boolean values for individual writes.
//!
//! However, DAQmx offers a more efficient mechanism: writing integers to "ports".
//! In this method, each significant binary bit in the sequence corresponds to a line's output.
//! This port-based approach provides a substantial efficiency gain, making it indispensable for
//! successful digital output streaming.
//!
//! As a result, while library users interact with "line channels" (with names in the format like
//! `"port0/line0"`), the library internally aggregates lines from the same port during compilation.
//! This aggregation merges their instructions for streamlined execution.
//!
//! For instance, if `line0/port0` is high between `t=1~3` and `line0/port4` is high between `t=2~4`,
//! the parent device compilation will produce an auxiliary port channel named `port0`.
//!  This channel has compiled instructions as follows:
//! `(0, t=0~1), (1, t=1~2), (17, t=2~3), (16, t=3~4), (0, t=4~5)`.
//!
//! Channels generated in this manner are labeled as `streamable`, meaning directly used during experiment
//! streaming to generate driver-write signals. Channels which users directly interact with are labeled as `editable`.
//!
//! AO channels are both streamable and editable. DO line channels are editable but not streamable, and DO port
//! channels are non-editable yet streamable.

use ndarray::{array, s, Array1};
use std::collections::BTreeSet;

use crate::instruction::*;

/// Enum type for NI tasks. Channels are associated
/// with a unique task type, which affects their behavior.
/// Currently supported types: `AO` (analogue output), `DO` (digital output)
#[derive(PartialEq, Clone, Copy)]
pub enum TaskType {
    AO,
    DO,
}

/// The [`BaseChannel`] trait defines the core methods required for a channel's interaction with
/// NI devices. It encapsulates both editing and compilation behaviors of a channel.
///
/// Implementing this trait allows a type to represent a channel on a NI device, providing methods
/// to access and modify essential properties such as the sampling rate, physical name, and type of task.
/// Additionally, it provides methods to access and edit the underlying instruction list and compiled
/// instructions, enabling the creation, modification, and execution of tasks on the hardware.
///
/// # Required Methods
///
/// Implementors of this trait must provide implementations for a set of methods that allow:
/// - Accessing immutable properties of the channel.
/// - Mutating certain properties and states of the channel.
///
/// This trait ensures that any type representing a channel offers the necessary functionality
/// to interact with NI devices, ensuring consistency and safety in channel operations.
pub trait BaseChannel {
    // Immutable field methods
    fn samp_rate(&self) -> f64;
    fn name(&self) -> &str;
    fn task_type(&self) -> TaskType;
    /// The `fresh_compiled` field is set to true by each [`BaseChannel::compile`] call and
    /// `false` by each [`BaseChannel::add_instr`].  
    fn is_fresh_compiled(&self) -> bool;
    /// Provies a reference to the edit cache of instrbook list.
    fn instr_list(&self) -> &BTreeSet<InstrBook>;
    /// Returns the ending points of compiled instructions.
    fn instr_end(&self) -> &Vec<usize>;
    /// Retrieves the values of compiled instructions.
    fn instr_val(&self) -> &Vec<Instruction>;
    // Mutable field methods
    /// Mutable access to the `fresh_compiled` status.
    fn fresh_compiled_(&mut self) -> &mut bool;
    /// Mutable access to the instruction list.
    fn instr_list_(&mut self) -> &mut BTreeSet<InstrBook>;
    /// Mutable access to the ending points of compiled instructions.
    fn instr_end_(&mut self) -> &mut Vec<usize>;
    /// Mutable access to the values of compiled instructions.
    fn instr_val_(&mut self) -> &mut Vec<Instruction>;

    /// Channel is marked as compiled if its compilation-data field `instr_end` is nonempty
    fn is_compiled(&self) -> bool {
        !self.instr_end().is_empty()
    }
    /// Channel is marked as edited if its edit-cache field `instr_list` is nonempty
    fn is_edited(&self) -> bool {
        !self.instr_list().is_empty()
    }
    /// Channel is marked as editable if it is a AO channel or DO line channel (name contains "line")
    fn editable(&self) -> bool {
        match self.task_type() {
            TaskType::AO => true,
            TaskType::DO => self.name().contains("line"),
        }
    }
    /// Channel is marked as streamable if it is a AO channel or DO port channel (name does not contain "line")
    fn streamable(&self) -> bool {
        match self.task_type() {
            TaskType::AO => true,
            // for DODevice, only port channels are streamable
            TaskType::DO => !self.name().contains("line"),
        }
    }

    /// Compiles the instructions in the channel up to the specified `stop_pos`.
    ///
    /// The `compile` method processes the instruction list (`instr_list`) to generate a compiled
    /// list of end positions (`instr_end`) and corresponding values (`instr_val`). During compilation,
    /// it ensures that instructions are contiguous, adding padding as necessary. If two consecutive
    /// instructions have the same value, they are merged into a single instruction. 
    /// Unspecified intervals default to zero value. 
    ///
    /// # Arguments
    ///
    /// * `stop_pos`: The position up to which the instructions should be compiled. This is used
    /// to determine if padding is required at the end of the compiled instruction list.
    ///
    /// # Panics
    ///
    /// This method will panic in the following scenarios:
    /// * If the last instruction's end position in the `instr_list` exceeds the specified `stop_pos`.
    /// * If the channel is being recompiled but the previously compiled end position matches `stop_pos`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use nicompiler_backend::channel::*;
    /// # use nicompiler_backend::instruction::*;
    /// let mut channel = Channel::new(TaskType::DO, "port0/line0", 1e7);
    ///
    /// // Add some instructions to the channel.
    /// channel.add_instr(Instruction::new_const(1.), 0., 1., false);
    /// channel.add_instr(Instruction::new_const(0.), 1., 1., false);
    ///
    /// // Compile the instructions up to a specified stop position.
    /// channel.compile(3e7 as usize); // Compile up to 3 seconds (given a sampling rate of 10^7)
    /// ```
    fn compile(&mut self, stop_pos: usize) {
        if self.instr_list().len() == 0 {
            return;
        }
        // Ignore double compiles
        if self.is_fresh_compiled() && *self.instr_end().last().unwrap() == stop_pos {
            return;
        }
        self.clear_compile_cache();
        *self.fresh_compiled_() = true;

        if self.instr_list().last().unwrap().end_pos > stop_pos {
            panic!(
                "Attempting to compile channel {} with stop_pos {} while instructions end at {}",
                self.name(),
                stop_pos,
                self.instr_list().last().unwrap().end_pos
            );
        }

        let mut last_val = 0.;
        let mut last_end = 0;
        let mut instr_val: Vec<Instruction> = Vec::new();
        let mut instr_end: Vec<usize> = Vec::new();

        // Padding, instructions are already sorted
        let samp_rate = self.samp_rate();
        for instr_book in self.instr_list().iter() {
            if last_end != instr_book.start_pos {
                // Add padding instruction
                instr_val.push(Instruction::new_const(last_val));
                instr_end.push(instr_book.start_pos);
            }
            // Add original instruction
            instr_val.push(instr_book.instr.clone());
            instr_end.push(instr_book.end_pos);

            if instr_book.keep_val {
                last_val = match instr_book.instr.instr_type {
                    // Constant instruction: just retrieve its value for future padding
                    InstrType::CONST => *instr_book.instr.args.get("value").unwrap(),
                    // Other instructions: simulate end_pos
                    _ => {
                        let t_end = (instr_book.end_pos as f64) / samp_rate;
                        let mut t_arr = array![t_end];
                        instr_book.instr.eval_inplace(&mut t_arr.view_mut());
                        t_arr[0]
                    }
                };
            } else {
                last_val = 0.;
            }
            last_end = instr_book.end_pos;
        }
        // Pad the last instruction
        if self.instr_list().last().unwrap().end_pos != stop_pos {
            instr_val.push(Instruction::new_const(last_val));
            instr_end.push(stop_pos);
        }

        // Merge instructions, if possible
        for i in 0..instr_end.len() {
            if self.instr_val().is_empty() || instr_val[i] != *self.instr_val().last().unwrap() {
                self.instr_val_().push(instr_val[i].clone());
                self.instr_end_().push(instr_end[i]);
            } else {
                *self.instr_end_().last_mut().unwrap() = instr_end[i];
            }
        }
    }

    /// Utility function for signal sampling.
    ///
    /// Assuming a compiled channel (does not check), this utility function uses a binary search
    /// to efficiently determine the index of the first instruction whose end position is no less than
    /// the given `start_pos`.
    ///
    /// Note: This function assumes that `instr_end` is sorted in ascending order. It does not perform
    /// any checks for this condition.
    ///
    /// # Arguments
    ///
    /// * `start_pos` - The starting position for which to find the intersecting instruction.
    ///
    /// # Returns
    ///
    /// Returns the index `i` of the first instruction such that `self.instr_end[i] >= pos`
    /// If no such instruction is found, the function returns an index pointing to where
    /// the `pos` would be inserted to maintain the sorted order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use nicompiler_backend::channel::*;
    /// let mut channel = Channel::new(TaskType::DO, "port0/line0", 1e7);
    /// channel.instr_end_().extend([10, 20, 30, 40, 50].iter());
    ///
    /// assert_eq!(channel.binfind_first_intersect_instr(15), 1);
    /// assert_eq!(channel.binfind_first_intersect_instr(20), 1);
    /// assert_eq!(channel.binfind_first_intersect_instr(25), 2);
    /// assert_eq!(channel.binfind_first_intersect_instr(55), 5);
    /// assert_eq!(channel.binfind_first_intersect_instr(5), 0);
    /// ```
    fn binfind_first_intersect_instr(&self, start_pos: usize) -> usize {
        let mut low: i32 = 0;
        let mut high: i32 = self.instr_end().len() as i32 - 1;
        while low <= high {
            let mid = ((low + high) / 2) as usize;
            if self.instr_end()[mid] < start_pos {
                low = mid as i32 + 1;
            } else if self.instr_end()[mid] > start_pos {
                high = mid as i32 - 1;
            } else {
                return mid as usize;
            }
        }
        low as usize
    }

    /// Clears the `instr_list` field of the channel.
    ///
    /// If the compiled cache is empty, it also sets the `fresh_compiled` field to `true`.
    fn clear_edit_cache(&mut self) {
        *self.fresh_compiled_() = self.instr_end().len() == 0;
        self.instr_list_().clear();
    }

    /// Clears the compiled cache of the channel.
    ///
    /// Specifically, the method clears the `instr_end` and `instr_val` fields.
    /// If the edit cache is empty, it also sets the `fresh_compiled` field to `true`.
    fn clear_compile_cache(&mut self) {
        *self.fresh_compiled_() = self.instr_list().len() == 0;
        self.instr_end_().clear();
        self.instr_val_().clear();
    }

    /// Returns the stop time of the compiled instructions.
    ///
    /// If the channel is not compiled, it returns `0`. Otherwise, it retrieves the last end position
    /// from the compiled cache and converts it to a time value using the sampling rate.
    fn compiled_stop_time(&self) -> f64 {
        *self.instr_end().last().unwrap_or(&0) as f64 / self.samp_rate()
    }

    /// Returns the stop time of the edited instructions.
    ///
    /// Retrieves the last instruction from the edit cache and converts its end position
    /// to a time value using the sampling rate. If the edit cache is empty, it returns `0`.
    fn edit_stop_time(&self) -> f64 {
        self.instr_list()
            .last()
            .map_or(0., |instr| instr.end_pos as f64 / self.samp_rate())
    }

    fn add_instr_(&mut self, instr: Instruction, t: f64, duration: f64, keep_val: bool, had_conflict: bool) {
        let start_pos = (t * self.samp_rate()) as usize;
        let end_pos = ((t * self.samp_rate()) as usize) + ((duration * self.samp_rate()) as usize);
        let new_instrbook = InstrBook::new(start_pos, end_pos, keep_val, instr);
        // Upon adding an instruction, the channel is not freshly compiled anymore
        *self.fresh_compiled_() = false;

        // Check for overlaps
        let name = self.name();
        let delta = (1e-3 * self.samp_rate()) as usize; // Accomodate shift up to 1ms
        if let Some(next) = self.instr_list().range(&new_instrbook..).next() {
            if next.start_pos < new_instrbook.end_pos {
                // Accomodate tick conflicts less than delta on the right
                if !had_conflict && next.start_pos + delta >= new_instrbook.end_pos {
                    let conflict_ticks = new_instrbook.end_pos - start_pos;
                    println!("Conflict ticks {}", conflict_ticks);
                    assert!(conflict_ticks != 0, "unintended behavior");
                    self.add_instr_(new_instrbook.instr, t - ((conflict_ticks + 1) as f64) / self.samp_rate(), duration, keep_val, true);
                    return;
                } else {
                    panic!(
                    "Channel {}\n Instruction {} overlaps with the next instruction {}. Had conflict: {}; next_start: {}; new_end {}; attempted new_end: {}\n",
                    name, new_instrbook, next, had_conflict, next.start_pos, new_instrbook.end_pos, new_instrbook.end_pos - delta);
                }
            }
        } 
        if let Some(prev) = self.instr_list().range(..&new_instrbook).next_back() {
            if prev.end_pos > new_instrbook.start_pos {
                // Accomodate tick conflicts less than delta on the right
                if !had_conflict && new_instrbook.start_pos + delta >= prev.end_pos {
                    let conflict_ticks = prev.end_pos - new_instrbook.start_pos;
                    println!("Conflict ticks {}", conflict_ticks);
                    assert!(conflict_ticks != 0, "unintended behavior");
                    self.add_instr_(new_instrbook.instr, t + ((conflict_ticks + 1) as f64) / self.samp_rate(), duration, keep_val, true);
                    return;
                } else {
                    panic!(
                    "Channel {}\n Instruction {} overlaps with the previous instruction {}. Had conflict: {}; prev_end {}; new_start {}; attempted new_start: {}\n",
                    name, new_instrbook, prev, had_conflict, prev.end_pos, new_instrbook.start_pos, new_instrbook.start_pos + delta);
                }
            } 
        }
        self.instr_list_().insert(new_instrbook);
    }

    /// Adds an instruction to the channel.
    ///
    /// This is the primary method for adding instructions. It computes the discrete position
    /// interval associated with the given instruction, updates the `fresh_compiled` field,
    /// and inserts the instruction if it does not overlap with existing ones.
    /// See the helper [`BaseChannel::add_instr_`] for implementation details. 
    ///
    /// # Arguments
    ///
    /// * `instr`: The instruction to be added.
    /// * `t`: The start time for the instruction.
    /// * `duration`: The duration of the instruction.
    /// * `keep_val`: Boolean value indicating whether to keep the instruction value after its end time.
    ///
    /// # Panics
    ///
    /// This method will panic if the new instruction overlaps with any existing instruction.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::channel::*;
    /// # use nicompiler_backend::instruction::*;
    /// let mut channel = Channel::new(TaskType::DO, "port0/line0", 1e7);
    ///
    /// // Ask the DO channel to go high at t=1 for 0.5 seconds, then return to default value (0)
    /// channel.add_instr(Instruction::new_const(1.), 1., 0.5, false);
    ///
    /// // Asks the DO channel to go high at t=0.5 for 0.001 seconds and keep its value.
    /// // This will be merged with the instruction above during compilation.
    /// channel.add_instr(Instruction::new_const(1.), 0.5, 0.001, true);
    ///
    /// // The following line is effectively the same as the two lines above after compilation.
    /// // However, adding it immediately after the previous instructions will cause an overlap panic.
    /// // Uncommenting the line below will trigger the panic.
    /// // channel.add_instr(Instruction::new_const(1.), 0.5, 1., false);
    /// ```
    ///
    /// Expected failure:
    ///
    /// ```should_panic
    /// # use nicompiler_backend::channel::*;
    /// # use nicompiler_backend::instruction::*;
    /// let mut channel = Channel::new(TaskType::DO, "port0/line0", 1e7);
    /// channel.add_instr(Instruction::new_const(1.), 1., 0.5, false);
    /// channel.add_instr(Instruction::new_const(1.), 0.5, 0.001, true);
    /// channel.add_instr(Instruction::new_const(1.), 0.5, 1., false); // This will panic
    /// ```
    ///
    /// The panic message will be:
    /// ```text
    /// "Channel port0/line0
    ///  Instruction InstrBook([CONST, {value: 1}], 5000000-15000000, false) overlaps with the next instruction InstrBook([CONST, {value: 1}], 5000000-5010000, true)"
    /// ```
    fn add_instr(&mut self, instr: Instruction, t: f64, duration: f64, keep_val: bool) {
        self.add_instr_(instr, t, duration, keep_val, false);
    }

    /// Utility function to add a constant instruction to the channel
    fn constant(&mut self, value: f64, t: f64, duration: f64, keep_val: bool) {
        self.add_instr(Instruction::new_const(value), t, duration, keep_val);
    }

    /// Fills a buffer (1D view of array) with the signal samples derived from a channel's instructions.
    ///
    /// This method samples the float-point signal from channel's compile cache
    /// between the positions `start_pos` and `end_pos`, and replaces the contents of the buffer with results.
    /// The number of samples is given by `num_samps`. Time-dependent instructions assume that
    /// the buffer is already populated with correctly sampled time values.
    ///
    /// # Arguments
    ///
    /// * `start_pos` - The starting position in the channel's instructions to begin sampling.
    /// * `end_pos` - The ending position in the channel's instructions to stop sampling.
    /// * `num_samps` - The number of samples required.
    /// * `buffer` - A mutable reference to an `ndarray::ArrayViewMut1<f64>` that will hold the sampled signal values.
    ///
    /// # Panics
    ///
    /// * If the channel is not compiled.
    /// * If `end_pos` is not greater than `start_pos`.
    /// * If `end_pos` exceeds the duration of the channel's compiled instructions.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::channel::*;
    /// # use nicompiler_backend::instruction::*;
    /// let mut channel = Channel::new(TaskType::AO, "ao0", 1e6);
    /// // Sample 100 samples from t=0 to t=10s
    /// let (start_pos, end_pos, num_samps) = (0, 1e7 as usize, 100);
    ///
    /// // Add an sine instruction sig=sin(2*pi*t*7.5) + 1 from t=0.5~9.5 which keeps its value
    /// let sine_instr = Instruction::new_sine(7.5, None, None, Some(1.0));
    /// channel.add_instr(sine_instr, 0.5, 9., true);
    /// channel.compile(1e7 as usize); // Compile the channel to stop at 10s (1e7 samples)
    ///
    /// let mut buffer = ndarray::Array1::<f64>::zeros(num_samps);
    /// channel.fill_signal_nsamps(start_pos, end_pos, num_samps, &mut buffer.view_mut());
    ///
    /// assert_eq!(buffer[0], 0.);
    /// assert_eq!(buffer[99], 2.);
    /// ```
    ///
    /// # Notes
    ///
    /// The method uses binary search to find the starting and ending instruction indices that intersect
    /// with the provided interval `[start_pos, end_pos]`. It then iterates over these instructions
    /// to sample the signal and populate the buffer. Time conversion is done internally to map
    /// between the position indices and the buffer's time values.
    fn fill_signal_nsamps(
        &self,
        start_pos: usize,
        end_pos: usize,
        num_samps: usize,
        buffer: &mut ndarray::ArrayViewMut1<f64>,
    ) {
        assert!(
            self.is_compiled(),
            "Attempting to calculate signal on not-compiled channel {}",
            self.name()
        );
        assert!(
            end_pos > start_pos,
            "Channel {} attempting to calculate signal for invalid interval {}-{}",
            self.name(),
            start_pos,
            end_pos
        );
        assert!(
            end_pos <= (self.compiled_stop_time() * self.samp_rate()) as usize,
            "Attempting to calculate signal interval {}-{} for channel {}, which ends at {}",
            start_pos,
            end_pos,
            self.name(),
            (self.compiled_stop_time() * self.samp_rate()) as usize
        );

        let start_instr_idx: usize = self.binfind_first_intersect_instr(start_pos);
        let end_instr_idx: usize = self.binfind_first_intersect_instr(end_pos);
        // Function for converting position idx (unit of start_pos, end_pos) to buffer offset
        // Linear function: start_pos |-> 0, end_pos |-> num_samps
        let cvt_idx = |pos| {
            ((pos - start_pos) as f64 / (end_pos - start_pos) as f64 * (num_samps as f64)) as usize
        };

        let mut cur_pos: usize = start_pos as usize;
        for i in start_instr_idx..=end_instr_idx {
            let instr_signal_length = std::cmp::min(end_pos, self.instr_end()[i]) - cur_pos;
            let slice =
                &mut buffer.slice_mut(s![cvt_idx(cur_pos)..cvt_idx(cur_pos + instr_signal_length)]);
            self.instr_val()[i].eval_inplace(slice);
            cur_pos += instr_signal_length;
        }
    }

    /// Calls `fill_signal_nsamps` with the appropriate buffer and returns signal vector.
    /// The in-place version `fill_signal_nsamps` is preferred to this method for efficiency.
    /// This is mainly a wrapper to expose channel-signal sampling to Python
    fn calc_signal_nsamps(&self, start_time: f64, end_time: f64, num_samps: usize) -> Vec<f64> {
        let mut buffer = Array1::linspace(start_time, end_time, num_samps);
        let start_pos = (start_time * self.samp_rate()) as usize;
        let end_pos = (end_time * self.samp_rate()) as usize;
        self.fill_signal_nsamps(start_pos, end_pos, num_samps, &mut buffer.view_mut());
        buffer.to_vec()
    }
}

/// Represents a physical channel on an NI device.
///
/// `Channel` provides a concrete implementation of the [`BaseChannel`] trait, offering
/// straightforward and direct methods to interact with the NI device channels. Each instance of
/// `Channel` corresponds to a physical channel on an NI device, characterized by its `name`.
///
/// The `Channel` struct ensures that any interactions with the NI devices are consistent with the
/// requirements and behaviors defined by the [`BaseChannel`] trait.
///
/// # Fields
/// - `samp_rate`: The sampling rate of the channel, determining how often the channel updates.
/// - `task_type`: Specifies the type of task associated with this channel.
/// - `fresh_compiled`: A boolean indicating whether the channel's compiled results are up-to-date with the edit cache.
/// - `name`: A string representation of the channel's identifier as recognized by the NI driver.
/// - `instr_list`: The edit-cache for the channel. Maintains a sorted list of instruction books.
/// - `instr_end`: Stores the ending points of compiled instructions.
/// - `instr_val`: Holds the values of the compiled instructions.
pub struct Channel {
    samp_rate: f64,
    fresh_compiled: bool,
    task_type: TaskType,
    name: String,
    instr_list: BTreeSet<InstrBook>,
    instr_end: Vec<usize>,
    instr_val: Vec<Instruction>,
}

impl BaseChannel for Channel {
    fn samp_rate(&self) -> f64 {
        self.samp_rate
    }
    fn is_fresh_compiled(&self) -> bool {
        self.fresh_compiled
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn instr_list(&self) -> &BTreeSet<InstrBook> {
        &self.instr_list
    }
    fn instr_end(&self) -> &Vec<usize> {
        &self.instr_end
    }
    fn instr_val(&self) -> &Vec<Instruction> {
        &self.instr_val
    }
    fn instr_list_(&mut self) -> &mut BTreeSet<InstrBook> {
        &mut self.instr_list
    }
    fn instr_end_(&mut self) -> &mut Vec<usize> {
        &mut self.instr_end
    }
    fn instr_val_(&mut self) -> &mut Vec<Instruction> {
        &mut self.instr_val
    }
    fn fresh_compiled_(&mut self) -> &mut bool {
        &mut self.fresh_compiled
    }
    fn task_type(&self) -> TaskType {
        self.task_type
    }
}

impl Channel {
    /// Constructs a new `Channel` instance.
    ///
    /// Creates a new channel with the specified task type, physical name, and sampling rate.
    ///
    /// # Arguments
    /// * `task_type`: Specifies the type of task associated with this channel.
    ///    It can be either `AO` (analogue output) or `DO` (digital output).
    /// * `name`: The string representation of the channel's identifier as recognized by the NI driver.
    /// * `samp_rate`: The sampling rate for the channel, determining how often the channel updates.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `Channel` initialized with the provided arguments.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::channel::*;
    /// let do_channel = Channel::new(TaskType::DO, "port0/line0", 1e7);
    /// let ao_channel = Channel::new(TaskType::AO, "ao0", 1e6);
    /// ```
    ///
    pub fn new(task_type: TaskType, name: &str, samp_rate: f64) -> Self {
        Self {
            samp_rate,
            task_type,
            fresh_compiled: true,
            name: name.to_string(),
            instr_list: BTreeSet::new(),
            instr_end: Vec::new(),
            instr_val: Vec::new(),
        }
    }
}
