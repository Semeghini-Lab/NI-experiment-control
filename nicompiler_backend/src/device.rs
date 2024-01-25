//! Implements struct and methods corresponding to NI devices. See [`BaseDevice`] for
//! implementation details.
//!
//! A NI control system consists of one or both of the components:
//! 1. Devices (cards) directly attached the computer via PCIe/USB.
//! 2. A PCIe link card connected to a PXIe chassis, which hosts multiple PXIe cards.
//!
//! ## Device
//! In this library, every [`Device`] object corresponds to a particular task for
//! a physical device (e.g. analogue output for `PXI1Slot1`). A `Device` trivially implements the
//! [`BaseDevice`] trait by supplying field methods.
//!
//! [`Device`] fields keep tracks of of the physical channels associated with the device
//! as well as device-wide data such as device name, trigger line, and synchronization behavior.
//!
//! The [`Device`] struct is the primary structure used to interact with NI hardware. It groups multiple
//! channels, each of which corresponds to a physical channel on an NI device. This struct provides
//! easy access to various properties of the device, such as its physical name, task type, and
//! several clock and trigger configurations.
//! For editing and compiling behavior of devices, see the [`BaseDevice`] trait.
//!
//!
//! ### Editable and streamable channels in devices
//! Library users create and edit editable channels. During compilation, based on the device's task type,
//! the library may internally add streamable channels.
//! For more details on editable and streamable channels, see the editable v.s. streamable section in
//! [`channel` module].
//!
//! ### Synchronization methods for devices
//! Each device's synchronization behavior is specified by its constructor arguments.
//! Refer to the [`Device`] struct for a more detailed explanation.
//!
//! [`channel` module]: crate::channel

use ndarray::{s, Array1, Array2};
use regex::Regex;
use std::collections::BTreeSet;
use indexmap::IndexMap;

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
/// - **Synchronization configuration**: Customize the synchronization behavior of devices via [`BaseDevice::cfg_trig`],
/// [`BaseDevice::cfg_ref_clk`], [`BaseDevice::cfg_samp_clk_src`]. See [`Device`] for more details.
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
///
/// # Implementing [`BaseDevice`]:
///
/// When creating a new type that represents an NI device, implementing this trait ensures that the type has all the necessary methods and behaviors typical of NI devices. Implementers can then extend or override these methods as necessary to provide device-specific behavior or optimizations.
pub trait BaseDevice {
    // Immutable accessors (getters)
    fn channels(&self) -> &IndexMap<String, Channel>;
    fn name(&self) -> &str;
    fn task_type(&self) -> TaskType;
    fn samp_rate(&self) -> f64;
    fn samp_clk_src(&self) -> Option<&str>;
    fn trig_line(&self) -> Option<&str>;
    fn export_trig(&self) -> Option<bool>;
    fn ref_clk_line(&self) -> Option<&str>;
    fn export_ref_clk(&self) -> Option<bool>;
    fn ref_clk_rate(&self) -> Option<f64>;

    // Mutable accessors
    fn channels_(&mut self) -> &mut IndexMap<String, Channel>;
    fn samp_clk_src_(&mut self) -> &mut Option<String>;
    fn trig_line_(&mut self) -> &mut Option<String>;
    fn export_trig_(&mut self) -> &mut Option<bool>;
    fn ref_clk_line_(&mut self) -> &mut Option<String>;
    fn export_ref_clk_(&mut self) -> &mut Option<bool>;
    fn ref_clk_rate_(&mut self) -> &mut Option<f64>;

    /// Shortcut to borrow channel instance by name
    fn chan(&self, name: &str) -> &Channel {
        if !self.channels().contains_key(name) {
            panic!("Device {} does not have channel {}", self.name(), name)
        }
        self.channels().get(name).unwrap()
    }
    /// Shortcut to mutably borrow channel instance by name
    fn chan_(&mut self, name: &str) -> &mut Channel {
        if !self.channels().contains_key(name) {
            panic!("Device {} does not have channel {}", self.name(), name)
        }
        self.channels_().get_mut(name).unwrap()
    }

    /// Returns sample clock period calculated as `1.0 / self.samp_rate()`
    fn clock_period(&self) -> f64 {
        1.0 / self.samp_rate()
    }
    /// Configures the sample clock source for the device.
    ///
    /// This method sets the `samp_clk_src` field of the device to the provided source string.
    ///
    /// # Arguments
    ///
    /// * `src` - The name of the sample clock source.
    fn cfg_samp_clk_src(&mut self, src: &str) {
        *(self.samp_clk_src_()) = Some(src.to_string());
    }

    /// Configures the trigger settings for the device.
    ///
    /// Depending on the value of `export_trig`, this method either:
    ///
    /// * Exports the device task's start trigger to `trig_line` (if `export_trig` is `true`), or
    /// * Imports the device task's start trigger from `trig_line` (if `export_trig` is `false`).
    ///
    /// # Arguments
    ///
    /// * `trig_line` - The trigger line identifier.
    /// * `export_trig` - A boolean that determines whether to export or import the trigger.
    fn cfg_trig(&mut self, trig_line: &str, export_trig: bool) {
        *(self.trig_line_()) = Some(trig_line.to_string());
        *(self.export_trig_()) = Some(export_trig);
    }

    /// Configures the reference clock settings for the device.
    ///
    /// If `export_ref_clk` is set to `true`, this method:
    ///
    /// * Exports the device's 10MHz on-board reference clock to `ref_clk_line`,
    /// * Asserts that `ref_clk_rate` is set to 1e7 (10MHz).
    ///
    /// If `export_ref_clk` is set to `false`, this method:
    ///
    /// * Sets the device's reference clock to the designated line and rate provided by the arguments.
    ///
    /// # Arguments
    ///
    /// * `ref_clk_line` - The line or channel to import or export the device's reference clock.
    /// * `ref_clk_rate` - The rate of the reference clock in Hz.
    /// * `export_ref_clk` - A boolean that determines whether to export (if `true`) or import (if `false`) the reference clock.
    fn cfg_ref_clk(&mut self, ref_clk_line: &str, ref_clk_rate: f64, export_ref_clk: bool) {
        if export_ref_clk {
            assert_eq!(ref_clk_rate, 1e7,
                "Device {} needs to explicitly acknowledge exporting 10Mhz clk by setting ref_clk_rate=1e7",
                self.name());
        }
        *(self.ref_clk_line_()) = Some(ref_clk_line.to_string());
        *(self.ref_clk_rate_()) = Some(ref_clk_rate);
        *(self.export_ref_clk_()) = Some(export_ref_clk);
    }

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
    /// This base method validates the provided `name` based on the device's `task_type`
    /// to ensure it adheres to the expected naming convention for the respective task type.
    ///
    /// # Naming Conventions:
    /// - For `TaskType::AO`: Channels should be named following the pattern "ao(number)"
    ///   (e.g., "ao0", "ao1").
    /// - For `TaskType::DO`: Channels should be named following the pattern "port(number)/line(number)"
    ///   (e.g., "port0/line1").
    ///
    /// # Panics
    /// - If the provided `name` does not adhere to the expected naming convention for the
    ///   associated task type.
    /// - If a channel with the same `name` already exists within the device.
    ///
    /// # Arguments
    /// - `name`: Name of the channel as seen by the NI driver, which must adhere to the
    ///   naming conventions detailed above.
    /// - `default_value`: a f64 value which specifies signal value for not explicitly defined intervals. 
    fn add_channel(&mut self, name: &str, default_value: f64) {
        // Check the name format
        let (name_match_string, name_format_description) = match self.task_type() {
            TaskType::AO => (String::from(r"^ao\d+$"), String::from("ao(number)")),
            TaskType::DO => (
                String::from(r"^port\d+/line\d+$"),
                String::from("port(number)/line(number)"),
            ),
        };

        let re = Regex::new(&name_match_string).unwrap();
        if !re.is_match(name) {
            panic!(
                "Expecting channels to be of format '{}' yet received channel name {}",
                name_format_description, name
            );
        }
        for channel in self.channels().values() {
            if channel.name() == name {
                panic!(
                    "Physical name of channel {} already registered. Registered channels are {:?}",
                    name,
                    self.channels()
                        .values()
                        .map(|c| c.name())
                        .collect::<Vec<_>>()
                );
            }
        }
        let new_channel = Channel::new(self.task_type(), name, self.samp_rate(), default_value);
        self.channels_().insert(name.to_string(), new_channel);
    }

    fn add_reset_instr(&mut self, reset_time: f64) {
        let reset_pos = (reset_time * self.samp_rate()).round() as usize;
        if reset_pos < self.last_instr_end_pos() {
            panic!(
                "Given reset_time {reset_time} was rounded to {reset_pos} clock cycles \
                which is below the last instruction end_pos {}",
                self.last_instr_end_pos()
            )
        }
        for chan in self.editable_channels_().iter_mut() {  // ToDo when splitting AO/DO types: remove `editable` filter
            chan.add_reset_instr(reset_pos)
        }
    }

    /// A device is compiled if any of its editable channels are compiled.
    /// Also see [`BaseChannel::is_compiled`]
    fn is_compiled(&self) -> bool {
        self.editable_channels()
            .iter()
            .any(|channel| channel.is_compiled())
    }
    /// A device is marked edited if any of its editable channels are edited.
    /// Also see [`BaseChannel::is_edited`]
    fn is_edited(&self) -> bool {
        self.editable_channels()
            .iter()
            .any(|channel| channel.is_edited())
    }
    /// A device is marked fresh-compiled if all if its editable channels are freshly compiled.
    /// Also see [`BaseChannel::is_fresh_compiled`]
    fn is_fresh_compiled(&self) -> bool {
        self.editable_channels()
            .iter()
            .all(|channel| channel.is_fresh_compiled())
    }
    /// Clears the edit-cache fields for all channels.
    /// Also see [`BaseChannel::clear_edit_cache`]
    fn clear_edit_cache(&mut self) {
        // Remove all made-up "port" channels
        self.channels_().retain(|_name, chan| chan.editable());

        for chan in self.channels_().values_mut() {
            chan.clear_edit_cache()
        }
    }
    /// Clears the compile-cache fields for all channels.
    /// Also see [`BaseChannel::clear_compile_cache`]
    fn clear_compile_cache(&mut self) {
        // Remove all made-up "port" channels
        self.channels_().retain(|_name, chan| chan.editable());

        for chan in self.channels_().values_mut() {
            chan.clear_compile_cache()
        }
    }

    fn check_end_clipped(&self, stop_tick: usize) -> bool {  // ToDo: TestMe
        if stop_tick < self.last_instr_end_pos() {
            panic!("Given stop_tick {stop_tick} is below the last instruction end_pos {}",
                   self.last_instr_end_pos())
        }
        self.channels()
            .values()
            .filter(|chan| chan.editable())  // ToDo when splitting AO/DO types: remove `editable()` filter
            .filter(|chan| !chan.instr_list().is_empty())
            .any(|chan| {
                let last_instr = chan.instr_list().last().unwrap();
                match last_instr.end_pos() {
                    Some(end_pos) => end_pos == stop_tick,
                    None => false
                }
            })
    }

    /// Compiles all editable channels to produce a continuous instruction stream.
    ///
    /// The method starts by compiling each individual editable channel to obtain a continuous
    /// stream of instructions (also see[`BaseChannel::compile`]).
    /// If the device type is `TaskType::DO` (Digital Output), an additional
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
    /// - `stop_time`: The stop time used to compile the channels.
    fn compile(&mut self, stop_time: f64) -> f64 {  // ToDo: TestMe
        let stop_tick = (stop_time * self.samp_rate()).round() as usize;
        if stop_tick < self.last_instr_end_pos() {
            panic!("Given stop_time {stop_time} was rounded to {stop_tick} clock cycles which is below the last instruction end_pos {}",
                   self.last_instr_end_pos())
        }

        // If on any of the channels, the last instruction has `end_spec = Some(end_pos, ...)`
        // and requested `stop_tick` precisely matches `end_pos`,
        // we ask the card to generate an additional sample at the end to ensure this "closing edge" is reliably formed.
        //
        // Explanation:
        // If there were no extra sample, generation will simply stop at the last sample of the pulse
        // and what happens next would be hardware-dependent. Specifically NI cards simply keep the last generated value,
        // resulting in the pulse having the first "opening" edge, but not having the second "closing" edge.
        //
        // To avoid this issue (and any similar surprises for other hardware platforms),
        // we explicitly ask the card to run for one more clock cycle longer and generate the extra sample at the end.
        // Channel's `compile()` logic will fill this sample with the last instruction's after-end padding
        // thus reliably forming its' "closing edge".
        let stop_pos = if self.check_end_clipped(stop_tick) {
            stop_tick + 1
        } else {
            stop_tick
        };
        // Compile all channels
        for chan in self.editable_channels_() {
            chan.compile(stop_pos)
        };

        // For DO channels: merge line channels into port channels
        if self.task_type() == TaskType::DO {
            // Remove all made-up "port" channels left from the previous compile run
            //  (although all port channels with new instructions coming would be replaced anyways during `self.channels_().insert()`,
            //  this step cleans out any old port channels for which there are no instructions this time)
            self.channels_().retain(|_name, chan| chan.editable());

            for match_port in self.unique_port_numbers() {
                // Collect a sorted list of possible intervals
                let mut instr_end_set = BTreeSet::new();
                instr_end_set.extend(
                    self.editable_channels()
                        .iter()
                        .filter(|chan| chan.is_edited() && extract_port_line_numbers(chan.name()).0 == match_port)
                        .flat_map(|chan| chan.instr_end().iter()),
                );
                let instr_end: Vec<usize> = instr_end_set.into_iter().collect();

                let mut instr_val = vec![0.; instr_end.len()];
                for chan in self.editable_channels().iter().filter(|chan| chan.is_edited()) {
                    let (port, line) = extract_port_line_numbers(chan.name());
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
                    // The default value for merged port channel does not matter since we never explicitly compile them
                    0.
                );
                *port_channel.instr_val_() = port_instr_val;
                *port_channel.instr_end_() = instr_end;
                self.channels_()
                    .insert(port_channel.name().to_string(), port_channel);
            }
        };

        // Return the total run duration to generate all the samples:
        self.total_run_time()
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

    /// Returns the total number of samples the card will generate according to the current compile cache.
    fn total_samps(&self) -> usize {
        // The assumption is that all the channels of any given device
        // must have precisely the same number of samples to generate
        // since all the channels are assumed to be driven by the same sample clock of the device.
        //
        // This function first checks `total_samps` are indeed consistent across all compiled channels
        // and then returns the common `total_samps`.

        // Collect `total_samps` from all compiled channels into an `IndexMap`
        let samps_per_chan: IndexMap<String, usize> =
            self.channels()
                .into_iter()
                .filter(|(_chan_name, chan)| !chan.instr_end().is_empty())
                .map(|(chan_name, chan)| (chan_name.to_string(), chan.total_samps()))
                .collect();

        if samps_per_chan.is_empty() {
            return 0
        } else {
            // To verify consistency, compare all against the first one:
            let &first_val = samps_per_chan.values().next().unwrap();
            let all_equal = samps_per_chan.values().all(|&stop_pos| stop_pos == first_val);
            if all_equal {
                return first_val
            } else {
                panic!(
                    "Channels of device {} have unequal compiled stop positions:\n\
                    {:?}\n\
                    When working at a device level, you are not supposed to compile individual channels directly. \
                    Instead, call `my_device.compile(stop_pos)` and it will compile all channels with the same `stop_pos`",
                    self.name(), samps_per_chan
                )
            }
        }

    }
    /// Calculates the maximum stop time among all compiled channels.
    ///
    /// Iterates over all the compiled channels in the device, regardless of their streamability or
    /// editability, and determines the maximum stop time.
    /// See [`BaseChannel::total_run_time`] for more information.
    ///
    /// # Returns
    /// A `f64` representing the maximum stop time (in seconds) across all compiled channels.
    fn total_run_time(&self) -> f64 {
        self.total_samps() as f64 * self.clock_period()
    }

    fn last_instr_end_pos(&self) -> usize {
        self.channels()
            .values()
            .filter(|chan| chan.editable())  // ToDo when splitting AO/DO types: remove `editable()` filter
            .map(|chan| chan.last_instr_end_pos())
            .fold(0, usize::max)
    }
    /// Calculates the maximum stop time among all editable channels and optionally adds an extra tick duration.
    ///
    /// This function determines the maximum stop time by iterating over all editable channels. 
    /// If `extra_tail_tick` is `true`, an additional duration, equivalent to one tick of the device's 
    /// sampling rate, is added to the maximum stop time.
    ///
    /// See [`BaseChannel::edit_stop_time`] for how individual channel stop times are determined.
    ///
    /// # Returns
    /// A `f64` representing the maximum stop time (in seconds) across all editable channels, 
    /// optionally increased by the duration of one tick.
    fn last_instr_end_time(&self) -> f64 {
        self.last_instr_end_pos() as f64 * self.clock_period()
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
            self.name(),
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
        // ToDo: look through
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
            self.name()
        );

        let mut port_numbers = BTreeSet::new();

        self.compiled_channels(false, true).iter().for_each(|chan| {
            // Capture the port
            let name = &chan.name();
            port_numbers.insert(extract_port_line_numbers(name).0);
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
/// - `name`: Name of the device as seen by the NI driver.
/// - `task_type`: Specifies the task type associated with the device.
/// - `samp_rate`: The sampling rate of the device in Hz.
/// - `samp_clk_src`: Optional source of the sampling clock; supply `None` for on-board clock source.
/// - `trig_line`: Optional identifier for the port through which to import/export the task start trigger.
///     Supply `None` for trivial triggering behavior.
/// - `export_trig`: Optional Boolean indicating if the device exports its start trigger. If `true`, the device
///     exports the start trigger of the NI-task associated with this device through `trig_line`. If `false` or `None`,
///     the device is set to import the start trigger. In case that any device in an experiment has nontrivial triggering behavior,
///     one and only one of the devices must have `export_trig` set to `true`.
/// - `ref_clk_line`: Optional source of the reference clock to phase-lock the device clock to.
/// - `export_ref_clk`: Optional indicator of whether to export the reference clock. If `true`, the device exports its
///     reference clock. If `false` or `None`, it imports the reference clock. Use `None` for trivial behavior.
/// - `ref_clk_rate`: Optional rate of the reference clock in Hz.
///
/// # Synchronization Methods
///
/// For experiments that do not require synchronization between devices, set all optional fields of `Device` to `None`.
/// However, for more accurate and cohesive experiments, we recommend at least implementing start-trigger synchronization.
///
/// ## Start-trigger Synchronization
///
/// Relevant fields: `trig_line`, `export_trig`.
///
/// Refer to the official [NI documentation on start-trigger synchronization](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/mxcncpts/syncstarttrigger.html).
///
/// This method designates one device to export its start trigger and others to import. When the experiment begins, tasks on
/// devices with `export_trig` set to `false` are set to wait for a digital edge trigger from the `trig_line` channel. The device with `export_trig` set to `true` exports its start trigger to `trig_line`.
///
/// **Note**: It's essential to physically connect the device that exports its trigger (where `export_trig` is `true`) to the corresponding lines on devices that import the trigger.
///
/// For PCIe devices, use a `PFI` label. For PXIe devices, use the label `PXI_Trig` followed by a number in the range 0-7.
/// This backend crate ensures task synchronization such that threads handling tasks set to import the trigger always start listening for triggers before the exporting task begins.
///
/// For PXIe devices linked to a chassis, ensure that you configure trigger bus routing using NI-MAX (on Windows) or the
/// NI Hardware Configuration Utilities (on Linux) when specifying backplane trigger lines. Detailed information can be
/// found [here](https://www.ni.com/docs/en-US/bundle/pxi-platform-services-help/page/trigger_routing_and_reservation.html).
///
/// It's important to note that after starting, each device's task utilizes its internal clock, which may result in incremental
/// drifts between devices over time. For longer signals, it's advisable to use additional synchronization methods to ensure
/// clock alignment.
///
/// ### Example:
/// Here, the device `PXI1Slot6` exports its start trigger to `PXI1_Trig0`, while `PXI1Slot7` imports its start
/// trigger from the same line.
/// ```
/// # use nicompiler_backend::*;
/// let mut exp = Experiment::new();
/// exp.add_do_device("PXI1Slot6", 1e6);
/// exp.add_do_device("PXI1Slot7", 1e6);
/// exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", true);
/// exp.device_cfg_trig("PXI1Slot7", "PXI1_Trig0", false);
/// ```
///
/// The compiler will panic if more than one device exports trigger
/// ```should_panic
/// # use nicompiler_backend::*;
/// let mut exp = Experiment::new();
/// exp.add_do_device("PXI1Slot6", 1e6);
/// exp.add_do_device("PXI1Slot7", 1e6);
/// exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", true);
/// exp.device_cfg_trig("PXI1Slot7", "PXI1_Trig0", true);
/// ```
///
/// The compiler **will** panic if some device is expecting a start trigger yet no device exports one.
/// ```should_panic
/// # use nicompiler_backend::*;
/// let mut exp = Experiment::new();
/// exp.add_do_device("PXI1Slot6", 1e6);
/// exp.add_do_channel("PXI1Slot6", 0, 4, 0.);
/// exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", false);
/// exp.go_high("PXI1Slot6", "port0/line4", 0.5);
/// exp.compile_with_stoptime(1.); // Panics here
/// ```
///
/// ## Phase-lock to Reference Clock
///
/// Relevant fields: `ref_clk_line`, `ref_clk_rate`, `export_ref_clk`.
///
/// Refer to the [NI documentation on phase-lock synchronization](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/mxcncpts/syncrefclock.html).
///
/// A subset of NI devices support this flexible synchronization method, which allows devices synchronized in this manner
/// to operate at different sampling rates. Devices phase-lock their on-board oscillators to an external reference at `ref_clk_line`
/// and indicate its frequency via `ref_clk_rate`. A device can optionally export its 10MHz onboard reference clock to `ref_clk_line` by setting `export_ref_clk` to `true`.
///
/// **Note**: Devices phase-locked in this manner still require start-trigger synchronization to ensure synchronized start times.
///
/// ### Example:
/// The device `PXI1Slot6` exports its start trigger signal to `PXI1_Trig0` and its 10MHz reference clock to `PXI1_Trig7`.
/// The device `PXI1Slot4` acts accordingly.
/// ```rust
/// use nicompiler_backend::*;
/// let mut exp = Experiment::new();
/// exp.add_ao_device("PXI1Slot3", 1e6);
/// exp.device_cfg_trig("PXI1Slot3", "PXI1_Trig0", true);
/// exp.device_cfg_ref_clk("PXI1Slot3", "PXI1_Trig7", 1e7, true);
///
/// exp.add_ao_device("PXI1Slot4", 1e6);
/// exp.device_cfg_trig("PXI1Slot4", "PXI1_Trig0", false);
/// exp.device_cfg_ref_clk("PXI1Slot4", "PXI1_Trig7", 1e7, false);
/// ```
///
/// ## Importing Sample Clock
///
/// Relevant fields: `samp_clk_src`.
///
/// Check out the [NI documentation on sample clock synchronization](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/mxcncpts/syncsampleclock.html).
///
/// Some NI devices do not support reference clock synchronization. As an alternative, they can directly use external
/// clock signals for their sampling clock. However, this constrains them to operate at the same rate as the imported sample clock.
///
/// ### Example:
/// Building on the previous example, an additional `PXI1Slot6` sources its sample clock from the 10MHz signal exported by `PXI1Slot3`.
/// ```rust
/// use nicompiler_backend::*;
/// let mut exp = Experiment::new();
/// exp.add_ao_device("PXI1Slot3", 1e6);
/// exp.device_cfg_trig("PXI1Slot3", "PXI1_Trig0", true);
/// exp.device_cfg_ref_clk("PXI1Slot3", "PXI1_Trig7", 1e7, true);
///
/// exp.add_ao_device("PXI1Slot4", 1e6);
/// exp.device_cfg_trig("PXI1Slot4", "PXI1_Trig0", false);
/// exp.device_cfg_ref_clk("PXI1Slot4", "PXI1_Trig7", 1e7, false);
///
/// exp.add_do_device("PXI1Slot6", 1e7);
/// exp.device_cfg_samp_clk_src("PXI1Slot6", "PXI1_Trig7");
/// exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", false);
/// ```
pub struct Device {
    channels: IndexMap<String, Channel>,

    name: String,
    task_type: TaskType,
    samp_rate: f64,

    samp_clk_src: Option<String>,
    trig_line: Option<String>,
    export_trig: Option<bool>,
    ref_clk_line: Option<String>,
    export_ref_clk: Option<bool>,
    ref_clk_rate: Option<f64>,
}

impl Device {
    /// Constructs a new `Device` instance.
    ///
    /// This constructor initializes a device with the provided parameters. The `channels` field
    /// is initialized as an empty collection. All synchronization fields are initialized to `None`
    /// by default. For nontrivial synchronization behavior, use the methods [`BaseDevice::cfg_samp_clk_src`],
    /// [`BaseDevice::cfg_trig`], and [`BaseDevice::cfg_ref_clk`].
    ///
    /// # Arguments
    /// - `name`: Name of the device as seen by the NI driver.
    /// - `task_type`: The type of task associated with the device.
    /// - `samp_rate`: Desired sampling rate in Hz.
    ///
    /// # Returns
    /// A new instance of `Device` with the specified configurations and all synchronization-related fields set to `None`.
    pub fn new(name: &str, task_type: TaskType, samp_rate: f64) -> Self {
        Self {
            channels: IndexMap::new(),

            name: name.to_string(),
            task_type,
            samp_rate,

            samp_clk_src: None,
            trig_line: None,
            export_trig: None,
            ref_clk_line: None,
            export_ref_clk: None,
            ref_clk_rate: None,
        }
    }
}

impl BaseDevice for Device {
    // Immutable accessors (getters)
    fn channels(&self) -> &IndexMap<String, Channel> {
        &self.channels
    }

    fn name(&self) -> &str {
        &self.name
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

    fn export_trig(&self) -> Option<bool> {
        self.export_trig
    }

    fn ref_clk_line(&self) -> Option<&str> {
        self.ref_clk_line.as_deref()
    }

    fn export_ref_clk(&self) -> Option<bool> {
        self.export_ref_clk
    }

    fn ref_clk_rate(&self) -> Option<f64> {
        self.ref_clk_rate
    }

    // Mutable accessors
    fn channels_(&mut self) -> &mut IndexMap<String, Channel> {
        &mut self.channels
    }

    fn samp_clk_src_(&mut self) -> &mut Option<String> {
        &mut self.samp_clk_src
    }

    fn trig_line_(&mut self) -> &mut Option<String> {
        &mut self.trig_line
    }

    fn export_trig_(&mut self) -> &mut Option<bool> {
        &mut self.export_trig
    }

    fn ref_clk_line_(&mut self) -> &mut Option<String> {
        &mut self.ref_clk_line
    }

    fn export_ref_clk_(&mut self) -> &mut Option<bool> {
        &mut self.export_ref_clk
    }

    fn ref_clk_rate_(&mut self) -> &mut Option<f64> {
        &mut self.ref_clk_rate
    }
}

#[cfg(test)]
mod test {
    use crate::device::*;
    use crate::instruction::*;

    #[test]
    fn last_instr_end_pos() {
        let mut dev = Device::new("Dev1", TaskType::AO, 1e3);
        dev.add_channel("ao0", 0.0);
        dev.add_channel("ao1", 0.0);
        let mock_func = Instruction::new_const(0.0);

        // No instructions
        assert_eq!(dev.last_instr_end_pos(), 0);

        // Instruction t=0..1 on ao0
        dev.chan_("ao0").add_instr(mock_func.clone(),
            0.0, Some((1.0, false))
        );
        assert_eq!(dev.last_instr_end_pos(), 1000);

        // Instruction t=1..2 on ao1
        dev.chan_("ao1").add_instr(mock_func.clone(),
            1.0, Some((1.0, false))
        );
        assert_eq!(dev.last_instr_end_pos(), 2000);

        // "Go-something" instruction on ao1 at t=2
        dev.chan_("ao1").add_instr(mock_func.clone(),
            2.0, None
        );
        assert_eq!(dev.last_instr_end_pos(), 2001);

        dev.clear_edit_cache();
        assert_eq!(dev.last_instr_end_pos(), 0);
    }

    #[test]
    fn check_end_clipped() {
        let mut dev = Device::new("Dev1", TaskType::AO, 1.0);
        dev.add_channel("ao0", 0.0);
        let mock_func = Instruction::new_const(0.0);

        // (1) No instructions
        assert_eq!(dev.check_end_clipped(0), false);

        // (2) Finite duration instruction t = 0..1s:
        //      start_pos = 0
        //      end_pos = 1
        dev.chan_("ao0").add_instr(mock_func.clone(),
            0.0, Some((1.0, false))
        );
        assert_eq!(dev.chan("ao0").last_instr_end_pos(), 1);
        assert_eq!(dev.check_end_clipped(2), false);
        assert_eq!(dev.check_end_clipped(1), true);
        dev.clear_edit_cache();

        // (3) "Go-something" instruction at t = 0s:
        //      start_pos = 0
        //      eff_end_pos = 1
        dev.chan_("ao0").add_instr(mock_func.clone(),
            0.0, None
        );
        assert_eq!(dev.chan("ao0").last_instr_end_pos(), 1);
        //  A "go-something" instruction is not meant to have the "closing" edge
        //  so setting `stop_tick` to precisely `eff_end_pos` is not considered clipping
        assert_eq!(dev.check_end_clipped(1), false);
    }

    #[test]
    fn compile() {
        let mut dev = Device::new("Dev1", TaskType::AO, 1e3);
        dev.add_channel("ao0", 0.0);
        dev.add_channel("ao1", 0.0);
        let mock_func = Instruction::new_const(0.0);

        // Not compiled yet
        assert_eq!(dev.total_samps(), 0);

        // Add some instructions on both channels
        dev.chan_("ao0").add_instr(mock_func.clone(),
            0.0, Some((1.0, false))
        );
        dev.chan_("ao1").add_instr(mock_func.clone(),
            1.0, Some((1.0, false))
        );
        assert_eq!(dev.last_instr_end_pos(), 2000);

        // Compile without clipping of the "closing edge" - no extra sample should be added
        dev.compile(3.0);
        assert_eq!(dev.total_samps(), 3000);

        // Compile with stop_pos matching the end of a finite-duration instruction on "ao1" -
        //  an additional sample should be added to form the "closing edge"
        dev.compile(2.0);
        assert_eq!(dev.total_samps(), 2001);
    }
}