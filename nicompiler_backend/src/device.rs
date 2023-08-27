use regex::Regex;
use std::ops::AddAssign;
use std::collections::{BTreeSet, HashMap};
use ndarray::{s, Array1, Array2};

use crate::channel::*;
use crate::utils::*;

#[derive(PartialEq, Clone, Copy)]
pub enum DeviceType {
    AODevice,
    DODevice,
}

// Trait-implementation of expectations for a device
pub trait BaseDevice {
    // Field methods
    fn physical_name(&self) -> &str;
    fn samp_rate(&self) -> f64;
    fn trig_line(&self) -> &str;
    fn is_primary(&self) -> bool;
    fn channels(&self) -> &HashMap<String, Channel>;
    fn channels_(&mut self) -> &mut HashMap<String, Channel>;
    fn device_type(&self) -> DeviceType;

    fn add_channel(&mut self, physical_name: &str) {
        // Check the physical_name format
        let (name_match_string, name_format_description) = match self.device_type() {
            DeviceType::AODevice => (String::from(r"^ao\d+$"), String::from("ao[number]")),
            DeviceType::DODevice => (
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
        let new_channel = Channel::new(physical_name, self.samp_rate());
        self.channels_()
            .insert(physical_name.to_string(), new_channel);
    }

    // Channel-broadcast methods
    fn is_compiled(&self) -> bool {
        self.channels()
            .values()
            .any(|channel| channel.is_compiled())
    }
    fn is_edited(&self) -> bool {
        self.channels().values().any(|channel| channel.is_edited())
    }
    fn is_fresh_compiled(&self) -> bool {
        self.channels()
            .values()
            .all(|channel| channel.is_fresh_compiled())
    }
    fn clear_edit_cache(&mut self) {
        self.channels_()
            .values_mut()
            .for_each(|chan| chan.clear_edit_cache());
    }
    fn clear_compile_cache(&mut self) {
        self.channels_()
            .values_mut()
            .for_each(|chan| chan.clear_compile_cache());
    }

    fn compile(&mut self, stop_pos: usize) {
        self.channels_()
            .values_mut()
            .for_each(|chan| chan.compile(stop_pos));

        // if self.device_type() == DeviceType::DODevice {

        // }
    }

    fn compiled_channels(&self) -> Vec<&Channel> {
        self.channels()
            .values()
            .filter_map(|chan| {
                if chan.is_compiled() {
                    Some(&*chan)
                } else {
                    None
                }
            })
            .collect()
    }

    fn compiled_stop_time(&self) -> f64 {
        self.compiled_channels()
            .iter()
            .map(|chan| chan.compiled_stop_time())
            .fold(0.0, f64::max)
    }

    fn edit_stop_time(&self) -> f64 {
        self.channels()
            .values()
            .map(|chan| chan.edit_stop_time())
            .fold(0.0, f64::max)
    }

    fn fill_signal_nsamps(
        &self,
        start_pos: usize,
        end_pos: usize,
        nsamps: usize,
        buffer: &mut ndarray::Array2<f64>,
    ) {
        // Assumes buffer of shape [num_compiled_channels][nsamps]
        assert!(
            buffer.dim().0 == self.compiled_channels().len(),
            "Device {} has {} channels but passed buffer has shape {:?}",
            self.physical_name(),
            self.compiled_channels().len(),
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
        for (i, chan) in self.compiled_channels().iter().enumerate() {
            let mut channel_slice = buffer.slice_mut(s![i, ..]);
            chan.fill_signal_nsamps(start_pos, end_pos, nsamps, &mut channel_slice);
        }
    }

    fn calc_signal_nsamps(&self, start_pos: usize, end_pos: usize, nsamps: usize) -> Array2<f64> {
        let num_chans = self.compiled_channels().len();
        let t_values = Array1::linspace(
            start_pos as f64 / self.samp_rate(),
            end_pos as f64 / self.samp_rate(),
            nsamps,
        );
        let mut buffer = Array2::from_elem((num_chans, nsamps), 0.);

        buffer
            .outer_iter_mut()
            .for_each(|mut row| row.assign(&t_values));
        self.fill_signal_nsamps(start_pos, end_pos, nsamps, &mut buffer);
        buffer
    }

    fn unique_port_numbers(&self) -> Vec<usize> {
        assert!(self.device_type() == DeviceType::DODevice,
                "unique ports should only be invoked for DODevices, but {} is not",
                self.physical_name());
        
        let mut port_numbers = BTreeSet::new();
        
        self.compiled_channels().iter().for_each(|chan| {
            // Capture the port
            let physical_name = &chan.physical_name();
            port_numbers.insert(extract_port_line_numbers(physical_name).0);
        });
        port_numbers.into_iter().collect()
    }

    // The only difference here is that for DODevices, we need to add the port channels together 
    fn calc_stream_signal(&self, start_pos: usize, end_pos: usize) -> Array2<f64> {
        let chan_signal = self.calc_signal_nsamps(start_pos, end_pos, end_pos-start_pos);
        match self.device_type() {
            DeviceType::AODevice => chan_signal, // AODevice, each channel directly corresponds to NI-DAQ channel
            DeviceType::DODevice => {
                let port_numbers = self.unique_port_numbers();
                let mut port_signal = Array2::from_elem((port_numbers.len(), end_pos-start_pos), 0.);
                let compiled_channels = self.compiled_channels();
                let port_lines = compiled_channels.iter().map(|chan| extract_port_line_numbers(chan.physical_name()));
                for (i, (port, line)) in port_lines.enumerate() {
                    for (j, port_number) in port_numbers.iter().enumerate() {
                        if port == *port_number {
                            let exponentiated = chan_signal.row(i).to_owned().mapv(|x| x*2.0f64.powf(line as f64));
                            port_signal.row_mut(j).add_assign(&exponentiated);                            
                        }
                    }
                }
                port_signal
            }
        }
    }
}

pub struct Device {
    physical_name: String,
    trig_line: String,
    is_primary: bool,
    device_type: DeviceType,
    samp_rate: f64,
    channels: HashMap<String, Channel>,
}

// Only non-trait implementation for device: constructor
impl Device {
    pub fn new(
        physical_name: &str,
        trig_line: &str,
        dev_type: DeviceType,
        is_primary: bool,
        samp_rate: f64,
    ) -> Self {
        Self {
            physical_name: physical_name.to_string(),
            trig_line: trig_line.to_string(),
            is_primary,
            device_type: dev_type,
            samp_rate,
            channels: HashMap::new(),
        }
    }
}

// Field methods for device class
impl BaseDevice for Device {
    fn physical_name(&self) -> &str {
        &self.physical_name
    }
    fn samp_rate(&self) -> f64 {
        self.samp_rate
    }
    fn trig_line(&self) -> &str {
        &self.trig_line
    }
    fn is_primary(&self) -> bool {
        self.is_primary
    }
    fn channels(&self) -> &HashMap<String, Channel> {
        &self.channels
    }
    fn channels_(&mut self) -> &mut HashMap<String, Channel> {
        &mut self.channels
    }
    fn device_type(&self) -> DeviceType {
        self.device_type
    }
}
