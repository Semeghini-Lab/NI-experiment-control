//! Implements struct and methods corresponding to NI devices. See [`BaseDevice`] for
//! implementation details.
//!
//! A NI control system consists of one or both of the components:
//! 1. Devices (cards) directly attached the computer via PCIe/USB.
//! 2. A PCIe link card connected to a PXIe chassis, which hosts multiple PXIe cards.
//!
//!
//! ## Device
//! In this library, every `Device` object corresponds to a particular task for
//! a physical device (e.g. analogue output for `PXI1Slot1`). A `Device` trivially implements the
//! [`BaseDevice`] trait by supplying field methods.
//!
//! The fields of [`Device]` keeps tracks of of the physical channels associated with the device
//! as well as device-wide data such as device name, trigger line, and synchronization behavior.
//!
//! The [`Device`] struct is the primary structure used to interact with NI hardware. It groups multiple
//! channels, each of which corresponds to a physical channel on an NI device. This struct provides
//! easy access to various properties of the device, such as its physical name, task type, and
//! several clock and trigger configurations.
//! For editing and compiling behavior of devices, see the [`BaseDevice`] trait.
//!
//! [`Device`] fields:
//! - `channels`: A collection of channels associated with this device.
//! - `physical_name`: Name of the device as seen by the NI driver.
//! - `task_type`: Specifies the task type associated with the device.
//! - `samp_rate`: The sampling rate of the device in Hz.
//! - `samp_clk_src`: Optional source of the sampling clock, supply `None` for on-board clock source.
//! - `trig_line`: Optional identifier for the port through which to import / export the task start trigger,
//! supply `None` for trivial triggering behavior
//! - `is_primary`: Optional Boolean indicating if the device is the primary device. Determines whether
//! to export (`true`) or (`import`) the start trigger of the NI-task associated with this device through `trig_line`.
//! In case that any device in an experiment has nontrivial triggering behavior, one and only one of the devices
//! must be primary.
//! - `ref_clk_src`: Optional source of the reference clock to phase-lock the device clock to. Supply `None` for
//! trivial reference clock behavior
//! - `ref_clk_rate`: Optional rate of the reference clock in Hz.
//!
//!
//! ### Editable and streamable channels in devices
//! Library users create and edit editable channels. During compilation, based on the device's task type,
//! the library may internally add streamable channels.
//! For more details on editable and streamable channels, see the editable v.s. streamable section in
//! [`channel` module].
//!
//! [`channel` module]: crate::channel

use ndarray::{s, Array1, Array2};
use regex::Regex;
use std::collections::{BTreeSet, HashMap};

use crate::channel::*;
use crate::instruction::*;
use crate::utils::*;

/// The `BaseDevice` trait defines the fundamental operations and attributes of a National Instruments (NI) device.
///
/// This trait abstracts the common functionalities that an NI device should possess, regardless of its specific hardware details or task type. Implementers of this trait will have access to core functionalities like channel management, device status checks, signal compilation, and more.
///
/// ## Typical Use
///
/// A type implementing `BaseDevice` is primarily used to interact with the associated NI hardware, manage its channels, and perform operations like signal generation, editing, and compilation.
///
/// # Trait Methods and Their Functionality:
///
/// - **Field methods**: These provide direct access to the properties of a device, such as its channels, physical name, 
/// sampling rate, and various configuration parameters.
///
/// - **Channel management**: Methods like [`BaseDevice::editable_channels`], [`BaseDevice::editable_channels_`], and 
/// [`BaseDevice::add_channel`] allow for the retrieval and manipulation of channels associated with the device.
///
/// - **Device status checks**: Methods like [`BaseDevice::is_compiled`], [`BaseDevice::is_edited`], and 
/// [`BaseDevice::is_fresh_compiled`] enable checking the compilation and editing status of the device's channels.
///
/// - **Cache operations**: The methods [`BaseDevice::clear_edit_cache`] and [`BaseDevice::clear_compile_cache`] are 
/// used to clear the edit and compile caches of the device's channels, respectively.
///
/// - **Compilation**: The [`BaseDevice::compile`] method takes care of the signal compilation process for the device's 
/// channels. For Digital Output (DO) channels, it provides additional functionality to merge line channels into port channels.
///
/// - **Signal generation**: The [`BaseDevice::fill_signal_nsamps`] and [`BaseDevice::calc_signal_nsamps`] methods are 
/// central to signal generation, allowing for the sampling of float-point values from compiled instructions based on 
/// various criteria.
///
/// - **Utility functions**: Methods like [`BaseDevice::unique_port_numbers`] offer utility functionalities specific to certain 
/// task types, aiding in operations like identifying unique ports in Digital Output (DO) devices.
///
/// # Implementing [`BaseDevice`]:
///
/// When creating a new type that represents an NI device, implementing this trait ensures that the type has all the necessary methods and behaviors typical of NI devices. Implementers can then extend or override these methods as necessary to provide device-specific behavior or optimizations.
pub trait BaseDevice {
    // Field methods
    fn channels(&self) -> &HashMap<String, Channel>;
    fn channels_(&mut self) -> &mut HashMap<String, Channel>;
    fn physical_name(&self) -> &str;
    fn task_type(&self) -> TaskType;
    fn samp_rate(&self) -> f64;
    fn samp_clk_src(&self) -> Option<&str>;
    fn trig_line(&self) -> Option<&str>;
    fn is_primary(&self) -> Option<bool>;
    fn ref_clk_src(&self) -> Option<&str>;
    fn ref_clk_rate(&self) -> Option<f64>;

    /// Returns a vector of references to editable channels
    fn editable_channels(&self) -> Vec<&Channel> {
        self.channels()
            .values()
            .filter(|&chan| chan.editable())
            .collect()
    }
    /// Returns a vector of mutable references to editable channels
    fn editable_channels_(&mut self) -> Vec<&mut Channel> {
        self.channels_()
            .values_mut()
            .filter(|chan| (*chan).editable())
            .collect()
    }

    /// Adds a new channel to the device.
    ///
    /// This base method validates the provided `physical_name` based on the device's `task_type`
    /// to ensure it adheres to the expected naming convention for the respective task type.
    ///
    /// # Naming Conventions:
    /// - For `TaskType::AO`: Channels should be named following the pattern "ao(number)"
    ///   (e.g., "ao0", "ao1").
    /// - For `TaskType::DO`: Channels should be named following the pattern "port(number)/line(number)"
    ///   (e.g., "port0/line1").
    ///
    /// # Panics
    /// - If the provided `physical_name` does not adhere to the expected naming convention for the
    ///   associated task type.
    /// - If a channel with the same `physical_name` already exists within the device.
    ///
    /// # Arguments
    /// - `physical_name`: Name of the channel as seen by the NI driver, which must adhere to the
    ///   naming conventions detailed above.
    fn add_channel(&mut self, physical_name: &str) {
        // Check the physical_name format
        let (name_match_string, name_format_description) = match self.task_type() {
            TaskType::AO => (String::from(r"^ao\d+$"), String::from("ao(number)")),
            TaskType::DO => (
                String::from(r"^port\d+/line\d+$"),
                String::from("port(number)/line(number)"),
            ),
        };

        let re = Regex::new(&name_match_string).unwrap();
        if !re.is_match(physical_name) {
            panic!(
                "Expecting channels to be of format '{}' yet received channel name {}",
                name_format_description, physical_name
            );
        }
        for channel in self.channels().values() {
            if channel.physical_name() == physical_name {
                panic!(
                    "Physical name of channel {} already registered. Registered channels are {:?}",
                    physical_name,
                    self.channels()
                        .values()
                        .map(|c| c.physical_name())
                        .collect::<Vec<_>>()
                );
            }
        }
        let new_channel = Channel::new(self.task_type(), physical_name, self.samp_rate());
        self.channels_()
            .insert(physical_name.to_string(), new_channel);
    }

    /// A device is compiled if any of its editable channels are compiled.
    fn is_compiled(&self) -> bool {
        self.editable_channels()
            .iter()
            .any(|channel| channel.is_compiled())
    }
    /// A device is marked edited if any of its editable channels are edited.
    fn is_edited(&self) -> bool {
        self.editable_channels()
            .iter()
            .any(|channel| channel.is_edited())
    }
    /// A device is marked fresh-compiled if all if its editable channels are freshly compiled.
    fn is_fresh_compiled(&self) -> bool {
        self.editable_channels()
            .iter()
            .all(|channel| channel.is_fresh_compiled())
    }
    /// Clears the edit-cache fields for all editable channels.
    fn clear_edit_cache(&mut self) {
        self.editable_channels_()
            .iter_mut()
            .for_each(|chan| chan.clear_edit_cache());
    }
    /// Clears the compile-cache fields for all editable channels.
    fn clear_compile_cache(&mut self) {
        self.editable_channels_()
            .iter_mut()
            .for_each(|chan| chan.clear_compile_cache());
    }

    /// Compiles all editable channels to produce a continuous instruction stream.
    ///
    /// The method starts by compiling each individual editable channel to obtain a continuous
    /// stream of instructions. If the device type is `TaskType::DO` (Digital Output), an additional
    /// processing step is performed. All the line channels belonging to the same port are merged
    /// into a single, streamable port channel that is non-editable. This aggregated port channel
    /// contains constant instructions whose integer values are determined by the combined state
    /// of all the lines of the corresponding port. Specifically, the `n`th bit of the integer
    /// value of the instruction corresponds to the boolean state of the `n`th line.
    ///
    /// # Port Channel Aggregation
    /// Each instruction inside the aggregated port channel is a constant instruction. The value of
    /// this instruction is an integer, where its `n`th bit represents the boolean state of the
    /// `n`th line. This way, the combined state of all lines in a port is efficiently represented
    /// by a single integer value, allowing for streamlined execution and efficient data transfer.
    ///
    /// # Arguments
    /// - `stop_pos`: The stop position used to compile the channels.
    fn compile(&mut self, stop_pos: usize) {
        self.editable_channels_()
            .iter_mut()
            .for_each(|chan| chan.compile(stop_pos));
        if self.task_type() != TaskType::DO {
            return;
        }
        // For DO channels: merge line channels into port channels
        for match_port in self.unique_port_numbers() {
            // Collect a sorted list of possible intervals
            let mut instr_end_set = BTreeSet::new();
            instr_end_set.extend(
                self.editable_channels()
                    .iter()
                    .filter(|chan| extract_port_line_numbers(chan.physical_name()).0 == match_port)
                    .flat_map(|chan| chan.instr_end().iter()),
            );
            let instr_end: Vec<usize> = instr_end_set.into_iter().collect();

            let mut instr_val = vec![0.; instr_end.len()];
            for chan in self.editable_channels() {
                let (port, line) = extract_port_line_numbers(chan.physical_name());
                if port == match_port {
                    let mut chan_instr_idx = 0;
                    for i in 0..instr_val.len() {
                        assert!(chan_instr_idx < chan.instr_end().len());
                        let chan_value =
                            chan.instr_val()[chan_instr_idx].args.get("value").unwrap();
                        instr_val[i] += *chan_value as f64 * 2.0f64.powf(line as f64);
                        if instr_end[i] == chan.instr_end()[chan_instr_idx] {
                            chan_instr_idx += 1;
                        }
                    }
                }
            }
            let port_instr_val: Vec<Instruction> = instr_val
                .iter()
                .map(|&val| Instruction::new_const(val))
                .collect();
            let mut port_channel = Channel::new(
                TaskType::DO,
                &format!("port{}", match_port),
                self.samp_rate(),
            );
            *port_channel.instr_val_() = port_instr_val;
            *port_channel.instr_end_() = instr_end;
            self.channels_()
                .insert(port_channel.physical_name().to_string(), port_channel);
        }
    }

    /// Returns a vector of compiled channels based on the given criteria.
    ///
    /// Filters the device's channels based on their compiled state and optional properties such as
    /// streamability and editability.
    ///
    /// # Arguments
    /// - `require_streamable`: If `true`, only compiled channels marked as streamable will be included in the result.
    /// - `require_editable`: If `true`, only compiled channels marked as editable will be included in the result.
    ///
    /// # Returns
    /// A `Vec` containing references to the channels that match the provided criteria.
    fn compiled_channels(&self, require_streamable: bool, require_editable: bool) -> Vec<&Channel> {
        self.channels()
            .values()
            .filter(|chan| {
                chan.is_compiled()
                    && (!require_streamable || chan.streamable())
                    && (!require_editable || chan.editable())
            })
            .collect()
    }

    /// Calculates the maximum stop time among all compiled channels.
    ///
    /// Iterates over all the compiled channels in the device, regardless of their streamability or
    /// editability, and determines the maximum stop time.
    ///
    /// # Returns
    /// A `f64` representing the maximum stop time (in seconds) across all compiled channels.
    fn compiled_stop_time(&self) -> f64 {
        self.compiled_channels(false, false)
            .iter()
            .map(|chan| chan.compiled_stop_time())
            .fold(0.0, f64::max)
    }

    /// Calculates the maximum stop time among all editable channels.
    ///
    /// Iterates over all the editable channels in the device and determines the maximum stop time.
    ///
    /// # Returns
    /// A `f64` representing the maximum stop time (in seconds) across all editable channels.
    fn edit_stop_time(&self) -> f64 {
        self.editable_channels()
            .iter()
            .map(|chan| chan.edit_stop_time())
            .fold(0.0, f64::max)
    }

    /// Generates a signal by sampling float-point values from compiled instructions.
    ///
    /// This method fills a given buffer with signal values based on the compiled instructions of the device's
    /// channels. Depending on the requirements, it can either generate signals intended for actual driver
    /// writing or for debugging editing intentions.
    ///
    /// # Arguments
    /// - `start_pos`: The starting position in the sequence of compiled instructions.
    /// - `end_pos`: The ending position in the sequence of compiled instructions.
    /// - `nsamps`: The number of samples to generate.
    /// - `buffer`: A mutable reference to a 2D array. The first axis corresponds to the channel index and
    ///    the second axis corresponds to the sample index.
    /// - `require_streamable`: If `true`, only signals from channels marked as streamable will be generated.
    /// - `require_editable`: If `true`, signals will be generated according to editing intentions for debugging purposes.
    ///
    /// # Panics
    /// This method will panic if:
    /// - The first dimension of the buffer does not match the number of channels that fulfill the provided requirements.
    /// - The second dimension of the buffer does not match the provided `nsamps` value.
    ///
    /// # TODO Notes
    /// The generation of signals from channels can be parallelized for performance improvements.
    fn fill_signal_nsamps(
        &self,
        start_pos: usize,
        end_pos: usize,
        nsamps: usize,
        buffer: &mut ndarray::Array2<f64>,
        require_streamable: bool,
        require_editable: bool,
    ) {
        // Assumes buffer of shape [num_compiled_and_streamable_channels][nsamps]
        assert!(
            buffer.dim().0
                == self
                    .compiled_channels(require_streamable, require_editable)
                    .len(),
            "Device {} has {} channels but passed buffer has shape {:?}",
            self.physical_name(),
            self.compiled_channels(require_streamable, require_editable)
                .len(),
            buffer.dim()
        );
        assert!(
            buffer.dim().1 == nsamps,
            "Simulating position {}-{} with {} elements, but buffer has shape {:?}",
            start_pos,
            end_pos,
            nsamps,
            buffer.dim()
        );
        // This can be parallelized (note)
        for (i, chan) in self
            .compiled_channels(require_streamable, require_editable)
            .iter()
            .enumerate()
        {
            let mut channel_slice = buffer.slice_mut(s![i, ..]);
            chan.fill_signal_nsamps(start_pos, end_pos, nsamps, &mut channel_slice);
        }
    }

    /// Computes and returns the signal values for specified channels in a device.
    ///
    /// This method calculates the signal values by sampling float-point values from compiled instructions
    /// of the device's channels. Depending on the requirements, the signal can be either intended for actual
    /// driver writing or for debugging editing intentions. For AO (Analog Output) devices, the returned buffer
    /// will contain time data.
    ///
    /// # Arguments
    /// - `start_pos`: The starting position in the sequence of compiled instructions.
    /// - `end_pos`: The ending position in the sequence of compiled instructions.
    /// - `nsamps`: The number of samples to generate.
    /// - `require_streamable`: If `true`, only signals from channels marked as streamable will be generated.
    /// - `require_editable`: If `true`, signals will be generated according to editing intentions for debugging purposes.
    ///
    /// # Returns
    /// A 2D array with the computed signal values. The first axis corresponds to the channel index and the
    /// second axis corresponds to the sample index.
    ///
    /// # Panics
    /// This method will panic if:
    /// - There are no channels that fulfill the provided requirements.
    /// - The device's task type is not AO (Analog Output) when initializing the buffer with time data.
    fn calc_signal_nsamps(
        &self,
        start_pos: usize,
        end_pos: usize,
        nsamps: usize,
        require_streamable: bool,
        require_editable: bool,
    ) -> Array2<f64> {
        let num_chans = self
            .compiled_channels(require_streamable, require_editable)
            .len();
        assert!(
            num_chans > 0,
            "There is no channel with streamable={}, editable={}",
            require_streamable,
            require_editable
        );
        let mut buffer = Array2::from_elem((num_chans, nsamps), 0.);
        // Only AOChannel needs to initialize buffer with time data
        if self.task_type() == TaskType::AO {
            let t_values = Array1::linspace(
                start_pos as f64 / self.samp_rate(),
                end_pos as f64 / self.samp_rate(),
                nsamps,
            );
            buffer
                .outer_iter_mut()
                .for_each(|mut row| row.assign(&t_values));
        }
        self.fill_signal_nsamps(
            start_pos,
            end_pos,
            nsamps,
            &mut buffer,
            require_streamable,
            require_editable,
        );
        buffer
    }

    /// Retrieves a list of unique port numbers from the device's channels.
    ///
    /// This utility function is primarily used with DO (Digital Output) devices to identify and operate
    /// on unique ports. It scans through the compiled channels of the device, filtering for those that are
    /// editable, and extracts the unique port numbers associated with them.
    ///
    /// # Returns
    /// A vector of unique port numbers identified in the device's channels.
    ///
    /// # Panics
    /// The method will panic if it's invoked on a device that is not of task type DO.
    fn unique_port_numbers(&self) -> Vec<usize> {
        assert!(
            self.task_type() == TaskType::DO,
            "unique ports should only be invoked for DOs, but {} is not",
            self.physical_name()
        );

        let mut port_numbers = BTreeSet::new();

        self.compiled_channels(false, true).iter().for_each(|chan| {
            // Capture the port
            let physical_name = &chan.physical_name();
            port_numbers.insert(extract_port_line_numbers(physical_name).0);
        });
        port_numbers.into_iter().collect()
    }
}

/// Represents a National Instruments (NI) device.
///
/// A `Device` is the primary structure used to interact with NI hardware. It groups multiple
/// channels, each of which corresponds to a physical channel on an NI device. This struct provides
/// easy access to various properties of the device, such as its physical name, task type, and
/// several clock and trigger configurations.
/// For editing and compiling behavior of devices, see the [`BaseDevice`] trait.
///
/// # Fields
/// - `channels`: A collection of channels associated with this device.
/// - `physical_name`: Name of the device as seen by the NI driver.
/// - `task_type`: Specifies the task type associated with the device.
/// - `samp_rate`: The sampling rate of the device in Hz.
/// - `samp_clk_src`: Optional source of the sampling clock, supply `None` for on-board clock source.
/// - `trig_line`: Optional identifier for the port through which to import / export the task start trigger,
/// supply `None` for trivial triggering behavior
/// - `is_primary`: Optional Boolean indicating if the device is the primary device. Determines whether
/// to export (`true`) or (`import`) the start trigger of the NI-task associated with this device through `trig_line`.
/// In case that any device in an experiment has nontrivial triggering behavior, one and only one of the devices
/// must be primary.
/// - `ref_clk_src`: Optional source of the reference clock to phase-lock the device clock to.
/// - `ref_clk_rate`: Optional rate of the reference clock in Hz.
pub struct Device {
    channels: HashMap<String, Channel>,

    physical_name: String,
    task_type: TaskType,
    samp_rate: f64,

    samp_clk_src: Option<String>,
    trig_line: Option<String>,
    is_primary: Option<bool>,
    ref_clk_src: Option<String>,
    ref_clk_rate: Option<f64>,
}

impl Device {
    /// Constructs a new `Device` instance.
    ///
    /// This constructor initializes a device with the provided parameters. The `channels` field
    /// is initialized as an empty collection.
    ///
    /// # Arguments
    /// - `physical_name`: Name of the device as seen by the NI driver.
    /// - `task_type`: The type of task associated with the device.
    /// - `samp_rate`: Desired sampling rate in Hz.
    /// - `samp_clk_src`: Optional source for the sampling clock.
    /// - `trig_line`: Optional identifier for the device's trigger line.
    /// - `is_primary`: Optional flag indicating if this is the primary device (imports or exports trigger line).
    /// - `ref_clk_src`: Optional source for the device's reference clock.
    /// - `ref_clk_rate`: Optional rate of the reference clock in Hz.
    ///
    /// # Returns
    /// A new instance of `Device` with the specified configurations.
    pub fn new(
        physical_name: &str,
        task_type: TaskType,
        samp_rate: f64,
        samp_clk_src: Option<&str>,
        trig_line: Option<&str>,
        is_primary: Option<bool>,
        ref_clk_src: Option<&str>,
        ref_clk_rate: Option<f64>,
    ) -> Self {
        Self {
            channels: HashMap::new(),

            physical_name: physical_name.to_string(),
            task_type: task_type,

            samp_rate: samp_rate,
            samp_clk_src: samp_clk_src.map(String::from),
            trig_line: trig_line.map(String::from),
            is_primary: is_primary,
            ref_clk_src: ref_clk_src.map(String::from),
            ref_clk_rate: ref_clk_rate,
        }
    }
}

impl BaseDevice for Device {
    fn channels(&self) -> &HashMap<String, Channel> {
        &self.channels
    }
    fn channels_(&mut self) -> &mut HashMap<String, Channel> {
        &mut self.channels
    }
    fn physical_name(&self) -> &str {
        &self.physical_name
    }
    fn task_type(&self) -> TaskType {
        self.task_type
    }
    fn samp_rate(&self) -> f64 {
        self.samp_rate
    }
    fn samp_clk_src(&self) -> Option<&str> {
        self.samp_clk_src.as_deref()
    }
    fn trig_line(&self) -> Option<&str> {
        self.trig_line.as_deref()
    }
    fn is_primary(&self) -> Option<bool> {
        self.is_primary
    }
    fn ref_clk_src(&self) -> Option<&str> {
        self.ref_clk_src.as_deref()
    }
    fn ref_clk_rate(&self) -> Option<f64> {
        self.ref_clk_rate
    }
}
