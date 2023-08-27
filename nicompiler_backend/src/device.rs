//! Implements struct and methods corresponding to NI devices. See [`BaseDevice`] for
//! implementation details.
//!
//! A NI control system corresponds to devices (cards) directly attached the computer via
//! PCIe/USB, or a PXIe chassis connected via a PCIe link card which, in turn, hosts multiple
//! PXIe cards. In this library, every `Device` entity corresponds to a particular task for
//! a physical device (e.g. analogue output for `PXI1Slot1`). A `Device`, trivially implementing
//! the [`BaseDevice`] trait, keeps tracks of of the physical channels associated with the device
//! as well as device-wide data such as device name, trigger line, and synchronization behavior.

use ndarray::{s, Array1, Array2};
use regex::Regex;
use std::collections::{BTreeSet, HashMap};

use crate::channel::*;
use crate::instruction::*;
use crate::utils::*;

// Trait-implementation of expectations for a device
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
    

    // Channels produced by edits. For DO, this means the line-channels
    fn editable_channels(&self) -> Vec<&Channel> {
        self.channels()
            .values()
            .filter(|&chan| chan.editable())
            .collect()
    }
    fn editable_channels_(&mut self) -> Vec<&mut Channel> {
        self.channels_()
            .values_mut()
            .filter(|chan| (*chan).editable())
            .collect()
    }

    fn add_channel(&mut self, physical_name: &str) {
        // Check the physical_name format
        let (name_match_string, name_format_description) = match self.task_type() {
            TaskType::AO => (String::from(r"^ao\d+$"), String::from("ao[number]")),
            TaskType::DO => (
                String::from(r"^port\d+/line\d+$"),
                String::from("port[number]/line[number]"),
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

    // Channel-broadcast methods
    fn is_compiled(&self) -> bool {
        self.editable_channels()
            .iter()
            .any(|channel| channel.is_compiled())
    }
    fn is_edited(&self) -> bool {
        self.editable_channels()
            .iter()
            .any(|channel| channel.is_edited())
    }
    fn is_fresh_compiled(&self) -> bool {
        self.editable_channels()
            .iter()
            .all(|channel| channel.is_fresh_compiled())
    }
    fn clear_edit_cache(&mut self) {
        self.editable_channels_()
            .iter_mut()
            .for_each(|chan| chan.clear_edit_cache());
    }
    fn clear_compile_cache(&mut self) {
        self.editable_channels_()
            .iter_mut()
            .for_each(|chan| chan.clear_compile_cache());
    }

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

    fn compiled_stop_time(&self) -> f64 {
        self.compiled_channels(false, false)
            .iter()
            .map(|chan| chan.compiled_stop_time())
            .fold(0.0, f64::max)
    }

    fn edit_stop_time(&self) -> f64 {
        self.editable_channels()
            .iter()
            .map(|chan| chan.edit_stop_time())
            .fold(0.0, f64::max)
    }

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
        let mut timer = TickTimer::new();
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
/// - `physical_name`: An identifier for the device as seen by the NI driver.
/// - `task_type`: Specifies the task type associated with the device.
/// - `samp_rate`: The sampling rate of the device in Hz.
/// - `samp_clk_src`: Optional source of the sampling clock.
/// - `trig_line`: Optional identifier for the trigger line.
/// - `is_primary`: Optional Boolean indicating if the device is the primary device.
/// - `ref_clk_src`: Optional source of the reference clock.
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