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

use std::collections::HashMap;
use numpy;
use pyo3::prelude::*;
use rayon::prelude::*;
use indexmap::IndexMap;
use std::sync::Arc;
use parking_lot::Mutex;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::thread::JoinHandle;

use nicompiler_backend::*;

use crate::device::*;
use crate::nidaqmx;
use crate::nidaqmx::DAQmxError;
// use crate::nidaqmx::*;
use crate::utils::Semaphore;
use crate::utils::StreamCounter;  // FixMe [after Device move to streamer crate]
use crate::worker_cmd_chan::{CmdChan, CmdRecvr, WorkerCmd};

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
    // FixMe [after Device move to streamer crate]:
    //  NiTask handles and StreamCounters should be saved in Device fields
    ref_clk_provider: Option<(String, String)>,  // Some((dev_name, terminal_name))
    start_trig_primary: Option<String>,  // Some(dev_name)
    running_devs: IndexMap<String, Arc<Mutex<Device>>>,
    worker_handles: IndexMap<String, JoinHandle<Result<(), WorkerError>>>,
    worker_report_recvrs: IndexMap<String, Receiver<()>>,
    worker_cmd_chan: CmdChan,
}

impl_exp_boilerplate!(Experiment);

impl Experiment {

    pub fn set_start_trig_primary(&mut self, dev: Option<String>) {
        self.start_trig_primary = dev;
    }
    pub fn get_start_trig_primary(&self) -> Option<String> {
        self.start_trig_primary.clone()
    }

    pub fn set_ref_clk_provider(&mut self, provider: Option<(String, String)>) {
        self.ref_clk_provider = provider;
    }
    pub fn get_ref_clk_provider(&self) -> Option<(String, String)> {
        self.ref_clk_provider.clone()
    }

    fn export_ref_clk_(&mut self) -> Result<(), DAQmxError> {
        if let Some((dev_name, term_name)) = &self.ref_clk_provider {
            // ToDo: Try tristating the terminal on all other cards in the streamer to ensure the line is not driven
            nidaqmx::connect_terms(
                &format!("/{dev_name}/10MHzRefClock"),
                &format!("/{dev_name}/{term_name}")
            )?;
        };
        Ok(())
    }
    pub fn undo_export_ref_clk_(&mut self) -> Result<(), DAQmxError> {
        if let Some((dev_name, term_name)) = &self.ref_clk_provider {
            nidaqmx::disconnect_terms(
                &format!("/{dev_name}/10MHzRefClock"),
                &format!("/{dev_name}/{term_name}")
            )?;
        };
        Ok(())
    }

    fn collect_thread_reports(&mut self) -> Result<(), String> {
        // Wait for each worker thread to report completion or stop working (by returning or panicking)
        let mut failed_worker_names = Vec::new();
        for (dev_name, recvr) in self.worker_report_recvrs.iter() {
            match recvr.recv() {
                Ok(()) => {},
                Err(_err) => { failed_worker_names.push(dev_name.to_string())},
            };
        };
        if failed_worker_names.is_empty() {
            return Ok(())
        }

        // If any of the workers did not report Ok, they must have stopped either by gracefully returning a WorkerError or by panicking.
        // Collect info from all failed workers and return the full error message.

        // For each failed worker:
        // * dispose of thread_report_receiver
        // * join the thread to collect error info (join() will automatically consume and dispose of the thread handle)
        let mut err_msg_map = IndexMap::new();
        for dev_name in failed_worker_names.iter() {
            self.worker_report_recvrs.shift_remove(dev_name).unwrap();

            let join_handle = self.worker_handles.shift_remove(dev_name).unwrap();
            match join_handle.join() {
                Ok(worker_result) => {
                    // The worker had an error but returned gracefully. The WorkerError should be contained in the return result
                    match worker_result {
                        Err(worker_error) => err_msg_map.insert(dev_name.to_string(), worker_error.to_string()),
                        Ok(_) => panic!("Unexpected scenario - worker '{dev_name}' has stopped working (did not report completion on thread_report_recvr) but returned a non-error result"),
                    }
                },
                Err(_panic_info) => {
                    // The worker has panicked. Panic info should be contained in the returned object
                    err_msg_map.insert(dev_name.to_string(), format!("Worker has panicked"))
                },
            };
        }
        // Assemble and return the full error message string
        let mut full_err_msg = String::new();
        for (dev_name, err_msg) in err_msg_map {
            full_err_msg.push_str(&format!(
                "[{dev_name}] {err_msg}\n"
            ))
        }

        // FixMe: this is a temporary dirty hack.
        //  Transfer device objects back to the main map (they were transferred out to be able to wrap them into Arc<Mutex<>> for multithreading)
        for dev_name in failed_worker_names.iter() {
            let dev_container = self.running_devs.shift_remove(dev_name).unwrap();
            let dev = Arc::into_inner(dev_container).unwrap().into_inner();  // this line extracts Device instance from Arc<Mutex<>> container
            self.devices.insert(dev_name.to_string(), dev);
        }

        // println!("[collect_thread_reports()] list of failed threads: {:?}", failed_worker_names);
        // println!("[collect_thread_reports()] full error message:\n{}", full_err_msg);
        Err(full_err_msg)
    }

    pub fn cfg_run_(&mut self, bufsize_ms: f64) -> Result<(), String> {
        let running_dev_names: Vec<String> = self.devices
            .iter()
            .filter(|(_name, dev)| dev.is_compiled())
            .map(|(name, _dev)| name.to_string())
            .collect();
        if running_dev_names.is_empty() {
            return Ok(())
        };

        /* ToDo: Maybe add a consistency check here: no clash between any exports (ref clk, start_trig, samp_clk) */

        // Prepare thread sync mechanisms

        // - command broadcasting channel
        self.worker_cmd_chan = CmdChan::new();  // the old instance can be reused, but refreshing here to zero `msg_num` for simplicity

        // - inter-worker start_trig sync channels
        let mut start_sync = HashMap::new();
        if let Some(primary_dev_name) = &self.start_trig_primary {
            // Sanity checks
            if !running_dev_names.contains(primary_dev_name) {
                return Err(format!("Either the primary device name '{primary_dev_name}' is invalid or this device didn't get any instructions and will not run at all"))
            };
            if self.devices.get(primary_dev_name).unwrap().get_start_trig_out().is_none() {
                return Err(format!("Device '{primary_dev_name}' was designated to be the start trigger primary, but has no trigger output terminal specified"))
            };

            // Create and pack sender-receiver pairs
            let mut recvr_vec = Vec::new();
            // - first create all the secondaries
            for dev_name in running_dev_names.iter().filter(|dev_name| dev_name != primary_dev_name) {
                let (sender, recvr) = channel();
                recvr_vec.push(recvr);
                start_sync.insert(
                    dev_name.to_string(),
                    StartSync::Secondary(sender)
                )
            }
            // - now create the primary
            start_sync.insert(
                primary_dev_name.to_string(),
                StartSync::Primary(recvr_vec)
            );
        } else {
            for dev_name in running_dev_names.iter() {
                start_sync.insert(
                    dev_name.to_string(),
                    StartSync::None,
                )
            }
        }

        // FixMe: this is a temporary dirty hack.
        //  Transfer device objects to a separate IndexMap to be able to wrap them into Arc<Mutex<>> for multithreading
        for dev_name in running_dev_names {
            let dev = self.devices.shift_remove(&dev_name).unwrap();
            self.running_devs.insert(dev_name, Arc::new(Mutex::new(dev)));
        }

        // Do static ref clk export
        /* We are using static ref_clk export (as opposed to task-based export) to be able to always use
        the same card as the clock reference source even if this card does not run this time. */
        if let Err(daqmx_err) = self.export_ref_clk_() {
            return Err(daqmx_err.to_string())
        };

        for (dev_name, dev_container) in self.running_devs.iter() {
            // - worker command receiver
            let cmd_recvr = self.worker_cmd_chan.new_recvr();

            // - worker report channel
            let (report_sendr, report_recvr) = channel();
            self.worker_report_recvrs.insert(dev_name.to_string(), report_recvr);

            // Launch worker thread
            let dev_mutex = dev_container.clone();
            let spawn_result = thread::Builder::new().name(dev_name.to_string())
                .spawn(move || -> Result<(), WorkerError> {
                    let mut dev = dev_mutex.lock();
                    dev.worker_loop(
                        bufsize_ms,
                        cmd_recvr, report_sendr,
                        start_sync.remove(dev_name).unwrap()
                    )
                });
            let handle = match spawn_result {
                Ok(handle) => handle,
                Err(err) => return Err(err.to_string())
            };
            self.worker_handles.insert(dev_name.to_string(), handle);
        }
        // Wait for all workers to report completion (handle error collection if necessary)
        self.collect_thread_reports()
    }
    pub fn stream_run_(&mut self, calc_next: bool) -> Result<(), String> {
        todo!()
    }
    pub fn close_run_(&mut self) {

        // FixMe: this is a dirty hack.
        //  Transfer device objects to a separate IndexMap to be able to wrap them into Arc<Mutex<>> for multithreading
        // Return all used device objects back to the main IndexMap
        for (dev_name, dev_box) in self.running_devs.into_iter() {
            let dev = Arc::into_inner(dev_box).unwrap().into_inner();
            self.devices.insert(dev_name, dev)
        }

        let _ = self.undo_export_ref_clk_();
        todo!()
    }
}

#[pymethods]
impl Experiment {

    pub fn run(&self, bufsize_ms: f64, nreps: usize) {
        todo!()
    }

    fn cfg_run(&mut self, bufsize_ms: f64) {  // -> Result<(), String>

        /* ToDo:
        Add consistency check here:
        no clash between any exports (ref clk, start_trig, samp_clk)
        */

        // Static ref clk export
        let mut exporting_devs: Vec<&Device> = self.devices.values().filter(|dev| {
            match dev.ref_clk() {
                Some((_line, export, _rate)) => export,
                None => false,
            }
        }).collect();
        match exporting_devs.len() {
            0 => {},
            1 => {
                let dev = exporting_devs[0];
                let name = dev.name();
                let (line, _export, _rate) = dev.ref_clk().unwrap();
                /* ToDo:
                Try tristating the terminal on all other cards to ensure the line is not driven
                */
                nidaqmx::connect_terms(
                    &format!("/{name}/10MHzRefClock"),
                    &format!("/{name}/{line}")
                );
            },
            _ => panic!("Only one device can export reference clock")
        };

        // Configure NI DAQmx tasks on all compiled devices
        let res_vec: Vec<_> = self.compiled_devices().par_iter_mut().map(
            |dev| -> _ {
                let (task, counter) = dev.cfg_run(bufsize_ms);
                (dev.name().to_string(), Arc::new(Mutex::new(task)), counter)
            }
        ).collect();

        /*
        let mut thread_handles = Vec::new();
        for dev in self.compiled_devices() {
            let dev_arc = Arc::new(Mutex::new(dev));
            let join_handle = thread::Builder::new().name(dev.name().to_string())
                .spawn(move || {
                    dev_arc.cfg_run(bufsize_ms)
                });
            thread_handles.push(join_handle.unwrap());
        }
        for handle in thread_handles {
            // FixMe [after Device move to streamer crate]: NiTask+StreamCounter should be saved in Device fields
            let dev_name = handle.thread().name().unwrap().to_string();
            let (ni_task, stream_counter) = handle.join().unwrap();
            self.ni_tasks.insert(
                dev_name.clone(),
                ni_task
            );
            self.stream_counters.insert(
                dev_name,
                stream_counter
            );
        }
        */
    }

    /*
    fn stream_run(&mut self, calc_next: bool) {
        let shared_sem = Arc::new(Semaphore::new(0));
        let mut join_handles = Vec::new();

        for dev in self.devices().values() {
            let sem_clone = shared_sem.clone();
            join_handle = thread::Builder::new().name(dev.name().to_string())
                .spawn(move || {
                    dev.stream_run(
                        sem_clone,
                        self.compiled_devices().len(),
                        calc_next,
                        // FixMe [after Device move to streamer crate]: this should be stored in Device fields:
                        self.ni_tasks.get(dev.name()).unwrap(),
                        self.stream_counters.get_mut(dev.name()).unwrap(),
                        10 * (self.bufsize_ms / 1000.0)
                    )
                }).unwrap();
        }
        for handle in join_handles {
            handle.join()
        }
    }

    fn clean_run(&mut self) {
        // FixMe [after Device move to streamer crate]: call Device.clear_run() instead
        self.ni_tasks.clear();
        self.stream_counters.clear();
        self.bufsize_ms = 0.0;
    }

    fn check_line_not_driven(&self, line: &str, exclude_dev: &str) {  //  -> Result<bool, String>
        todo!()
    }
    */

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
    // pub fn stream_exp(&self, bufsize_ms: f64, nreps: usize) {
    //     // Simple parallelization: invoke stream_task for every device
    //     let sem_shared = Arc::new(Semaphore::new(1));
    //     self.compiled_devices().par_iter().for_each(|dev| {
    //         let sem_clone = sem_shared.clone();
    //         dev.stream_task(
    //             &sem_clone,
    //             self.compiled_devices().len(),
    //             bufsize_ms,
    //             nreps,
    //         );
    //     });
    // }

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
        self.device_op(name, |_dev| nidaqmx::reset_ni_device(name));
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
            .for_each(|dev| nidaqmx::reset_ni_device(dev.name()));
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
            // FixMe [after Device move to streamer crate]:
            //  NiTask+StreamCounter should be saved in Device fields
            ref_clk_provider: None,
            running_devs: IndexMap::new(),
            worker_handles: IndexMap::new(),
            worker_report_recvrs: IndexMap::new(),
            worker_cmd_chan: CmdChan::new(),
        }
    }
}

impl Drop for Experiment {
    fn drop(&mut self) {
        self.close_run_();
    }
}
