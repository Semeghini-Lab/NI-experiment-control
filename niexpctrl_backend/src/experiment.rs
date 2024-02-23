//! # NI Device Streaming and Control with the `experiment` Module
//!
//! This module is dedicated to providing a seamless interface for experiments that require direct
//! streaming to National Instruments (NI) devices. Building on the foundation of the
//! [`nicompiler_backend::Experiment`] struct, it introduces an extended `Experiment` struct which
//! offers enhanced functionalities tailored for NI device management.
//!
//! ## Key Features:
//!
//! - **NI Device Streaming:** With methods like [`Experiment::stream_exp`], you can start streaming processes
//!   for all compiled devices within an experiment concurrently. This ensures efficient use of resources and
//!   a smoother user experience.
//!
//! - **Device Reset Capabilities:** Provides methods such as [`Experiment::reset_device`] and
//!   [`Experiment::reset_devices`] to reset specific or all devices, ensuring they're brought back
//!   to a default or known state.
//!
//! - **Multi-threading Support:** The module employs multi-threading capabilities, specifically through the
//!   [`rayon`] crate, to handle concurrent device streaming.
//!
//! ## How to Use:
//!
//! 1. **Initialization:** Create an instance of the `Experiment` struct with the [`Experiment::new`] method.
//!    This gives you an experiment environment with no associated devices initially.
//!
//! 2. **Experiment design:** Design your interface using implemented methods in the [`nicompiler_backend::BaseExperiment`]
//! trait.
//!
//! 3. **Streaming and Control:** Use methods like [`Experiment::stream_exp`] to start the streaming process
//!    and [`Experiment::reset_device`] methods for device resets.
//!
//! ## Relationship with `nicompiler_backend`:
//!
//! This module's `Experiment` struct is an extended version of the [`nicompiler_backend::Experiment`].
//! While it offers NI-specific functionalities, for most general experiment behaviors, it leverages the
//! implementations in [`nicompiler_backend::BaseExperiment`]. Thus, users familiar with the compiler's
//! `Experiment` will find many commonalities here but with added advantages for NI device control.
//!
//! If your use-case doesn't involve direct interaction with NI devices, or you're looking for more general
//! experiment functionalities, please refer to the [`nicompiler_backend::Experiment`] for a broader scope.
//!
//! ## Further Reading:
//!
//! For more in-depth details and examples, refer to the individual struct and method documentations within this
//! module. Also, make sure to explore other related modules like [`device`], [`utils`] for comprehensive
//! device streaming behavior and NI-DAQmx specific operations, respectively.

use numpy;
use pyo3::prelude::*;
use rayon::prelude::*;
use indexmap::IndexMap;
use std::sync::Arc;

use nicompiler_backend::*;

use crate::device::*;
use crate::nidaqmx::*;
use crate::utils::Semaphore;

/// An extended version of the [`nicompiler_backend::Experiment`] struct, tailored to provide direct
/// interfacing capabilities with NI devices.
///
/// This `Experiment` struct is designed to integrate closely with National Instruments (NI) devices,
/// providing methods to stream data to these devices and reset them as needed. It incorporates
/// multi-threading capabilities to handle concurrent device streaming and is equipped with helper
/// methods for device management.
///
/// The underlying `devices` IndexMap contains all the devices registered to this experiment, with
/// device names (strings) as keys mapping to their respective `Device` objects.
///
/// While this struct offers enhanced functionalities specific to NI device management,
/// for most general experiment behaviors, it relies on the implementation of [`nicompiler_backend::BaseExperiment`] in
/// [`nicompiler_backend::Experiment`]. Thus, users familiar with the compiler's `Experiment`
/// will find many similarities here, but with added methods to facilitate control over NI devices.
///
/// If you are not looking to interact directly with NI devices or if your use-case doesn't involve
/// NI-specific operations, refer to [`nicompiler_backend::Experiment`] for a more general-purpose
/// experimental control.
#[pyclass]
pub struct Experiment {
    devices: IndexMap<String, Device>,
}

impl_exp_boilerplate!(Experiment);

#[pymethods]
impl Experiment {
    /// Starts the streaming process for all compiled devices within the experiment.
    ///
    /// This method leverages multi-threading to concurrently stream multiple devices.
    /// For each device in the experiment, a new thread is spawned to handle its streaming behavior.
    /// The streaming behavior of each device is defined by the [`StreamableDevice::stream_task`] method.
    ///
    /// The method ensures lightweight and safe parallelization, ensuring that devices do not interfere with
    /// each other during their streaming processes.
    ///
    /// # Parameters
    ///
    /// * `stream_buftime`: The buffer time for the streaming process. Determines how much data should be
    /// preloaded to ensure continuous streaming.
    /// * `nreps`: Number of repetitions for the streaming process. The devices will continuously stream
    /// their data for this many repetitions.
    pub fn stream_exp(&self, bufsize_ms: f64, nreps: usize) {
        // Simple parallelization: invoke stream_task for every device
        let sem_shared = Arc::new(Semaphore::new(1));
        self.compiled_devices().par_iter().for_each(|dev| {
            let sem_clone = sem_shared.clone();
            dev.stream_task(
                &sem_clone,
                self.compiled_devices().len(),
                bufsize_ms,
                nreps,
            );
        });
    }

    /// Resets a specific device associated with the experiment using the NI-DAQmx framework.
    ///
    /// If the named device is found within the experiment, this method will invoke the necessary calls
    /// to reset it, ensuring it's brought back to a default or known state.
    ///
    /// # Parameters
    ///
    /// * `name`: The name or identifier of the device to be reset.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use niexpctrl_backend::*;
    ///
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot3", 1e6);
    /// exp.reset_device("PXI1Slot3");
    /// ```
    pub fn reset_device(&mut self, name: &str) {
        self.device_op(name, |_dev| reset_ni_device(name));
    }

    /// Resets all the devices that are registered and associated with the experiment.
    ///
    /// This method iterates over all devices within the experiment and invokes the necessary reset calls
    /// using the NI-DAQmx framework. It ensures that all devices are brought back to a default or known state.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use niexpctrl_backend::*;
    ///
    /// let mut exp = Experiment::new();
    /// exp.add_ao_device("PXI1Slot3", 1e6);
    /// exp.add_ao_device("PXI1Slot4", 1e6);
    /// exp.reset_devices();
    /// ```
    pub fn reset_devices(&self) {
        self.devices
            .values()
            .for_each(|dev| reset_ni_device(dev.name()));
    }
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
