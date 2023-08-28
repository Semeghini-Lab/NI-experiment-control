use numpy;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use rayon::prelude::*;

use nicompiler_backend::*;

use crate::device::*;
use crate::nidaqmx::*;
use crate::utils::Semaphore;

#[pyclass]
pub struct Experiment {
    devices: HashMap<String, Device>,
}

impl_exp_boilerplate!(Experiment);

#[pymethods]
impl Experiment {
    pub fn stream_exp(&self, stream_buftime: f64, nreps: usize) {
        // Simple parallelization: invoke stream_task for every device
        let sem_shared = Arc::new(Semaphore::new(1));
        self.compiled_devices().par_iter().for_each(|dev| {
            let sem_clone = sem_shared.clone();
            dev.stream_task(&sem_clone, self.compiled_devices().len(), stream_buftime, nreps);
        });
    }

    pub fn reset_device(&mut self, dev_name: &str) {
        self.device_op(dev_name, |_dev| reset_ni_device(dev_name));
    }

    pub fn reset_devices(&self) {
        self.devices
            .values()
            .for_each(|dev| reset_ni_device(dev.physical_name()));
    }
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