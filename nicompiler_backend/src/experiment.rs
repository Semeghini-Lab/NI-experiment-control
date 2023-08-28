//! Implements the main trait [`BaseExperiment`] for the [`Experiment`] struct, which constitute the highest
//! level of abstraction for interacting with NI tasks. The [`Experiment`] task, together
//! with its implementation, constitute the main API through which python
//! processes invoke the rust backend.

use ndarray::Array2;
use numpy;
use pyo3::prelude::*;
use std::collections::HashMap;

use crate::channel::*;
use crate::device::*;
use crate::instruction::*;
use crate::utils::*;

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
///     - [`device_calc_signal_nsamps`]
///     - [`device_edit_stop_time`], [`device_compiled_stop_time`]
///     - [`device_clear_compile_cache`], [`device_clear_edit_cache`]
/// 3. Channel-targeted methods which alter or query the behavior of a particular channel
///     - [`constant`], [`sine`], [`high`], [`low`], [`go_high`], [`go_low`]
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
    /// exp.add_do_device("PXI1Slot6", 1e6, None, None, None, None, None, None,);
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
    /// exp.add_do_device("PXI1Slot6", 1e6, None, None, None, None, None, None,);
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
    /// This method registers a new device to the experiment ensuring that there are no duplicates
    /// and maintaining the synchronization conditions related to primary devices.
    ///
    /// # Arguments
    /// 
    /// * `dev`: A [`Device`] instance to be added to the experiment.
    ///
    /// # Synchronization Check
    /// 
    /// The method enforces synchronization rules related to primary devices:
    /// 
    /// 1. If all devices in the experiment are non-primary (i.e., `is_primary()` returns `None` for all devices), 
    ///    then the new device can be added without any restrictions related to its primary status.
    /// 2. If there's already a primary device (i.e., a device for which `is_primary()` returns `Some(true)`), 
    ///    then the new device must not also be primary. The experiment can only have one primary device.
    /// 
    /// # Panics
    /// 
    /// This method will panic in the following situations:
    /// 
    /// 1. If a device with the same name as the provided `dev` is already registered in the experiment.
    /// 2. If the synchronization conditions related to primary devices are violated.
    /// 3. If the `trig_line` and `is_primary` options for the device are not both ignored or both specified.
    /// 4. If any of the `ref_clk_line`, `ref_clk_rate`, and `import_ref_clk` arguments for the device are 
    ///    not all ignored or all specified together.
    fn add_device_base(&mut self, dev: Device) {
        // Duplicate check
        let dev_name = dev.physical_name();
        assert!(
            !self.devices().contains_key(dev_name),
            "Device {} already registered. Registered devices are {:?}",
            dev_name,
            self.devices().keys().collect::<Vec<_>>()
        );
        // Synchronization check
        assert!(
            // Either all d.is_primary() are None
            self.devices().values().all(|d| d.is_primary().is_none()) ||
            // Or only one d.is_primary() is Some(true)
            (dev.is_primary() == Some(true) && 
            self.devices().values().filter(|d| d.is_primary() == Some(true)).count() == 0),
            "Cannot register another primary device {}",
            dev_name
        );
        // Optional argument check: 
        assert!(
            dev.is_primary().is_none() == dev.trig_line().is_none(),
            "trig_line and is_primary options for device {} must be both ignored or specified",
            dev.physical_name(),
        );
        assert!(
            dev.ref_clk_line().is_none() == dev.ref_clk_rate().is_none() && 
            dev.ref_clk_rate().is_none() == dev.import_ref_clk().is_none(),
            "ref_clk_line, ref_clk_rate, and import_ref_clk arguments for device {} must be 
            all ignored or specified",
            dev.physical_name(),
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
    /// * Other parameters represent optional settings related to the device's operation, 
    /// refer to [`Device`] fields for more information. 
    ///
    /// # Example
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot6", 1e6, None, None, None, None, None, None);
    /// // Adding the same device, even with different parameters, will cause panic
    /// // exp.add_ao_device("PXI1Slot6", 1e6, None, Some("PXI_Trig0"), Some(true), None, None, None);
    /// ```
    fn add_ao_device(
        &mut self,
        physical_name: &str,
        samp_rate: f64,
        samp_clk_src: Option<&str>,
        trig_line: Option<&str>,
        is_primary: Option<bool>,
        ref_clk_line: Option<&str>,
        import_ref_clk: Option<bool>,
        ref_clk_rate: Option<f64>,
    ) {
        self.add_device_base(Device::new(
            physical_name,
            TaskType::AO,
            samp_rate,
            samp_clk_src,
            trig_line,
            is_primary,
            ref_clk_line,
            import_ref_clk,
            ref_clk_rate,
        ));
    }

    /// Registers a Digital Output (DO) device to the experiment.
    /// 
    /// Similar to [`BaseExperiment::add_ao_device`], this method creates a DO device with the provided 
    /// parameters and registers it to the experiment.
    /// 
    /// # Arguments
    /// 
    /// * `physical_name`: A string slice that holds the name of the DO device.
    /// * `samp_rate`: Sampling rate for the DO device.
    /// * Other parameters represent optional settings related to the device's operation, 
    /// refer to [`Device`] fields for more information. 
    ///
    /// # Example
    /// ```should_panic
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot7", 1e6, None, Some("PXI_Trig0"), Some(true), None, None, None);
    /// // Adding two primary devices will cause panic
    /// exp.add_do_device("PXI1Slot6", 1e6, None, Some("PXI_Trig0"), Some(true), None, None, None);
    /// ```
    fn add_do_device(
        &mut self,
        physical_name: &str,
        samp_rate: f64,
        samp_clk_src: Option<&str>,
        trig_line: Option<&str>,
        is_primary: Option<bool>,
        ref_clk_line: Option<&str>,
        import_ref_clk: Option<bool>,
        ref_clk_rate: Option<f64>,
    ) {
        self.add_device_base(Device::new(
            physical_name,
            TaskType::DO,
            samp_rate,
            samp_clk_src,
            trig_line,
            is_primary,
            ref_clk_line,
            import_ref_clk,
            ref_clk_rate,
        ));
    }

    /// Retrieves the latest `edit_stop_time` from all registered devices.
    /// See [`BaseDevice::edit_stop_time`] for more information. 
    /// 
    /// The maximum `edit_stop_time` across all devices.
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
    /// exp.add_do_device("PXI1Slot6", 1e6, None, None, None, None, None, None);
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
    /// exp.add_ao_device("PXI1Slot6", 1e6, None, Some("PXI_Trig0"), Some(false), None, None, None);
    /// // This will panic as there are no primary devices
    /// exp.compile_with_stoptime(10.0);
    /// ```
    /// 
    /// ```
    /// # use nicompiler_backend::*;
    /// let mut exp = Experiment::new();
    /// exp.add_do_device("PXI1Slot6", 1e6, None, None, None, None, None, None);
    /// exp.add_do_channel("PXI1Slot6", 0, 0);
    /// exp.high("PXI1Slot6", "port0/line0", 1., 4.);
    /// 
    /// exp.compile();
    /// assert_eq!(exp.compiled_stop_time(), 4.);
    /// exp.compile_with_stoptime(5.); // Experiment signal will stop at t=5 now
    /// assert_eq!(exp.compiled_stop_time(), 5.);
    /// ```
    fn compile_with_stoptime(&mut self, stop_time: f64) {
        assert!(
            self.devices().values().all(|d| d.is_primary().is_none()) || 
            self.devices().values().filter(|d| d.is_primary() == Some(true)).count() == 1,
            "Cannot compile an experiment with no primary device"
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
    ///
    /// See [`BaseDevice::is_fresh_compiled`] for more details on what constitutes a "freshly compiled" state.
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

    // TEMPLATE METHODS (for forwarding device and channel methods)
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

    // FORWARDED DEVICE METHODS
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

    fn device_clear_compile_cache(&mut self, dev_name: &str) {
        self.device_op(dev_name, |dev| (*dev).clear_compile_cache())
    }

    fn device_clear_edit_cache(&mut self, dev_name: &str) {
        self.device_op(dev_name, |dev| (*dev).clear_edit_cache())
    }

    fn add_ao_channel(&mut self, dev_name: &str, channel_id: usize) {
        self.typed_device_op(dev_name, TaskType::AO, |dev| {
            (*dev).add_channel(&format!("ao{}", channel_id))
        });
    }

    fn add_do_channel(&mut self, dev_name: &str, port_id: usize, line_id: usize) {
        self.typed_device_op(dev_name, TaskType::DO, |dev| {
            (*dev).add_channel(&format!("port{}/line{}", port_id, line_id))
        });
    }

    // Channel methods
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

    fn high(&mut self, dev_name: &str, chan_name: &str, t: f64, duration: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(1., t, duration, false);
        });
    }

    fn low(&mut self, dev_name: &str, chan_name: &str, t: f64, duration: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(0., t, duration, false);
        });
    }

    fn go_high(&mut self, dev_name: &str, chan_name: &str, t: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(1., t, 1. / (*chan).samp_rate(), true);
        });
    }

    fn go_low(&mut self, dev_name: &str, chan_name: &str, t: f64) {
        self.typed_channel_op(dev_name, chan_name, TaskType::DO, |chan| {
            (*chan).constant(0., t, 1. / (*chan).samp_rate(), true);
        });
    }
}

/// A concrete struct implementing the [`BaseExperiment`] trait, consisting of a collection of devices.
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
            fn add_ao_device(
                &mut self,
                physical_name: &str,
                samp_rate: f64,
                samp_clk_src: Option<&str>,
                trig_line: Option<&str>,
                is_primary: Option<bool>,
                ref_clk_line: Option<&str>,
                import_ref_clk: Option<bool>,
                ref_clk_rate: Option<f64>,
            ) {
                self.add_device_base(Device::new(
                    physical_name,
                    TaskType::AO,
                    samp_rate,
                    samp_clk_src,
                    trig_line,
                    is_primary,
                    ref_clk_line,
                    import_ref_clk,
                    ref_clk_rate,
                ));
            }

            fn add_do_device(
                &mut self,
                physical_name: &str,
                samp_rate: f64,
                samp_clk_src: Option<&str>,
                trig_line: Option<&str>,
                is_primary: Option<bool>,
                ref_clk_line: Option<&str>,
                import_ref_clk: Option<bool>,
                ref_clk_rate: Option<f64>,
            ) {
                self.add_device_base(Device::new(
                    physical_name,
                    TaskType::DO,
                    samp_rate,
                    samp_clk_src,
                    trig_line,
                    is_primary,
                    ref_clk_line,
                    import_ref_clk,
                    ref_clk_rate,
                ));
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

            pub fn device_compiled_channel_names(&mut self, dev_name: &str) -> Vec<String> {
                self.device_op(dev_name, |dev| {
                    (*dev)
                        .compiled_channels(false, true)
                        .iter()
                        .map(|chan| chan.physical_name().to_string())
                        .collect()
                })
            }

            pub fn calc_signal(
                &mut self,
                dev_name: &str,
                t_start: f64,
                t_end: f64,
                nsamps: usize,
                py: Python,
            ) -> PyResult<PyObject> {
                self.assert_has_device(dev_name);
                let samp_rate = self.devices().get(dev_name).unwrap().samp_rate();
                // To python, only expose editable channels
                let arr = BaseExperiment::device_calc_signal_nsamps(
                    self,
                    dev_name,
                    (t_start * samp_rate) as usize,
                    (t_end * samp_rate) as usize,
                    nsamps,
                    false,
                    true,
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
