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
use indexmap::IndexMap;

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
///     - [`device_last_instr_end_time`], [`device_compiled_stop_time`]
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
/// [`device_last_instr_end_time`]: BaseExperiment::device_last_instr_end_time
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
    fn devices(&self) -> &IndexMap<String, Device>;
    fn devices_(&mut self) -> &mut IndexMap<String, Device>;

    /// Asserts that the specified device exists in the experiment.
    ///
    /// This function checks if the provided device name is present within the collection
    /// of devices in the current experiment. If the device is not found, it triggers an
    /// assertion failure with a descriptive error message indicating the missing device
    /// and a list of all registered devices.
    ///
    /// # Arguments
    ///
    /// * `name`: Name of the device to check.
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
    fn assert_has_device(&self, name: &str) {
        assert!(
            self.devices().contains_key(name),
            "Physical device {} not found. Registered devices are {:?}",
            name,
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
    /// * `name`: Name of the device to look into.
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
    /// exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
    /// exp.assert_device_has_channel("PXI1Slot6", "port0/line0");
    ///
    /// // This will panic
    /// // exp.assert_device_has_channel("PXI1Slot6", "port0/line1");
    /// ```
    ///
    /// [`assert_has_device`]: BaseExperiment::assert_has_device
    fn assert_device_has_channel(&self, name: &str, chan_name: &str) {
        self.assert_has_device(name);
        let device = self.devices().get(name).unwrap();
        assert!(
            device.channels().contains_key(chan_name),
            "Channel name {} not found in device {}. Registered channels are: {:?}",
            chan_name,
            name,
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
        let name = dev.name();
        assert!(
            !self.devices().contains_key(name),
            "Device {} already registered. Registered devices are {:?}",
            name,
            self.devices().keys().collect::<Vec<_>>()
        );
        self.devices_().insert(name.to_string(), dev);
    }

    /// Registers an Analog Output (AO) device to the experiment.
    ///
    /// This method creates an AO device with the specified parameters and adds it to the
    /// experiment's collection of devices using the [`BaseExperiment::add_device_base`] method.
    ///
    /// # Arguments
    ///
    /// * `name`: A string slice that holds the name of the AO device.
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
    fn add_ao_device(&mut self, name: &str, samp_rate: f64) {
        self.add_device_base(Device::new(name, TaskType::AO, samp_rate));
    }

    /// Registers a Digital Output (DO) device to the experiment.
    ///
    /// This method creates a DO device with the specified parameters and registers it to the experiment.
    ///
    /// # Arguments
    ///
    /// * `name`: A string slice that holds the name of the DO device.
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
    fn add_do_device(&mut self, name: &str, samp_rate: f64) {
        self.add_device_base(Device::new(name, TaskType::DO, samp_rate));
    }

    /// Shortcut to borrow device instance by name
    fn dev(&self, name: &str) -> &Device {
        if !self.devices().contains_key(name) {
            panic!("There is no device {name} registered")
        }
        self.devices().get(name).unwrap()
    }
    /// Shortcut to mutably borrow device instance by name
    fn dev_(&mut self, name: &str) -> &mut Device {
        if !self.devices().contains_key(name) {
            panic!("There is no device {name} registered")
        }
        self.devices_().get_mut(name).unwrap()
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
    /// exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
    /// exp.add_do_channel("PXI1Slot6", 0, 4, 0.);
    /// exp.high("PXI1Slot6", "port0/line0", 1., 4.); // stop time at 5
    /// assert_eq!(exp.edit_stop_time(), 5.);
    /// exp.high("PXI1Slot6", "port0/line4", 0., 6.); // stop time at 6
    /// assert_eq!(exp.edit_stop_time(), 6.);
    /// ```
    fn last_instr_end_time(&self) -> f64 {
        self.devices()
            .values()
            .map(|dev| dev.last_instr_end_time())
            .fold(0.0, f64::max)
    }

    /// Retrieves the `total_run_time` from all registered devices.
    /// See [`BaseDevice::total_run_time`] for more information.
    ///
    /// The maximum `total_run_time` across all devices.
    fn total_run_time(&self) -> f64 {
        self.devices()
            .values()
            .map(|dev| dev.total_run_time())
            .fold(0.0, f64::max)
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
    /// exp.device_set_start_trig_term("PXI1Slot6", "PXI_Trig0");
    /// // This will panic as there are no primary devices, but PXI1Slot6 is expecting a trigger source
    /// exp.compile_with_stoptime(10.0);
    /// ```
    ///
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6);
    /// exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
    /// exp.high("PXI1Slot6", "port0/line0", 1., 3.);
    ///
    /// exp.compile(false);
    /// assert_eq!(exp.compiled_stop_time(), 4.);
    /// exp.compile_with_stoptime(5.); // Experiment signal will stop at t=5 now
    /// assert_eq!(exp.compiled_stop_time(), 5.);
    /// ```
    fn compile(&mut self, stop_time: Option<f64>) -> f64 {
        let stop_time = match stop_time {
            Some(stop_time) => {
                if stop_time < self.last_instr_end_time() {
                    panic!(
                        "Attempted to compile with stop_time={stop_time} [s] while the last instruction end time is {} [s]\n\
                        If you intended to provide stop_time=last_instr_end_time, use stop_time=None",
                        self.last_instr_end_time()
                    )
                };
                stop_time
            },
            None => self.last_instr_end_time()
        };
        for dev in self.devices_().values_mut() {
            dev.compile(stop_time);
        }
        return self.total_run_time()
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
        self.clear_compile_cache();
        self.devices_()
            .values_mut()
            .for_each(|dev| dev.clear_edit_cache());
    }

    /// Adds a reset tick with value of 0 across all editable channels of the experiment.
    ///
    /// This function computes the last `edit_stop_time` of the experiment and uses it
    /// to determine the appropriate time to insert the reset tick. A reset tick is a
    /// point in time where all editable channels are reset to a value of 0.
    ///
    /// # Arguments
    ///
    /// * `t`: if `Some(t)`, time point at which to insert the all-channel reset instruction.
    /// If `None`, reset is inserted at the latest end of all existing instructions across all channels.
    ///
    /// # Returns
    ///
    /// Returns the time at which the reset tick was added. This time corresponds to
    /// the earliest unspecified interval across all channels after the last `edit_stop_time`.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal operations fail, such as accessing 
    /// non-existent channels or devices. Ensure that devices and channels are properly 
    /// set up before calling this function.
    ///
    /// # Notes
    ///
    /// It's recommended to call `compile` after using this function 
    /// to ensure that the newly added reset ticks are taken into account in the compiled output.
    /// 
    /// # Example
    /// 
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// // Define devices and associated channels
    /// exp.add_do_device("PXI1Slot6", 10.);
    /// exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
    /// exp.add_do_channel("PXI1Slot6", 0, 1, 0.);
    ///
    /// exp.high("PXI1Slot6", "port0/line0", 0., 1.);
    /// exp.go_high("PXI1Slot6", "port0/line1", 0.);
    /// exp.compile_with_stoptime(5.);
    ///
    /// // Calculate from t=0 ~ 5
    /// let sig = exp.device_calc_signal_nsamps("PXI1Slot6", 0, 50, 50, false, true);
    /// assert!(sig[[0, 9]] == 1. && sig[[0, 10]] == 0.); // go_high takes effect on the tick corresponding to specified time. 
    /// assert!(sig[[1, 9]] == 1. && sig[[1, 10]] == 1.); 
    /// 
    /// let reset_tick_time = exp.add_reset_instr();
    /// // Reset tick happens at the earliest unspecified interval across all channels
    /// assert!(reset_tick_time == 1.0); 
    /// exp.compile_with_stoptime(5.);
    /// let sig = exp.device_calc_signal_nsamps("PXI1Slot6", 0, 50, 50, false, true);
    /// assert!(sig[[0, 9]] == 1. && sig[[0, 10]] == 0.); 
    /// assert!(sig[[1, 9]] == 1. && sig[[1, 10]] == 0.); // Also zeros channel 1 at t=1
    /// // println!("{:?}, reset_tick_time={}", sig, reset_tick_time);
    /// ```
    fn add_reset_instr(&mut self, reset_time: Option<f64>) {
        let last_instr_end_time = self.last_instr_end_time();
        let reset_time = match reset_time {
            Some(reset_time) => {
                if reset_time < last_instr_end_time {
                    panic!(
                        "Requested to insert the all-channel reset instruction at t = {reset_time} [s] \
                        but some channels have instructions spanning until {last_instr_end_time} [s].\n\
                        If you intended to provide `reset_time=last_instr_end_time`, use `reset_time=None`"
                    )
                }
                reset_time
            },
            None => last_instr_end_time
        };
        for dev in self.devices_().values_mut() {
            dev.add_reset_instr(reset_time)
        }
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
    /// * `name`: The name of the target device.
    /// * `task_type`: The expected `TaskType` of the device.
    /// * `f`: A closure that defines the operation to be performed on the device. It should accept a mutable reference to a `Device` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// 1. If a device with the given `name` doesn't exist.
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
    fn typed_device_op<F, R>(&mut self, name: &str, task_type: TaskType, mut f: F) -> R
    where
        F: FnMut(&mut Device) -> R,
    {
        // This helper function performs checks and asserts the required device type
        // then executes closure `f` on the specified device
        self.assert_has_device(name);
        let dev = self.devices_().get_mut(name).unwrap();
        assert!(
            dev.task_type() == task_type,
            "Device {} is incompatible with instruction",
            name
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
    /// * `name`: The name of the target device.
    /// * `f`: A closure that defines the operation to be performed on the device. It should accept a mutable reference to a `Device` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// If a device with the given `name` doesn't exist.
    ///
    /// # Returns
    ///
    /// The return value of the closure `f`.
    fn device_op<F, R>(&mut self, name: &str, mut f: F) -> R
    where
        F: FnMut(&mut Device) -> R,
    {
        // This helper function performs checks (existence of device) then performs closure)
        // Type-agnostic variant of typed_device_op
        self.assert_has_device(name);
        let dev = self.devices_().get_mut(name).unwrap();
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
    /// * `name`: The name of the parent device.
    /// * `chan_name`: The name of the target channel within the device.
    /// * `task_type`: The expected `TaskType` of the parent device.
    /// * `f`: A closure that defines the operation to be performed on the channel. It should accept a mutable reference to a `Channel` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// 1. If a device with the given `name` doesn't exist.
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
    /// exp.add_ao_channel("PXI1Slot6", 0, 0.);
    /// exp.typed_channel_op("PXI1Slot6", "ao0", TaskType::AO, |chan| {(*chan).constant(1., 0., 1., false)});
    /// assert_eq!(exp.typed_channel_op("PXI1Slot6", "ao0", TaskType::AO,
    ///             |chan| {(*chan).is_edited()}), true);
    /// ```
    fn typed_channel_op<F, R>(
        &mut self,
        name: &str,
        chan_name: &str,
        task_type: TaskType,
        mut f: F,
    ) -> R
    where
        F: FnMut(&mut Channel) -> R,
    {
        // This helper function performs checks and asserts the required device type
        // then executes closure `f` on the specified channel
        self.assert_device_has_channel(name, chan_name);
        let dev = self.devices_().get_mut(name).unwrap();
        assert!(
            dev.task_type() == task_type,
            "Channel {}/{} is incompatible with instruction",
            name,
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
    /// * `name`: The name of the parent device.
    /// * `chan_name`: The name of the target channel within the device.
    /// * `f`: A closure that defines the operation to be performed on the channel. It should accept a mutable reference to a `Channel` and return a value of type `R`.
    ///
    /// # Panics
    ///
    /// 1. If a device with the given `name` doesn't exist.
    /// 2. If the channel with the given `chan_name` doesn't exist within the device.
    ///
    /// # Returns
    ///
    /// The return value of the closure `f`.
    fn channel_op<F, R>(&mut self, name: &str, chan_name: &str, mut f: F) -> R
    where
        F: FnMut(&mut Channel) -> R,
    {
        // Type-agnostic variant of typed_channel_op
        self.assert_device_has_channel(name, chan_name);
        let chan = self
            .devices_()
            .get_mut(name)
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
    /// * `name`: The name of the target device.
    /// * `channel_id`: The identifier for the AO channel to be added.
    ///
    /// # Panics
    ///
    /// This method will panic if the device with the provided `name` is not of `TaskType::AO`.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot3", 1e6);
    /// exp.add_ao_channel("PXI1Slot3", 0, 0.);
    /// ```
    fn add_ao_channel(&mut self, name: &str, channel_id: usize, default_value: f64) {
        self.typed_device_op(name, TaskType::AO, |dev| {
            (*dev).add_channel(&format!("ao{}", channel_id), default_value)
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
    /// * `name`: The name of the target device.
    /// * `port_id`: The identifier for the digital port.
    /// * `line_id`: The identifier for the digital line within the port.
    ///
    /// # Panics
    ///
    /// This method will panic if the device with the provided `name` is not of `TaskType::DO`.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e7);
    /// exp.add_do_channel("PXI1Slot6", 0, 0, 0.); // adds channel "port0/line0"
    /// ```
    fn add_do_channel(&mut self, name: &str, port_id: usize, line_id: usize, default_value: f64) {
        assert!(default_value == 0. || default_value == 1., 
            "Expected default value 0 or 1 for device {} DO channel {} but received {}", name, port_id, default_value);
        self.typed_device_op(name, TaskType::DO, |dev| {
            (*dev).add_channel(&format!("port{}/line{}", port_id, line_id), default_value)
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
    /// * `name` - The name of the device to configure.
    /// * `src` - The name of the sample clock source.
    ///
    /// See also: [`BaseDevice::cfg_samp_clk_src`]
    fn device_cfg_samp_clk_src(&mut self, name: &str, src: &str) {
        self.device_op(name, |dev| (*dev).set_samp_clk_src(src))
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
    /// * `name` - The name of the device to configure.
    /// * `trig_line` - The trigger line identifier.
    /// * `export_trig` - A boolean that determines whether to export or import the trigger.
    ///
    /// # Panics
    ///
    /// This method will panic if the synchronization condition related to a device exporting triggers is violated.
    ///
    /// See also: [`BaseDevice::cfg_trig`]
    fn device_set_start_trig_term(&mut self, name: &str, terminal: &str) {
        // assert!(
        //     !export_trig
        //         || (export_trig
        //             && self
        //                 .devices()
        //                 .values()
        //                 .all(|dev| dev.export_trig().is_none())),
        //     "Device {} cannot export triggers since another device already exports triggers.",
        //     name
        // );
        self.device_op(name, |dev| (*dev).set_start_trig_term(trig_line));
    }

    /// Configures the reference clock settings of a device in the experiment.
    ///
    /// This method retrieves the specified device and delegates the configuration
    /// of the reference clock settings to its base method [`BaseDevice::cfg_ref_clk`].
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the device to configure.
    /// * `ref_clk_line` - The line or channel to import or export the device's reference clock.
    /// * `ref_clk_rate` - The rate of the reference clock in Hz.
    /// * `export_ref_clk` - A boolean that determines whether to export (if `true`) or import (if `false`) the reference clock.
    ///
    /// See also: [`BaseDevice::cfg_ref_clk`]
    fn device_import_ref_clk(
        &mut self,
        name: &str,
        src: &str,
        rate: f64,
    ) {
        self.device_op(name, |dev| {
            (*dev).import_ref_clk(src, rate)
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
    /// * `name`: The name of the target device.
    ///
    /// # Returns
    ///
    /// Returns the edit stop time for the specified device.
    fn device_last_instr_end_time(&mut self, name: &str) -> f64 {
        self.device_op(name, |dev| (*dev).last_instr_end_time())
    }

    /// Retrieves the `total_run_time` for the specified device.
    ///
    /// # Returns
    ///
    /// The `total_run_time` for this device.
    ///
    /// See [`BaseDevice::total_run_time`] for more details.
    fn device_total_run_time(&mut self, name: &str) -> f64 {
        self.device_op(name, |dev| (*dev).total_run_time())
    }

    /// Clears the compilation cache for a specific device.
    ///
    /// Utilizing the [`BaseExperiment::device_op`] function, this method forwards the request
    /// to clear the compilation cache to the specified device. Also see [`BaseDevice::clear_compile_cache`].
    ///
    /// # Arguments
    ///
    /// * `name`: The name of the target device.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e7);
    /// // ... other operations ...
    /// exp.device_clear_compile_cache("PXI1Slot6");
    /// ```
    fn device_clear_compile_cache(&mut self, name: &str) {
        self.device_op(name, |dev| (*dev).clear_compile_cache())
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
    /// * `name`: The name of the target device.
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e7);
    /// // ... other operations ...
    /// exp.device_clear_edit_cache("PXI1Slot6");
    /// ```
    fn device_clear_edit_cache(&mut self, name: &str) {
        self.device_op(name, |dev| (*dev).clear_edit_cache())
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
    /// * `name`: The name of the target device.
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
    /// exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
    /// exp.add_do_channel("PXI1Slot6", 2, 0, 0.);
    /// exp.add_do_channel("PXI1Slot6", 2, 1, 0.);
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
        name: &str,
        require_streamable: bool,
        require_editable: bool,
    ) -> Vec<String> {
        self.device_op(name, |dev| {
            (*dev)
                .compiled_channels(require_streamable, require_editable)
                .iter()
                .map(|chan| chan.name().to_string())
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
    ///
    /// Note: after `duration` elapses, output value is automatically set to channel default and kept there
    /// until the next instruction or global end. Use [`go_constant`] if you want to keep the same value instead.
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
    /// exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
    /// exp.add_do_channel("PXI1Slot6", 2, 0, 0.);
    /// exp.add_do_channel("PXI1Slot6", 2, 1, 0.);
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
    ) {
        self.typed_channel_op(dev_name, chan_name, TaskType::AO, |chan| {
            (*chan).constant(value, t, Some((duration, false)));
        });
    }
    /// Sets the specified analogue output (AO) channel to a specified constant value for a short duration.
    ///
    /// It allows the user to set the AO channel to an arbitrary value.
    ///
    /// The duration for which the value is held is determined as the inverse of the channel's sampling rate,
    /// ensuring that the signal remains constant for one tick.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target AO channel within the device.
    /// * `t`: The start time for the signal to take the specified value.
    /// * `value`: The desired constant value for the signal.
    ///
    /// # Panics
    ///
    /// This method will panic if the channel is not of type AO.
    fn go_constant(&mut self, dev_name: &str, chan_name: &str, t: f64, value: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::AO, |chan| {
            (*chan).constant(value, t, None);
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
    /// exp.add_ao_channel("PXI1Slot3", 0, 0.);
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
            (*chan).add_instr(instr, t, Some((duration, keep_val)))
        });
    }
    /// Same as [`sine`] but without specific end time ("keep running until the next instruction or global end")
    fn go_sine(
        &mut self,
        dev_name: &str,
        chan_name: &str,
        t: f64,
        freq: f64,
        amplitude: Option<f64>,
        phase: Option<f64>,
        dc_offset: Option<f64>,
    ) {
        self.typed_channel_op(dev_name, chan_name, TaskType::AO, |chan| {
            let instr = Instruction::new_sine(freq, amplitude, phase, dc_offset);
            (*chan).add_instr(instr, t, None)
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
            (*chan).constant(1., t, Some((duration, false)));
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
            (*chan).constant(0., t, Some((duration, false)));
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
            (*chan).constant(1., t, None);
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
            (*chan).constant(0., t, None);
        });
    }

    /// Ramps the specified analogue output (AO) channel linearly between two values over a specified duration.
    ///
    /// This method generates a linear ramping signal, starting from `start_val` and ending at `end_val`, 
    /// over the duration of `duration`. It is useful for gradually changing the signal level on the AO channel.
    ///
    /// # Arguments
    ///
    /// * `dev_name`: The name of the target device.
    /// * `chan_name`: The name of the target AO channel within the device.
    /// * `t`: The start time for the signal to begin ramping.
    /// * `duration`: The duration over which the signal should ramp from `start_val` to `end_val`.
    /// * `start_val`: The initial value of the signal at the start of the ramping period.
    /// * `end_val`: The final value of the signal at the end of the ramping period.
    /// * `keep_val`: A boolean indicating whether the signal should maintain the `end_val` after the ramping 
    ///   duration has completed.
    ///
    /// # Panics
    ///
    /// This method will panic if the channel is not of type AO.
    fn linramp(
        &mut self,
        dev_name: &str,
        chan_name: &str,
        t: f64,
        duration: f64,
        start_val: f64,
        end_val: f64,
        keep_val: bool,
    ) {
        self.typed_channel_op(dev_name, chan_name, TaskType::AO, |chan| {
            let instr = Instruction::new_linramp(start_val, end_val, t, t+duration);
            (*chan).add_instr(instr, t, Some((duration, keep_val)))
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
    /// exp.add_do_channel("PXI1Slot7", 0, 7, 0.);
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
    /// exp.add_do_channel("PXI1Slot7", 0, 7, 0.);
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
    /// exp.add_do_channel("PXI1Slot7", 0, 7, 0.);
    /// exp.go_high("PXI1Slot7", "port0/line7", 0.);
    /// exp.compile(false);
    /// assert_eq!(exp.is_compiled(), true);
    /// exp.channel_clear_compile_cache("PXI1Slot7", "port0/line7");
    /// assert_eq!(exp.is_compiled(), false);
    /// ```
    fn channel_clear_compile_cache(&mut self, dev_name: &str, chan_name: &str) {
        self.channel_op(dev_name, chan_name, |chan| (*chan).clear_compile_cache());
    }

    fn channel_last_instr_end_time(&mut self, dev_name: &str, chan_name: &str) -> f64 {
        self.channel_op(dev_name, chan_name, |chan| {
            (*chan).last_instr_end_time()
        })
    }
}

/// A concrete struct consisting of a collection of devices.
///
/// **Refer to the [`BaseExperiment`] trait for method behavior.**
#[pyclass]
pub struct Experiment {
    devices: IndexMap<String, Device>,
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
/// use indexmap::IndexMap;
///
/// #[pyclass]
/// struct CustomExperiment {
///     devices: IndexMap<String, Device>,
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
///             devices: IndexMap::new(),
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
            fn devices(&self) -> &IndexMap<String, Device> {
                &self.devices
            }
            fn devices_(&mut self) -> &mut IndexMap<String, Device> {
                &mut self.devices
            }
        }

        #[pymethods]
        impl $exp_type {
            fn add_ao_device(&mut self, name: &str, samp_rate: f64) {
                BaseExperiment::add_ao_device(self, name, samp_rate);
            }

            fn add_do_device(&mut self, name: &str, samp_rate: f64) {
                BaseExperiment::add_do_device(self, name, samp_rate);
            }

            pub fn last_instr_end_time(&self) -> f64 {
                BaseExperiment::last_instr_end_time(self)
            }

            pub fn total_run_time(&self) -> f64 {
                BaseExperiment::total_run_time(self)
            }

            pub fn check_trig_config(&self) {
                BaseExperiment::check_trig_config(self)
            }

            pub fn compile(&mut self, stop_time: Option<f64>) -> f64 {
                BaseExperiment::compile(self, stop_time)
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

            pub fn add_reset_instr(&mut self, reset_time: Option<f64>) {
                BaseExperiment::add_reset_instr(self, reset_time)
            }

            pub fn clear_compile_cache(&mut self) {
                BaseExperiment::clear_compile_cache(self);
            }

            // DEVICE METHODS
            pub fn add_ao_channel(&mut self, name: &str, channel_id: usize, default_value: f64) {
                BaseExperiment::add_ao_channel(self, name, channel_id, default_value);
            }

            pub fn add_do_channel(&mut self, name: &str, port_id: usize, line_id: usize, default_value: f64) {
                BaseExperiment::add_do_channel(self, name, port_id, line_id, default_value);
            }

            pub fn device_cfg_samp_clk_src(&mut self, name: &str, src: &str) {
                BaseExperiment::device_cfg_samp_clk_src(self, name, src);
            }

            pub fn device_set_start_trig_term(&mut self, name: &str, terminal: &str) {
                BaseExperiment::device_set_start_trig_term(self, name, terminal);
            }

            pub fn device_import_ref_clk(
                &mut self,
                name: &str,
                src: &str,
                rate: f64,
            ) {
                BaseExperiment::device_import_ref_clk(
                    self,
                    name,
                    src,
                    rate,
                );
            }

            pub fn device_compiled_channel_names(
                &mut self,
                name: &str,
                require_streamable: bool,
                require_editable: bool,
            ) -> Vec<String> {
                BaseExperiment::device_compiled_channel_names(
                    self,
                    name,
                    require_streamable,
                    require_editable,
                )
            }

            pub fn calc_signal(
                &mut self,
                name: &str,
                t_start: f64,
                t_end: f64,
                nsamps: usize,
                require_streamable: bool,
                require_editable: bool,
                py: Python,
            ) -> PyResult<PyObject> {
                self.assert_has_device(name);
                let samp_rate = self.devices().get(name).unwrap().samp_rate();
                let arr = BaseExperiment::device_calc_signal_nsamps(
                    self,
                    name,
                    (t_start * samp_rate) as usize,
                    (t_end * samp_rate) as usize,
                    nsamps,
                    require_streamable,
                    require_editable,
                );
                Ok(numpy::PyArray::from_array(py, &arr).to_object(py))
            }

            pub fn device_last_instr_end_time(&mut self, name: &str) -> f64 {
                BaseExperiment::device_last_instr_end_time(self, name)
            }

            pub fn device_total_run_time(&mut self, name: &str) -> f64 {
                BaseExperiment::device_total_run_time(self, name)
            }

            pub fn device_clear_compile_cache(&mut self, name: &str) {
                BaseExperiment::device_clear_compile_cache(self, name)
            }

            pub fn device_clear_edit_cache(&mut self, name: &str) {
                BaseExperiment::device_clear_edit_cache(self, name)
            }

            // INSTRUCTION METHODS
            pub fn constant(
                &mut self,
                dev_name: &str,
                chan_name: &str,
                t: f64,
                duration: f64,
                value: f64,
            ) {
                BaseExperiment::constant(self, dev_name, chan_name, t, duration, value);
            }
            pub fn go_constant(&mut self, dev_name: &str, chan_name: &str, t: f64, value:f64) {
                BaseExperiment::go_constant(self, dev_name, chan_name, t, value);
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
            pub fn go_sine(
                &mut self,
                dev_name: &str,
                chan_name: &str,
                t: f64,
                freq: f64,
                amplitude: Option<f64>,
                phase: Option<f64>,
                dc_offset: Option<f64>,
            ) {
                BaseExperiment::go_sine(
                    self, dev_name, chan_name, t, freq, amplitude, phase, dc_offset,
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

            pub fn linramp(
                &mut self,
                dev_name: &str,
                chan_name: &str,
                t: f64,
                duration: f64,
                start_val: f64,
                end_val: f64,
                keep_val: bool,
            ) {
                BaseExperiment::linramp(self, dev_name, chan_name, t, duration, start_val, end_val, keep_val);
            }

            // CHANNEL METHODS
            pub fn channel_clear_compile_cache(&mut self, dev_name: &str, chan_name: &str) {
                BaseExperiment::channel_clear_compile_cache(self, dev_name, chan_name);
            }

            pub fn channel_clear_edit_cache(&mut self, dev_name: &str, chan_name: &str) {
                BaseExperiment::channel_clear_edit_cache(self, dev_name, chan_name);
            }

            pub fn channel_last_instr_end_time(&mut self, dev_name: &str, chan_name: &str) -> f64 {
                BaseExperiment::channel_last_instr_end_time(self, dev_name, chan_name)
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
    /// The underlying representation of this collection is a IndexMap where device names (strings) map to their
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
            devices: IndexMap::new(),
        }
    }
}

impl_exp_boilerplate!(Experiment);

#[cfg(test)]
/// One of the main parts of the `BaseExperiment` logic is to handle a collection of devices
/// with different and incommensurate sample clock rates.
/// Their clock grids generally don't align, so timing has to be in `f64` at the `BaseExperiment` level
/// while each device, when give a float time, should round it to its' own `usize` clock grid.
///
/// Clock grid mismatch mostly affects the operations at the global sequence end - compiling
/// with some (or None) stop time, adding reset instruction, and calculating total run time.
///
/// In particular:
/// * Devices cannot always physically stop at the same time. Even if requested to compile
///   to the same total duration, actual run time of each device may be different;
/// * The `usize clock grid` -> `common f64 time` (typically max across all devices) -> back to `usize clock grid`
///   conversion process can potentially lead to "last sample clipping" errors due to unsafe rounding.
///
/// -----------------------------------------------
/// For all the tests below we pick:
///     Dev1: samp_rate = 1000 Sa/s
///     Dev2: samp_rate = 123 Sa/s
/// Their clock grids do not match anywhere except for integer multiple of 1s.
///
/// We typically add pulses to both devices with nominally identical edge times of 0.1s, 0.2s, ..., 0.9s, 1.0s
///
/// * For Dev1, these times match clock grid and the actual pulses will have the expected edges:
///     0.100  0.200  0.300  0.400  0.500  0.600  0.700  0.800  0.900  1.000
///
/// * For Dev2, these times don't match clock grid and rounding will make edges "jump" around originally specified values:
///     0.098  0.203  0.301  0.398  0.496  0.602  0.699  0.797  0.902  1.000
///
/// As a result, the `last_instr_end_time` for Dev2 is sometimes slightly above, slightly below,
/// or precisely equal to that for Dev1, giving the full range of how finial edges could compare
/// across different devices.
///
/// We can also run from 0s to 10s to repeat the 1s mismatch period 10 times.
mod test {
    mod compile_stop_time {
        use crate::experiment::*;
        use crate::instruction::*;

        #[test]
        /// Test to ensure there are no panics with compiling with `stop_time = None`
        /// (flash with the `last_instr_end_time`) for the case of incommensurate clock grids.
        ///
        /// **Successful test**: no device should panic at `dev.compile(exp.last_instr_end_time())`
        /// due to its' last instruction being clipped.
        fn incommensurate_clocks() {
            let mut exp = Experiment::new();
            exp.add_do_device("Dev1", 1000.0);
            exp.add_do_device("Dev2", 123.0);
            exp.dev_("Dev1").add_channel("port0/line0", 0.0);
            exp.dev_("Dev2").add_channel("port0/line0", 0.0);

            let mock_func = Instruction::new_const(1.0);  // actual function doesn't matter

            // Array of nominal stop times:
            //  from 0s to 1s in steps of 100ms to go through the full mismatch period
            //  actually go from 0s to 10s to repeat this 10 times
            let interv = 100e-3;
            let dur = 50e-3;
            let stop_time_arr: Vec<f64> = (1..101).into_iter().map(|i| interv * i as f64).collect();

            for stop_time in stop_time_arr {
                for dev in exp.devices_().values_mut() {
                    for chan in dev.editable_channels_() {
                        chan.add_instr(
                            mock_func.clone(),
                            stop_time - dur, Some((dur, false))
                        )
                    }
                }
                // The actual test - this call should not panic due to any last instructions being clipped:
                exp.compile(None);

                // Additional (not really necessary) check to ensure no instructions were clipped
                //  (hard to check for the exact individual total_run_times
                //  because it would require keeping track of the rounding jumps at instruction adding
                //  and the extra tail tick when compiling):
                let dev1 = exp.dev("Dev1");
                assert!(dev1.last_instr_end_time() - 1e-10 < dev1.total_run_time());
                let dev2 = exp.dev("Dev2");
                assert!(dev2.last_instr_end_time() - 1e-10 < dev2.total_run_time());

                /*
                println!("\n=============================================");
                let dev1 = exp.dev("Dev1");
                println!("dev1.last_instr_end_time() = {}", dev1.last_instr_end_time());
                println!("dev1.total_samps() = {}", dev1.total_samps());
                println!("dev1.total_run_time() = {}", dev1.total_run_time());

                println!("\n");
                let dev2 = exp.dev("Dev2");
                println!("dev2.last_instr_end_time() = {}", dev2.last_instr_end_time());
                println!("dev2.total_samps() = {}", dev2.total_samps());
                println!("dev2.total_run_time() = {}", dev2.total_run_time());
                */
            }
        }

        #[test]
        /// When the requested compile stop time matches an integer tick across all clock grids
        /// and is far away from any closing edges of last pulses (so no extra sample is added anywhere),
        /// total run time must be precisely equal to the requested stop time.
        ///
        /// We pick
        ///     Dev1: samp_rate = 1000 Sa/s
        ///     Dev2: samp_rate = 123 Sa/s
        /// Their clock ticks generally do not match, but align for integer multiple of 1s.
        ///
        /// So compiling with `stop_time = Some(1.0)` far away from any closing pulse edges
        /// must give `total_run_time = 1.0`
        fn predictable_total_run_time() {
            let mut exp = Experiment::new();
            exp.add_do_device("Dev1", 1000.0);
            exp.add_do_device("Dev2", 123.0);
            exp.dev_("Dev1").add_channel("port0/line0", 0.0);
            exp.dev_("Dev2").add_channel("port0/line0", 0.0);

            let mock_func = Instruction::new_const(0.0);  // actual function doesn't matter

            exp.dev_("Dev1").chan_("port0/line0").add_instr(
                mock_func.clone(),
                0.0, Some((0.5, false))
            );
            exp.dev_("Dev2").chan_("port0/line0").add_instr(
                mock_func.clone(),
                0.0, Some((0.5, false))
            );

            exp.compile(Some(1.0));
            assert!(
                f64::abs(exp.total_run_time() - 1.0) < 1e-10
            );
        }
    }

    mod add_reset_instr {
        use crate::experiment::*;
        use crate::instruction::*;

        #[test]
        /// Main goal - no device should panic due to reset instruction colliding with the end
        /// of its latest instruction.
        fn incommensurate_clocks() {
            let mut exp = Experiment::new();
            exp.add_do_device("Dev1", 1000.0);
            exp.add_do_device("Dev2", 123.0);
            exp.dev_("Dev1").add_channel("port0/line0", 0.0);
            exp.dev_("Dev2").add_channel("port0/line0", 0.0);

            let mock_func = Instruction::new_const(0.0);

            // Prepare the range of nominal "last_instr_end_time":
            //      0.1, 0.2, ..., 0.9, 1.0, 1.1, ..., 9.9, 10.0
            let interv = 100e-3;
            let dur = 50e-3;
            let stop_time_arr: Vec<f64> = (1..101).into_iter().map(|i| interv * i as f64).collect();
            // println!("stop_time_arr = {stop_time_arr:?}");

            for stop_time in stop_time_arr {
                exp.clear_edit_cache();
                for dev in exp.devices_().values_mut() {
                    for chan in dev.editable_channels_() {  // ToDo when splitting AO/DO types: remove `editable` filter
                        chan.add_instr(
                            mock_func.clone(),
                            stop_time - dur, Some((dur, true))
                        )
                    }
                }
                // Neither of the following calls should panic:
                exp.add_reset_instr(None);
                exp.compile(None);
            }
        }

        #[test]
        /// When requested reset time matches an integer clock tick across all devices,
        /// reset should happen precisely at this time
        fn predictable_reset_time() {
            let mut exp = Experiment::new();
            exp.add_do_device("Dev1", 1000.0);
            exp.add_do_device("Dev2", 123.0);
            exp.dev_("Dev1").add_channel("port0/line0", 0.0);
            exp.dev_("Dev2").add_channel("port0/line0", 0.0);

            let mock_func = Instruction::new_const(1.0);
            for dev in exp.devices_().values_mut() {
                for chan in dev.editable_channels_() {
                    chan.add_instr(mock_func.clone(), 0.0, None)
                }
            }
            // In this test, clock grids align at `t = 1.0s`
            let reset_time = 1.0;
            exp.add_reset_instr(Some(reset_time));
            exp.compile(None);

            // Confirm that all channels actually give their reset values at `t = match_time`
            for dev in exp.devices().values() {
                for chan in dev.editable_channels() {
                    // Reset is the last instruction, so its' start position is the end of the previous instruction:
                    let actual_reset_pos = chan.instr_end()[chan.instr_end().len() - 2];
                    let expected_reset_pos = (reset_time * chan.samp_rate()).round() as usize;
                    assert_eq!(actual_reset_pos, expected_reset_pos);

                    // Additionally, confirm the reset function indeed gives the reset value
                    // (this only probes the function of the reset instruction and does not probe reset time)
                    let reset_func = chan.instr_val().last().unwrap();
                    assert!(f64::abs(
                        reset_func.eval_point(reset_time) - chan.reset_value()  // ToDo: this could be written as `chan.calc_signal_nsamps(1.0, 1.0, 1)[0]`, but currently `fill_signal_nsamps()` requires `start_pos < end_pos`
                    ) < 1e-10)
                }
            }
        }
    }
}
