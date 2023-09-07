//! The experiment module provides the highest level of abstraction for managing NI experiments, and
//! the single place by which methods are exposed to python.
//!
//! ## Overview
//!
//! At the heart of this module lies the [`Experiment`] struct, which consists of a collection of devices.
//! The behavior of the `Experiment` struct is primarily defined
//! by the [`BaseExperiment`] trait, which prescribes a collection of methods for experiment management and manipulation.
//!
//! The module is organized into the following primary components:
//!
//! 1. **Experiment Struct**: The main data structure representing the entire experimental setup. It houses devices
//!    and their associated channels.
//! 2. **Traits**: Including the pivotal [`BaseExperiment`] trait, which defines the expected behaviors and operations
//!    possible on an `Experiment`.
//! 3. **Macro**: The module features a macro, `impl_exp_boilerplate!`, designed to generate boilerplate code to assist
//!    in bridging Rust's trait system and Python's class system, as well as to make the python methods extensible.
//!
//! ## Key Structures and Their Relationships
//!
//! - **Experiment**: This is the main structure that users interact with. It represents a collection of devices and
//!   provides methods for their management.
//! - **Device**: Each device, represented by the [`Device`] struct, corresponds to a specific piece of NI hardware.
//!   Devices contain channels, and methods in the `Experiment` struct often redirect to these devices.
//! - **Channel**: Channels, denoted by the [`Channel`] struct, symbolize distinct physical channels on an NI device.
//!   They hold instructions and other functionalities specific to the channel.
//! - **Instruction**: Instructions, housed within [`InstrBook`], define specific tasks or commands for channels.
//!
//! ## Navigating the Module
//!
//! If you're looking to:
//!
//! - **Understand core behaviors**: Dive into the [`BaseExperiment`] trait.
//! - **Integrate with python**: Refer to the [`impl_exp_boilerplate`] macro and its source. The macro provides
//! python-exposed wrappers for methods implemented in [`BaseExperiment`] trait.

use ndarray::Array2;
use numpy;
use pyo3::prelude::*;
use std::collections::HashMap;

use crate::channel::*;
use crate::device::*;
use crate::instruction::*;

/// This trait defines the behavior of the [`Experiment`] struct through default trait implementations.
///
/// Trait methods are primary classified into the following categories:
/// 1. Experiment-targed methods which alter or query the behavior of the entire experiment:
///     - [`add_ao_device`], [`add_do_device`]
///     - [`compile`], [`compile_with_stoptime`]
///     - [`edit_stop_time`], [`compiled_stop_time`]
///     - [`is_edited`], [`is_compiled`], [`is_fresh_compiled`]
///     - [`clear_edit_cache`], [`clear_compile_cache`]
/// 2. Device-targeted methods which alter or query the behavior of a specific device:
///     - [`add_ao_channel`], [`add_do_channel`]
///     - [`device_calc_signal_nsamps`], [`device_compiled_channel_names`]
///     - [`device_edit_stop_time`], [`device_compiled_stop_time`]
///     - [`device_clear_compile_cache`], [`device_clear_edit_cache`]
/// 3. Channel-targeted methods which alter or query the behavior of a particular channel
///     - [`constant`], [`sine`], [`high`], [`low`], [`go_high`], [`go_low`]
///     - [`channel_clear_compile_cache`], [`channel_clear_edit_cache`]
///     - [`channel_calc_signal_nsamps`]
/// 4. Internal helper methods which are not exposed to python
///     - [`devices`], [`devices_`]
///     - [`assert_has_device`], [`assert_device_has_channel`]
///     - [`typed_device_op`], [`device_op`], [`typed_channel_op`], [`channel_op`]
///
/// [`add_ao_device`]: BaseExperiment::add_ao_device
/// [`add_do_device`]: BaseExperiment::add_do_device
/// [`compile`]: BaseExperiment::compile
/// [`compile_with_stoptime`]: BaseExperiment::compile_with_stoptime
/// [`edit_stop_time`]: BaseExperiment::edit_stop_time
/// [`compiled_stop_time`]: BaseExperiment::compiled_stop_time
/// [`is_edited`]: BaseExperiment::is_edited
/// [`is_compiled`]: BaseExperiment::is_compiled
/// [`is_fresh_compiled`]: BaseExperiment::is_fresh_compiled
/// [`clear_edit_cache`]: BaseExperiment::clear_edit_cache
/// [`clear_compile_cache`]: BaseExperiment::clear_compile_cache
/// [`add_ao_channel`]: BaseExperiment::add_ao_channel
/// [`add_do_channel`]: BaseExperiment::add_do_channel
/// [`device_calc_signal_nsamps`]: BaseExperiment::device_calc_signal_nsamps
/// [`device_edit_stop_time`]: BaseExperiment::device_edit_stop_time
/// [`device_compiled_stop_time`]: BaseExperiment::device_compiled_stop_time
/// [`device_clear_compile_cache`]: BaseExperiment::device_clear_compile_cache
/// [`device_clear_edit_cache`]: BaseExperiment::device_clear_edit_cache
/// [`constant`]: BaseExperiment::constant
/// [`sine`]: BaseExperiment::sine
/// [`high`]: BaseExperiment::high
/// [`low`]: BaseExperiment::low
/// [`go_high`]: BaseExperiment::go_high
/// [`go_low`]: BaseExperiment::go_low
/// [`devices`]: BaseExperiment::devices
/// [`devices_`]: BaseExperiment::devices_
/// [`assert_has_device`]: BaseExperiment::assert_has_device
/// [`assert_device_has_channel`]: BaseExperiment::assert_device_has_channel
/// [`typed_device_op`]: BaseExperiment::typed_device_op
/// [`device_op`]: BaseExperiment::device_op
/// [`typed_channel_op`]: BaseExperiment::typed_channel_op
/// [`channel_op`]: BaseExperiment::channel_op
/// [`channel_clear_compile_cache`]: BaseExperiment::channel_clear_compile_cache
/// [`channel_clear_edit_cache`]: BaseExperiment::channel_clear_edit_cache
/// [`device_compiled_channel_names`]: BaseExperiment::device_compiled_channel_names
/// [`channel_calc_signal_nsamps`]: BaseExperiment::channel_calc_signal_nsamps

pub trait BaseExperiment {
    // FIELD methods
    fn devices(&self) -> &HashMap<String, Device>;
    fn devices_(&mut self) -> &mut HashMap<String, Device>;

    /// Asserts that the specified device exists in the experiment.
    ///
    /// This function checks if the provided device name is present within the collection
    /// of devices in the current experiment. If the device is not found, it triggers an
    /// assertion failure with a descriptive error message indicating the missing device
    /// and a list of all registered devices.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: Name of the device to check.
    ///
    /// # Panics
    ///
    /// Panics if the provided device name is not found in the experiment's collection of devices.
    ///
    /// # Example
    /// ```
    /// use nicompiler_backend::experiment::*;
    ///
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6,);
    /// exp.assert_has_device("PXI1Slot6");
    ///
    /// // This will panic
    /// // exp.assert_has_device("PXI1Slot5");
    /// ```
    fn assert_has_device(&self, dev_name: &str) {
        assert!(
            self.devices().contains_key(dev_name),
            "Physical device {} not found. Registered devices are {:?}",
            dev_name,
            self.devices().keys().collect::<Vec<_>>()
        );
    }

    /// Asserts that the specified channel exists within the given device in the experiment.
    ///
    /// This function first checks if the provided device name is present within the collection
    /// of devices in the current experiment using the [`assert_has_device`] function.
    /// If the device is found, it then checks if the specified channel name exists within
    /// the found device. If the channel is not found, it triggers an assertion failure with
    /// a descriptive error message indicating the missing channel and a list of all registered
    /// channels within the device.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: Name of the device to look into.
    /// * `chan_name`: Name of the channel to check within the specified device.
    ///
    /// # Panics
    ///
    /// Panics if the provided channel name is not found in the specified device's collection of channels.
    ///
    /// # Example
    ///
    /// ```
    /// use nicompiler_backend::experiment::*;
    ///
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6,);
    /// exp.add_do_channel("PXI1Slot6", 0, 0);
    /// exp.assert_device_has_channel("PXI1Slot6", "port0/line0");
    ///
    /// // This will panic
    /// // exp.assert_device_has_channel("PXI1Slot6", "port0/line1");
    /// ```
    ///
    /// [`assert_has_device`]: BaseExperiment::assert_has_device
    fn assert_device_has_channel(&self, dev_name: &str, chan_name: &str) {
        self.assert_has_device(dev_name);
        let device = self.devices().get(dev_name).unwrap();
        assert!(
            device.channels().contains_key(chan_name),
            "Channel name {} not found in device {}. Registered channels are: {:?}",
            chan_name,
            dev_name,
            device.channels().keys().collect::<Vec<_>>()
        );
    }

    /// Helper method to add a device to the experiment's collection of devices.
    ///
    /// This method registers a new device to the experiment, ensuring that there are no duplicates.
    /// Used by [`BaseExperiment::add_ao_device`] and [`BaseExperiment::add_do_device`].
    ///
    /// # Arguments
    ///
    /// * `dev`: A [`Device`] instance to be added to the experiment.
    ///
    /// # Panics
    ///
    /// This method will panic if a device with the same name as the provided `dev` is already registered in the experiment.
    fn add_device_base(&mut self, dev: Device) {
        // Duplicate check
        let dev_name = dev.physical_name();
        assert!(
            !self.devices().contains_key(dev_name),
            "Device {} already registered. Registered devices are {:?}",
            dev_name,
            self.devices().keys().collect::<Vec<_>>()
        );
        self.devices_().insert(dev_name.to_string(), dev);
    }

    /// Registers an Analog Output (AO) device to the experiment.
    ///
    /// This method creates an AO device with the specified parameters and adds it to the
    /// experiment's collection of devices using the [`BaseExperiment::add_device_base`] method.
    ///
    /// # Arguments
    ///
    /// * `physical_name`: A string slice that holds the name of the AO device.
    /// * `samp_rate`: Sampling rate for the AO device.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot6", 1e6);
    /// // Adding the same device again, even with different parameters, will cause panic
    /// // exp.add_ao_device("PXI1Slot6", 1e7);
    /// ```
    fn add_ao_device(&mut self, physical_name: &str, samp_rate: f64) {
        self.add_device_base(Device::new(physical_name, TaskType::AO, samp_rate));
    }

    /// Registers a Digital Output (DO) device to the experiment.
    ///
    /// This method creates a DO device with the specified parameters and registers it to the experiment.
    ///
    /// # Arguments
    ///
    /// * `physical_name`: A string slice that holds the name of the DO device.
    /// * `samp_rate`: Sampling rate for the DO device.
    ///
    /// # Example
    /// ```should_panic
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot7", 1e6);
    /// // Adding the same device name will cause panic
    /// exp.add_do_device("PXI1Slot7", 1e7);
    /// ```
    fn add_do_device(&mut self, physical_name: &str, samp_rate: f64) {
        self.add_device_base(Device::new(physical_name, TaskType::DO, samp_rate));
    }

    /// Retrieves the latest `edit_stop_time` from all registered devices.
    /// See [`BaseDevice::edit_stop_time`] for more information.
    ///
    /// The maximum `edit_stop_time` across all devices.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6);
    /// exp.add_do_channel("PXI1Slot6", 0, 0);
    /// exp.add_do_channel("PXI1Slot6", 0, 4);
    /// exp.high("PXI1Slot6", "port0/line0", 1., 4.); // stop time at 5
    /// assert_eq!(exp.edit_stop_time(), 5.);
    /// exp.high("PXI1Slot6", "port0/line4", 0., 6.); // stop time at 6
    /// assert_eq!(exp.edit_stop_time(), 6.);
    /// ```
    fn edit_stop_time(&self) -> f64 {
        self.devices()
            .values()
            .map(|dev| dev.edit_stop_time())
            .fold(0.0, f64::max)
    }

    /// Retrieves the `compiled_stop_time` from all registered devices.
    /// See [`BaseDevice::compiled_stop_time`] for more information.
    ///
    /// The maximum `compiled_stop_time` across all devices.
    fn compiled_stop_time(&self) -> f64 {
        self.devices()
            .values()
            .map(|dev| dev.compiled_stop_time())
            .fold(0.0, f64::max)
    }

    /// Broadcasts the compile command to all devices, relying on the `edit_stop_time`
    /// as the compilation stop-target.
    /// See [`BaseDevice::compile`] and [`BaseExperiment::compiled_stop_time`] for more information.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6);
    /// exp.add_do_channel("PXI1Slot6", 0, 0);
    /// exp.high("PXI1Slot6", "port0/line0", 1., 4.);
    ///
    /// exp.compile();
    /// assert_eq!(exp.compiled_stop_time(), exp.edit_stop_time());
    /// ```
    fn compile(&mut self) -> f64 {
        // Called without arguments, compiles based on stop_time of instructions
        let stop_time = self.edit_stop_time();
        self.compile_with_stoptime(stop_time);
        assert!(stop_time == self.compiled_stop_time());
        stop_time
    }

    /// Compiles the experiment by broadcasting the compile command to all devices.
    ///
    /// This method checks for a primary device before proceeding. An experiment must
    /// either have no primary devices or exactly one primary device to compile successfully.
    ///
    /// # Arguments
    ///
    /// * `stop_time`: The target time for the compilation.
    ///
    /// # Panics
    ///
    /// Panics if there is no primary device in the experiment or if multiple primary devices are found.
    ///
    /// # Example
    ///
    /// ```should_panic
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot6", 1e6);
    /// exp.device_cfg_trig("PXI1Slot6", "PXI_Trig0", false);
    /// // This will panic as there are no primary devices, but PXI1Slot6 is expecting a trigger source
    /// exp.compile_with_stoptime(10.0);
    /// ```
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6);
    /// exp.add_do_channel("PXI1Slot6", 0, 0);
    /// exp.high("PXI1Slot6", "port0/line0", 1., 3.);
    ///
    /// exp.compile();
    /// assert_eq!(exp.compiled_stop_time(), 4.);
    /// exp.compile_with_stoptime(5.); // Experiment signal will stop at t=5 now
    /// assert_eq!(exp.compiled_stop_time(), 5.);
    /// ```
    fn compile_with_stoptime(&mut self, stop_time: f64) {
        assert!(
            self.devices().values().all(|d| d.export_trig().is_none())
                || self
                    .devices()
                    .values()
                    .filter(|d| d.export_trig() == Some(true))
                    .count()
                    == 1,
            "Cannot compile an experiment with devices expecting yet no device exporting trigger"
        );
        self.devices_()
            .values_mut()
            .for_each(|dev| dev.compile(((stop_time) * dev.samp_rate()) as usize));
    }

    /// Retrieves a list of devices that have been successfully compiled.
    ///
    /// # Returns
    ///
    /// A vector containing references to all compiled devices.
    fn compiled_devices(&self) -> Vec<&Device> {
        self.devices()
            .values()
            .filter_map(|dev| if dev.is_compiled() { Some(&*dev) } else { None })
            .collect()
    }

    /// Checks if any of the registered devices have been edited.
    /// Also see [`BaseDevice::is_edited`].
    ///
    /// # Returns
    ///
    /// `true` if at least one device has been edited, otherwise `false`.
    fn is_edited(&self) -> bool {
        self.devices().values().any(|dev| dev.is_edited())
    }

    /// Checks if any of the registered devices have been compiled.
    /// Also see [`BaseDevice::is_compiled`].
    ///
    /// # Returns
    ///
    /// `true` if at least one device has been compiled, otherwise `false`.
    fn is_compiled(&self) -> bool {
        self.devices().values().any(|dev| dev.is_compiled())
    }

    /// Checks if all registered devices are in a freshly compiled state.
    /// Also see [`BaseDevice::is_fresh_compiled`].
    ///
    /// # Returns
    ///
    /// `true` if all devices are freshly compiled, otherwise `false`.
    fn is_fresh_compiled(&self) -> bool {
        self.devices().values().all(|dev| dev.is_fresh_compiled())
    }

    /// Clears the edit cache for all registered devices.
    /// Also see [`BaseDevice::clear_edit_cache`].
    ///
    /// This method is useful to reset or clear any temporary data or states stored during the editing phase for each device.
    fn clear_edit_cache(&mut self) {
        self.devices_()
            .values_mut()
            .for_each(|dev| dev.clear_edit_cache());
    }

    /// Clears the compile cache for all registered devices.
    /// Also see [`BaseDevice::clear_compile_cache`].
    ///
    /// This method is useful to reset or clear any temporary data or states stored during the compilation phase for each device.
    fn clear_compile_cache(&mut self) {
        self.devices_()
            .values_mut()
            .for_each(|dev| dev.clear_compile_cache());
    }

    /// Executes a specified operation (given by the closure `f`) on a targeted device of a specific `TaskType`.
    ///
    /// This method is primarily a utility function to abstract away the common checks and operations performed
    /// on a device. It first ensures the device exists, then checks if the device's task type matches the provided
    /// `task_type`, and finally, invokes the provided closure on the device.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `task_type`: The expected `TaskType` of the device.
    /// * `f`: A closure that defines the operation to be performed on the device. It should accept a mutable reference to a `Device` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// 1. If a device with the given `dev_name` doesn't exist.
    /// 2. If the device's task type doesn't match the provided `task_type`.
    ///
    /// # Returns
    ///
    /// The return value of the closure `f`.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot6", 1e6);
    /// exp.typed_device_op("PXI1Slot6", TaskType::AO, |dev| dev.clear_compile_cache());
    /// // This will panic, since we're requiring that PXI1Slot6 be DO
    /// // exp.typed_device_op("PXI1Slot6", TaskType::DO, |dev| dev.clear_compile_cache());
    /// ```
    fn typed_device_op<F, R>(&mut self, dev_name: &str, task_type: TaskType, mut f: F) -> R
    where
        F: FnMut(&mut Device) -> R,
    {
        // This helper function performs checks and asserts the required device type
        // then executes closure `f` on the specified device
        self.assert_has_device(dev_name);
        let dev = self.devices_().get_mut(dev_name).unwrap();
        assert!(
            dev.task_type() == task_type,
            "Device {} is incompatible with instruction",
            dev_name
        );
        f(dev)
    }

    /// Executes a specified operation (given by the closure `f`) on a targeted device without considering its `TaskType`.
    ///
    /// This method is a type-agnostic version of [`BaseExperiment::typed_device_op`]. It performs the same basic utility
    /// function but without asserting a specific `TaskType` for the device.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `f`: A closure that defines the operation to be performed on the device. It should accept a mutable reference to a `Device` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// If a device with the given `dev_name` doesn't exist.
    ///
    /// # Returns
    ///
    /// The return value of the closure `f`.
    fn device_op<F, R>(&mut self, dev_name: &str, mut f: F) -> R
    where
        F: FnMut(&mut Device) -> R,
    {
        // This helper function performs checks (existence of device) then performs closure)
        // Type-agnostic variant of typed_device_op
        self.assert_has_device(dev_name);
        let dev = self.devices_().get_mut(dev_name).unwrap();
        f(dev)
    }

    /// Executes a specified operation (given by the closure `f`) on a targeted channel of a specific device and `TaskType`.
    ///
    /// This utility method abstracts the common checks and operations performed on a channel. It ensures the device
    /// and its channel both exist, then checks if the device's task type matches the provided `task_type`, and
    /// finally, invokes the provided closure on the channel.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the parent device.
    /// * `chan_name`: The name of the target channel within the device.
    /// * `task_type`: The expected `TaskType` of the parent device.
    /// * `f`: A closure that defines the operation to be performed on the channel. It should accept a mutable reference to a `Channel` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// 1. If a device with the given `dev_name` doesn't exist.
    /// 2. If the channel with the given `chan_name` doesn't exist within the device.
    /// 3. If the device's task type doesn't match the provided `task_type`.
    ///
    /// # Returns
    ///
    /// The return value of the closure `f`.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot6", 1e6);
    /// exp.add_ao_channel("PXI1Slot6", 0);
    /// exp.typed_channel_op("PXI1Slot6", "ao0", TaskType::AO, |chan| {(*chan).constant(1., 0., 1., false)});
    /// assert_eq!(exp.typed_channel_op("PXI1Slot6", "ao0", TaskType::AO,
    ///             |chan| {(*chan).is_edited()}), true);
    /// ```
    fn typed_channel_op<F, R>(
        &mut self,
        dev_name: &str,
        chan_name: &str,
        task_type: TaskType,
        mut f: F,
    ) -> R
    where
        F: FnMut(&mut Channel) -> R,
    {
        // This helper function performs checks and asserts the required device type
        // then executes closure `f` on the specified channel
        self.assert_device_has_channel(dev_name, chan_name);
        let dev = self.devices_().get_mut(dev_name).unwrap();
        assert!(
            dev.task_type() == task_type,
            "Channel {}/{} is incompatible with instruction",
            dev_name,
            chan_name
        );
        let chan = dev.channels_().get_mut(chan_name).unwrap();
        f(chan)
    }

    /// Executes a specified operation (given by the closure `f`) on a targeted channel of a device without considering its `TaskType`.
    ///
    /// This method is a type-agnostic version of [`BaseExperiment::typed_channel_op`]. It abstracts away the common checks and operations
    /// performed on a channel without asserting a specific `TaskType` for the parent device.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the parent device.
    /// * `chan_name`: The name of the target channel within the device.
    /// * `f`: A closure that defines the operation to be performed on the channel. It should accept a mutable reference to a `Channel` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// 1. If a device with the given `dev_name` doesn't exist.
    /// 2. If the channel with the given `chan_name` doesn't exist within the device.
    ///
    /// # Returns
    ///
    /// The return value of the closure `f`.
    fn channel_op<F, R>(&mut self, dev_name: &str, chan_name: &str, mut f: F) -> R
    where
        F: FnMut(&mut Channel) -> R,
    {
        // Type-agnostic variant of typed_channel_op
        self.assert_device_has_channel(dev_name, chan_name);
        let chan = self
            .devices_()
            .get_mut(dev_name)
            .unwrap()
            .channels_()
            .get_mut(chan_name)
            .unwrap();
        f(chan)
    }

    /// Adds an analogue output (AO) channel to the designated device.
    ///
    /// This method leverages the [`BaseExperiment::typed_device_op`] function to forward
    /// the channel addition request to the specified device. Adds a channel of name `ao(channel_id)`
    /// to the designated device.
    ///
    /// Refer to the [`BaseDevice::add_channel`] method for detailed information on channel addition.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `channel_id`: The identifier for the AO channel to be added.
    ///
    /// # Panics
    ///
    /// This method will panic if the device with the provided `dev_name` is not of `TaskType::AO`.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot3", 1e6);
    /// exp.add_ao_channel("PXI1Slot3", 0);
    /// ```
    fn add_ao_channel(&mut self, dev_name: &str, channel_id: usize) {
        self.typed_device_op(dev_name, TaskType::AO, |dev| {
            (*dev).add_channel(&format!("ao{}", channel_id))
        });
    }

    /// Adds a digital output (DO) channel to the designated device.
    ///
    /// This method uses the [`BaseExperiment::typed_device_op`] function to forward
    /// the channel addition request to the specified device.
    ///
    /// For further details on how channels are added, see the [`BaseDevice::add_channel`] method.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `port_id`: The identifier for the digital port.
    /// * `line_id`: The identifier for the digital line within the port.
    ///
    /// # Panics
    ///
    /// This method will panic if the device with the provided `dev_name` is not of `TaskType::DO`.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e7);
    /// exp.add_do_channel("PXI1Slot6", 0, 0); // adds channel "port0/line0"
    /// ```
    fn add_do_channel(&mut self, dev_name: &str, port_id: usize, line_id: usize) {
        self.typed_device_op(dev_name, TaskType::DO, |dev| {
            (*dev).add_channel(&format!("port{}/line{}", port_id, line_id))
        });
    }

    /// Given interval and number of samples, calculates signal from specified device.
    ///
    /// This method uses the [`BaseExperiment::device_op`] to forward the calculation request
    /// to the target device. The calculation is based on the [`BaseDevice::calc_signal_nsamps`] method.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `start_pos`: The start position for the calculation.
    /// * `end_pos`: The end position for the calculation.
    /// * `nsamps`: The number of samples for the calculation.
    /// * `require_streamable`: Flag to indicate if the signal should be streamable.
    /// * `require_editable`: Flag to indicate if the signal should be editable.
    ///
    /// # Returns
    ///
    /// Returns an array of calculated signal samples for the specified device.    
    fn device_calc_signal_nsamps(
        &mut self,
        dev_name: &str,
        start_pos: usize,
        end_pos: usize,
        nsamps: usize,
        require_streamable: bool,
        require_editable: bool,
    ) -> Array2<f64> {
        self.device_op(dev_name, |dev| {
            (*dev).calc_signal_nsamps(
                start_pos,
                end_pos,
                nsamps,
                require_streamable,
                require_editable,
            )
        })
    }

    /// Configures the sample clock source of a device in the experiment.
    ///
    /// This method retrieves the specified device and delegates the configuration
    /// of the sample clock source to its base method [`BaseDevice::cfg_samp_clk_src`].
    ///
    /// # Arguments
    ///
    /// * `dev_name` - The name of the device to configure.
    /// * `src` - The name of the sample clock source.
    ///
    /// See also: [`BaseDevice::cfg_samp_clk_src`]
    fn device_cfg_samp_clk_src(&mut self, dev_name: &str, src: &str) {
        self.device_op(dev_name, |dev| (*dev).cfg_samp_clk_src(src))
    }

    /// Configures the trigger settings of a device in the experiment while ensuring synchronization.
    ///
    /// Before delegating the configuration to its base method [`BaseDevice::cfg_trig`], this method
    /// performs a synchronization check to ensure:
    ///
    /// If the current device is set to export a trigger (`export_trig` is `true`), then no other device
    /// in the experiment should already be exporting a trigger (`export_trig` should be `None` for all other devices).
    ///
    /// The experiment can only have one device that exports triggers at any given time.
    ///
    /// # Arguments
    ///
    /// * `dev_name` - The name of the device to configure.
    /// * `trig_line` - The trigger line identifier.
    /// * `export_trig` - A boolean that determines whether to export or import the trigger.
    ///
    /// # Panics
    ///
    /// This method will panic if the synchronization condition related to a device exporting triggers is violated.
    ///
    /// See also: [`BaseDevice::cfg_trig`]
    fn device_cfg_trig(&mut self, dev_name: &str, trig_line: &str, export_trig: bool) {
        assert!(
            !export_trig
                || (export_trig
                    && self
                        .devices()
                        .values()
                        .all(|dev| dev.export_trig().is_none())),
            "Device {} cannot export triggers since another device already exports triggers.",
            dev_name
        );
        self.device_op(dev_name, |dev| (*dev).cfg_trig(trig_line, export_trig))
    }

    /// Configures the reference clock settings of a device in the experiment.
    ///
    /// This method retrieves the specified device and delegates the configuration
    /// of the reference clock settings to its base method [`BaseDevice::cfg_ref_clk`].
    ///
    /// # Arguments
    ///
    /// * `dev_name` - The name of the device to configure.
    /// * `ref_clk_line` - The line or channel to import or export the device's reference clock.
    /// * `ref_clk_rate` - The rate of the reference clock in Hz.
    /// * `export_ref_clk` - A boolean that determines whether to export (if `true`) or import (if `false`) the reference clock.
    ///
    /// See also: [`BaseDevice::cfg_ref_clk`]
    fn device_cfg_ref_clk(
        &mut self,
        dev_name: &str,
        ref_clk_line: &str,
        ref_clk_rate: f64,
        export_ref_clk: bool,
    ) {
        self.device_op(dev_name, |dev| {
            (*dev).cfg_ref_clk(ref_clk_line, ref_clk_rate, export_ref_clk)
        })
    }

    /// Retrieves the `edit_stop_time` for a specific device.
    ///
    /// This method employs the [`BaseExperiment::device_op`] function to request
    /// the `edit_stop_time` from the given device.
    ///
    /// For details on how the stop time is determined, refer to the [`BaseDevice::edit_stop_time`] method.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    ///
    /// # Returns
    ///
    /// Returns the edit stop time for the specified device.
    fn device_edit_stop_time(&mut self, dev_name: &str) -> f64 {
        self.device_op(dev_name, |dev| (*dev).edit_stop_time())
    }

    /// Retrieves the maximum `compiled_stop_time` from all registered devices.
    ///
    /// This method determines the longest stop time across all devices
    /// that have been compiled, which may be useful for synchronization purposes.
    ///
    /// # Returns
    ///
    /// The maximum `compiled_stop_time` across all devices.
    ///
    /// See [`BaseDevice::compiled_stop_time`] for more details on individual device stop times.
    fn device_compiled_stop_time(&mut self, dev_name: &str) -> f64 {
        self.device_op(dev_name, |dev| (*dev).compiled_stop_time())
    }

    /// Clears the compilation cache for a specific device.
    ///
    /// Utilizing the [`BaseExperiment::device_op`] function, this method forwards the request
    /// to clear the compilation cache to the specified device. Also see [`BaseDevice::clear_compile_cache`].
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e7);
    /// // ... other operations ...
    /// exp.device_clear_compile_cache("PXI1Slot6");
    /// ```
    fn device_clear_compile_cache(&mut self, dev_name: &str) {
        self.device_op(dev_name, |dev| (*dev).clear_compile_cache())
    }

    /// Clears the edit cache for a specific device.
    ///
    /// This method, using the [`BaseExperiment::device_op`] function, forwards the request
    /// to clear the edit cache to the designated device. Also see [`BaseDevice::clear_edit_cache`].
    ///
    /// The edit cache holds intermediate results during the editing phase.
    /// Clearing it can be beneficial when making significant changes to the device's configuration or to free up memory.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e7);
    /// // ... other operations ...
    /// exp.device_clear_edit_cache("PXI1Slot6");
    /// ```
    fn device_clear_edit_cache(&mut self, dev_name: &str) {
        self.device_op(dev_name, |dev| (*dev).clear_edit_cache())
    }

    /// Retrieves the names of compiled channels from the specified device based on the given requirements.
    ///
    /// This method fetches the names of all channels from the designated device that have been compiled
    /// and meet the criteria specified by `require_streamable` and `require_editable`: see
    /// [`BaseDevice::compiled_channels`]. The order of the
    /// returned names will match that in [`BaseExperiment::device_calc_signal_nsamps`].
    /// Set `require_editable=true` to see the signals as they are written into the experiment object.
    /// Set `require_streamable=true` to see signals from channels as they are written in to the NI-DAQmx driver.
    /// Also see the `channel` module on streamable and editable channel properties.
    ///
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `require_streamable`: If set to `true`, only channels that are streamable will be considered.
    /// * `require_editable`: If set to `true`, only channels that are editable will be considered.
    ///
    /// # Returns
    ///
    /// A vector of strings, each representing the name of a channel that matches the given criteria.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6,);
    /// exp.add_do_channel("PXI1Slot6", 0, 0);
    /// exp.add_do_channel("PXI1Slot6", 2, 0);
    /// exp.add_do_channel("PXI1Slot6", 2, 1);
    /// exp.go_high("PXI1Slot6", "port0/line0", 0.);
    /// exp.go_high("PXI1Slot6", "port2/line0", 1.);
    /// exp.go_high("PXI1Slot6", "port2/line1", 2.);
    /// exp.compile_with_stoptime(3.);
    /// let compiled_streamable_channels = exp.device_compiled_channel_names("PXI1Slot6", true, false);
    /// // 2 strealable channels: "port0" and "port2"
    /// assert_eq!(compiled_streamable_channels.len(), 2);
    /// // 3 editable channels: "port0/line0", "port2/line0", "port2/line1"
    /// let compiled_editable_channels = exp.device_compiled_channel_names("PXI1Slot6", false, true);
    /// assert_eq!(compiled_editable_channels.len(), 3);
    /// ```
    fn device_compiled_channel_names(
        &mut self,
        dev_name: &str,
        require_streamable: bool,
        require_editable: bool,
    ) -> Vec<String> {
        self.device_op(dev_name, |dev| {
            (*dev)
                .compiled_channels(require_streamable, require_editable)
                .iter()
                .map(|chan| chan.physical_name().to_string())
                .collect()
        })
    }

    /// Adds a constant value instruction to the specified analogue output (AO) channel.
    ///
    /// This method leverages the [`BaseExperiment::typed_channel_op`] function to forward the constant value
    /// instruction request to the targeted AO channel using the [`BaseChannel::constant`] method.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target AO channel within the device.
    /// * `t`: The start time of the constant instruction.
    /// * `duration`: Duration for which the constant value is applied.
    /// * `value`: The constant value to apply.
    /// * `keep_val`: Flag indicating whether to maintain the value beyond the specified duration.
    ///
    /// # Panics
    ///
    /// This method will panic if the provided channel is not of type AO.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6,);
    /// exp.add_do_channel("PXI1Slot6", 0, 0);
    /// exp.add_do_channel("PXI1Slot6", 2, 0);
    /// exp.add_do_channel("PXI1Slot6", 2, 1);
    /// exp.go_high("PXI1Slot6", "port0/line0", 0.);
    /// exp.go_high("PXI1Slot6", "port2/line0", 1.);
    /// exp.go_high("PXI1Slot6", "port2/line1", 2.);
    /// exp.compile_with_stoptime(3.);
    /// let compiled_streamable_channels = exp.device_compiled_channel_names("PXI1Slot6", true, false);
    /// // 2 strealable channels: "port0" and "port2"
    /// assert_eq!(compiled_streamable_channels.len(), 2);
    /// // 3 editable channels: "port0/line0", "port2/line0", "port2/line1"
    /// let compiled_editable_channels = exp.device_compiled_channel_names("PXI1Slot6", false, true);
    /// assert_eq!(compiled_editable_channels.len(), 3);
    /// ```
    fn constant(
        &mut self,
        dev_name: &str,
        chan_name: &str,
        t: f64,
        duration: f64,
        value: f64,
        keep_val: bool,
    ) {
        self.typed_channel_op(dev_name, chan_name, TaskType::AO, |chan| {
            (*chan).constant(value, t, duration, keep_val);
        });
    }

    /// Adds a sine waveform instruction to the specified analogue output (AO) channel.
    ///
    /// This method uses the [`BaseExperiment::typed_channel_op`] function to relay the sine instruction
    /// request to the appropriate AO channel via the [`BaseChannel::add_instr`] method.
    /// See [`Instruction::new_sine`] for more detailed explanation of the sine arguments.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target AO channel within the device.
    /// * `t`: The start time of the sine instruction.
    /// * `duration`: Duration of the sine waveform.
    /// * `keep_val`: Flag indicating whether to maintain the waveform's value beyond the specified duration.
    /// * `freq`: Frequency of the sine waveform.
    /// * `amplitude`: Optional amplitude of the sine waveform.
    /// * `phase`: Optional phase shift for the sine waveform.
    /// * `dc_offset`: Optional DC offset for the sine waveform.
    ///
    /// # Panics
    ///
    /// This method will panic if the designated channel is not of type AO.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot3", 1e6,);
    /// exp.add_ao_channel("PXI1Slot3", 0);
    /// // t=0, duration=1, keep_val=false, freq=10Hz, amplitude=10, phase=0(default), dc_offset=0(default)
    /// exp.sine("PXI1Slot3", "ao0", 0., 1., false, 10., Some(10.), None, None);
    /// ```
    fn sine(
        &mut self,
        dev_name: &str,
        chan_name: &str,
        t: f64,
        duration: f64,
        keep_val: bool,
        freq: f64,
        amplitude: Option<f64>,
        phase: Option<f64>,
        dc_offset: Option<f64>,
    ) {
        self.typed_channel_op(dev_name, chan_name, TaskType::AO, |chan| {
            let instr = Instruction::new_sine(freq, amplitude, phase, dc_offset);
            (*chan).add_instr(instr, t, duration, keep_val)
        });
    }

    /// Sets the specified digital output (DO) channel to a high state for the given duration.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target DO channel within the device.
    /// * `t`: The start time for the high state.
    /// * `duration`: Duration for which the channel remains high.
    ///
    /// # Panics
    ///
    /// This method will panic if the channel is not of type DO.
    fn high(&mut self, dev_name: &str, chan_name: &str, t: f64, duration: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(1., t, duration, false);
        });
    }

    /// Sets the specified digital output (DO) channel to a low state for the given duration.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target DO channel within the device.
    /// * `t`: The start time for the low state.
    /// * `duration`: Duration for which the channel remains low.
    ///
    /// # Panics
    ///
    /// This method will panic if the channel is not of type DO.
    fn low(&mut self, dev_name: &str, chan_name: &str, t: f64, duration: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(0., t, duration, false);
        });
    }

    /// Sets the specified digital output (DO) channel to a high state, until the next instruction.
    ///
    /// The duration is determined as the inverse of the channel's sampling rate. A `go_high` instruction
    /// is a one-tick low pulse which keeps its value.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target DO channel within the device.
    /// * `t`: The start time for the high pulse.
    ///
    /// # Panics
    ///
    /// This method will panic if the channel is not of type DO.
    fn go_high(&mut self, dev_name: &str, chan_name: &str, t: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(1., t, 1. / (*chan).samp_rate(), true);
        });
    }

    /// Sets the specified digital output (DO) channel to a low state for a short duration.
    ///
    /// The duration is determined as the inverse of the channel's sampling rate. A `go_low` instruction
    /// translates to a one-tick low pulse which keeps its value
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target DO channel within the device.
    /// * `t`: The start time for the low pulse.
    ///
    /// # Panics
    ///
    /// This method will panic if the channel is not of type DO.
    fn go_low(&mut self, dev_name: &str, chan_name: &str, t: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(0., t, 1. / (*chan).samp_rate(), true);
        });
    }

    /// Clears the edit cache of the specified channel.
    ///
    /// This method resets the channel to its pre-edit state. Clearing the edit cache can be helpful
    /// when a sequence of edits needs to be discarded without affecting the compiled state of the channel.
    /// Also see [`BaseExperiment::channel_op`] and [`BaseChannel::clear_edit_cache`].
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target channel within the device.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot7", 1e6);
    /// exp.add_do_channel("PXI1Slot7", 0, 7);
    /// exp.go_high("PXI1Slot7", "port0/line7", 0.);
    /// assert_eq!(exp.is_fresh_compiled(), false);
    /// exp.channel_clear_edit_cache("PXI1Slot7", "port0/line7");
    /// assert_eq!(exp.is_fresh_compiled(), true);
    /// ```
    fn channel_clear_edit_cache(&mut self, dev_name: &str, chan_name: &str) {
        self.channel_op(dev_name, chan_name, |chan| (*chan).clear_edit_cache());
    }

    /// Calculates the sampled signal for a given channel over a specified time interval.
    ///
    /// The function computes the signal values based on the given start and end times,
    /// along with the number of samples desired.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the device associated with the channel.
    /// * `chan_name`: The name of the channel for which the signal is calculated.
    /// * `start_time`: The starting time of the sampling interval (in seconds).
    /// * `end_time`: The ending time of the sampling interval (in seconds).
    /// * `num_samps`: The number of samples to be computed over the specified interval.
    ///
    /// # Returns
    ///
    /// Returns a vector of `f64` containing the signal values sampled over the interval.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot7", 1e6);
    /// exp.add_do_channel("PXI1Slot7", 0, 7);
    /// exp.go_high("PXI1Slot7", "port0/line7", 0.5);
    /// exp.compile_with_stoptime(1.);
    /// let sig = exp.channel_calc_signal_nsamps("PXI1Slot7", "port0/line7", 0., 1., 10);
    /// assert_eq!(sig[0], 0.);
    /// assert_eq!(sig[sig.len() - 1], 1.);
    /// ```
    fn channel_calc_signal_nsamps(
        &mut self,
        dev_name: &str,
        chan_name: &str,
        start_time: f64,
        end_time: f64,
        num_samps: usize,
    ) -> Vec<f64> {
        self.channel_op(dev_name, chan_name, |chan| {
            (*chan).calc_signal_nsamps(start_time, end_time, num_samps)
        })
    }

    /// Clears the compile cache of the specified channel.
    ///
    /// By invoking this method, any compiled data related to the channel will be removed. This is useful when
    /// the compiled state of a channel needs to be invalidated, such as after a series of edits or changes.
    /// Also see [`BaseExperiment::channel_op`] and [`BaseChannel::clear_compile_cache`].
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target channel within the device.
    ///
    /// # Example
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot7", 1e6);
    /// exp.add_do_channel("PXI1Slot7", 0, 7);
    /// exp.go_high("PXI1Slot7", "port0/line7", 0.);
    /// exp.compile();
    /// assert_eq!(exp.is_compiled(), true);
    /// exp.channel_clear_compile_cache("PXI1Slot7", "port0/line7");
    /// assert_eq!(exp.is_compiled(), false);
    /// ```
    fn channel_clear_compile_cache(&mut self, dev_name: &str, chan_name: &str) {
        self.channel_op(dev_name, chan_name, |chan| (*chan).clear_compile_cache());
    }
}

/// A concrete struct consisting of a collection of devices.
///
/// **Refer to the [`BaseExperiment`] trait for method behavior.**
#[pyclass]
pub struct Experiment {
    devices: HashMap<String, Device>,
}

/// A macro to generate boilerplate implementations for structs representing experiments.
///
/// This macro assists in the conversion between Rust's trait system and Python's class system.
/// Given that PyO3 doesn't support exposing trait methods directly to Python, this macro wraps
/// each [`BaseExperiment`] trait method with a direct implementation, facilitating its export to Python.
///
/// The majority of methods are exported with their arguments and types preserved.
/// Any deviations from this convention should be explicitly noted and elaborated upon.
///
/// Usage:
/// ```rust
/// use nicompiler_backend::device::*;
/// use nicompiler_backend::channel::*;
/// use nicompiler_backend::*;
/// use pyo3::prelude::*;
/// use std::collections::HashMap;
///
/// #[pyclass]
/// struct CustomExperiment {
///     devices: HashMap<String, Device>,
///     some_property: f64,
/// }
/// impl_exp_boilerplate!(CustomExperiment);
///
/// // Implement additional methods which can be exposed to python
/// #[pymethods]
/// impl CustomExperiment {
///     #[new]
///     pub fn new(some_property: f64) -> Self {
///         Self {
///             devices: HashMap::new(),
///             some_property
///         }
///     }
/// }
/// ```
///
/// This will generate the required implementations and additional Python bindings for `CustomExperiment`.

#[macro_export]
macro_rules! impl_exp_boilerplate {
    ($exp_type: ty) => {
        impl BaseExperiment for $exp_type {
            fn devices(&self) -> &HashMap<String, Device> {
                &self.devices
            }
            fn devices_(&mut self) -> &mut HashMap<String, Device> {
                &mut self.devices
            }
        }

        #[pymethods]
        impl $exp_type {
            fn add_ao_device(&mut self, physical_name: &str, samp_rate: f64) {
                BaseExperiment::add_ao_device(self, physical_name, samp_rate);
            }

            fn add_do_device(&mut self, physical_name: &str, samp_rate: f64) {
                BaseExperiment::add_do_device(self, physical_name, samp_rate);
            }

            pub fn edit_stop_time(&self) -> f64 {
                BaseExperiment::edit_stop_time(self)
            }

            pub fn compiled_stop_time(&self) -> f64 {
                BaseExperiment::compiled_stop_time(self)
            }

            pub fn compile(&mut self) -> f64 {
                BaseExperiment::compile(self)
            }

            pub fn compile_with_stoptime(&mut self, stop_time: f64) {
                BaseExperiment::compile_with_stoptime(self, stop_time);
            }

            pub fn is_edited(&self) -> bool {
                BaseExperiment::is_edited(self)
            }

            pub fn is_compiled(&self) -> bool {
                BaseExperiment::is_compiled(self)
            }

            pub fn is_fresh_compiled(&self) -> bool {
                BaseExperiment::is_fresh_compiled(self)
            }

            pub fn clear_edit_cache(&mut self) {
                BaseExperiment::clear_edit_cache(self);
            }

            pub fn clear_compile_cache(&mut self) {
                BaseExperiment::clear_compile_cache(self);
            }

            // DEVICE METHODS
            pub fn add_ao_channel(&mut self, dev_name: &str, channel_id: usize) {
                BaseExperiment::add_ao_channel(self, dev_name, channel_id);
            }

            pub fn add_do_channel(&mut self, dev_name: &str, port_id: usize, line_id: usize) {
                BaseExperiment::add_do_channel(self, dev_name, port_id, line_id);
            }

            pub fn device_cfg_samp_clk_src(&mut self, dev_name: &str, src: &str) {
                BaseExperiment::device_cfg_samp_clk_src(self, dev_name, src);
            }

            pub fn device_cfg_trig(&mut self, dev_name: &str, trig_line: &str, export_trig: bool) {
                BaseExperiment::device_cfg_trig(self, dev_name, trig_line, export_trig);
            }

            pub fn device_cfg_ref_clk(
                &mut self,
                dev_name: &str,
                ref_clk_line: &str,
                ref_clk_rate: f64,
                export_ref_clk: bool,
            ) {
                BaseExperiment::device_cfg_ref_clk(
                    self,
                    dev_name,
                    ref_clk_line,
                    ref_clk_rate,
                    export_ref_clk,
                );
            }

            pub fn device_compiled_channel_names(
                &mut self,
                dev_name: &str,
                require_streamable: bool,
                require_editable: bool,
            ) -> Vec<String> {
                BaseExperiment::device_compiled_channel_names(
                    self,
                    dev_name,
                    require_streamable,
                    require_editable,
                )
            }

            pub fn calc_signal(
                &mut self,
                dev_name: &str,
                t_start: f64,
                t_end: f64,
                nsamps: usize,
                require_streamable: bool,
                require_editable: bool,
                py: Python,
            ) -> PyResult<PyObject> {
                self.assert_has_device(dev_name);
                let samp_rate = self.devices().get(dev_name).unwrap().samp_rate();
                let arr = BaseExperiment::device_calc_signal_nsamps(
                    self,
                    dev_name,
                    (t_start * samp_rate) as usize,
                    (t_end * samp_rate) as usize,
                    nsamps,
                    require_streamable,
                    require_editable,
                );
                Ok(numpy::PyArray::from_array(py, &arr).to_object(py))
            }

            pub fn device_edit_stop_time(&mut self, dev_name: &str) -> f64 {
                BaseExperiment::device_edit_stop_time(self, dev_name)
            }

            pub fn device_compiled_stop_time(&mut self, dev_name: &str) -> f64 {
                BaseExperiment::device_compiled_stop_time(self, dev_name)
            }

            pub fn device_clear_compile_cache(&mut self, dev_name: &str) {
                BaseExperiment::device_clear_compile_cache(self, dev_name)
            }

            pub fn device_clear_edit_cache(&mut self, dev_name: &str) {
                BaseExperiment::device_clear_edit_cache(self, dev_name)
            }

            // INSTRUCTION METHODS
            pub fn constant(
                &mut self,
                dev_name: &str,
                chan_name: &str,
                t: f64,
                duration: f64,
                value: f64,
                keep_val: bool,
            ) {
                BaseExperiment::constant(self, dev_name, chan_name, t, duration, value, keep_val);
            }

            pub fn sine(
                &mut self,
                dev_name: &str,
                chan_name: &str,
                t: f64,
                duration: f64,
                keep_val: bool,
                freq: f64,
                amplitude: Option<f64>,
                phase: Option<f64>,
                dc_offset: Option<f64>,
            ) {
                BaseExperiment::sine(
                    self, dev_name, chan_name, t, duration, keep_val, freq, amplitude, phase,
                    dc_offset,
                );
            }

            pub fn high(&mut self, dev_name: &str, chan_name: &str, t: f64, duration: f64) {
                BaseExperiment::high(self, dev_name, chan_name, t, duration);
            }

            pub fn low(&mut self, dev_name: &str, chan_name: &str, t: f64, duration: f64) {
                BaseExperiment::low(self, dev_name, chan_name, t, duration);
            }

            pub fn go_high(&mut self, dev_name: &str, chan_name: &str, t: f64) {
                BaseExperiment::go_high(self, dev_name, chan_name, t);
            }

            pub fn go_low(&mut self, dev_name: &str, chan_name: &str, t: f64) {
                BaseExperiment::go_low(self, dev_name, chan_name, t);
            }

            pub fn channel_clear_compile_cache(&mut self, dev_name: &str, chan_name: &str) {
                BaseExperiment::channel_clear_compile_cache(self, dev_name, chan_name);
            }

            pub fn channel_clear_edit_cache(&mut self, dev_name: &str, chan_name: &str) {
                BaseExperiment::channel_clear_edit_cache(self, dev_name, chan_name);
            }

            pub fn channel_calc_signal_nsamps(
                &mut self,
                dev_name: &str,
                chan_name: &str,
                start_time: f64,
                end_time: f64,
                num_samps: usize,
            ) -> Vec<f64> {
                BaseExperiment::channel_calc_signal_nsamps(
                    self, dev_name, chan_name, start_time, end_time, num_samps,
                )
            }
        }
    };
}

#[pymethods]
impl Experiment {
    /// Constructor for the `Experiment` class.
    ///
    /// This constructor initializes an instance of the `Experiment` class with an empty collection of devices.
    /// The underlying representation of this collection is a hashmap where device names (strings) map to their
    /// respective `Device` objects.
    ///
    /// # Returns
    /// - An `Experiment` instance with no associated devices.
    ///
    /// # Example (python)
    /// ```python
    /// from nicompiler_backend import Experiment
    ///
    /// exp = Experiment()
    /// assert len(exp.devices()) == 0
    /// ```
    #[new]
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }
}

impl_exp_boilerplate!(Experiment);
