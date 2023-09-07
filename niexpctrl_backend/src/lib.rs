//! # `niexpctrl_backend` - NI Experiment Control and Streaming
//!
//! `niexpctrl_backend` provides a seamless interface to control and stream experiments involving 
//! National Instruments (NI) devices. It extends the foundational functionalities of the [`nicompiler_backend`] crate 
//! to define interaction behavior with NI hardware, while maintaining an optimized and user-friendly 
//! interface for its users.
//!
//! ## Core Functionalities:
//!
//! - **NI Device Streaming and Control:** With the `experiment` module, users can access a refined 
//!   version of the `Experiment` struct from `nicompiler_backend` that incorporates NI-specific 
//!   functionalities, enabling direct streaming to NI devices and their resets.
//!
//! - **NI-DAQmx Specific Operations:** The `nidaqmx` module offers a suite of functionalities 
//!   that interfaces with the NI-DAQmx C library, translating Rust calls into NI-DAQmx specific tasks.
//!
//! - **Utilities and Helpers:** The `utils` module provides additional utilities and helper functions. 
//! 
//!
//! ## Integration with `nicompiler_backend`:
//!
//! This crate is designed to be a natural extension of [`nicompiler_backend`]. 
//! The primary [`Experiment`] struct depends on, and extends its counterpart 
//! in `nicompiler_backend`, maintaining general experiment behaviors and additionally introducing methods 
//! specific to NI device management.
//! **Refer to [`nicompiler_backend`] for general implementations unrelated to NI streaming behavior**. 
//!
//! ## Where to Start:
//! - **Experiment Design and Control:** The [`experiment`] module provides implementation of how device tasks 
//! are concurrently streamed. 
//!
//! - **Device Management:** The [`device`] module implements streaming and synchronization behavior. 
//!
//! - **NI-DAQmx Operations:** The [`nidaqmx`] module provides Rust wrapper methods for calling the 
//!   NI-DAQmx C library, translating functionalities for seamless NI device operations.
//!
//! - **Utilities:** For general utilities and helper functionalities, explore the [`utils`] module.
//! 
//! ## Example usage with streaming 
//! ### Rust
//! Recall the same example snippet from [`nicompiler_backend`].
//! 
//! We additionally call `exp.stream_exp(50., 2);` after the experiment has been designed and compiled to 
//! stream the experiment with a streaming buffer of 50ms, and two repetitions. 
//! Refer to [`StreamableDevice::stream_task`] for more detailed information on streaming behavior. 
//! ```
//! use niexpctrl_backend::*;
//! let mut exp = Experiment::new();
//! // Define devices and associated channels
//! exp.add_ao_device("PXI1Slot3", 1e6);
//! exp.add_ao_channel("PXI1Slot3", 0);
//!
//! exp.add_ao_device("PXI1Slot4", 1e6);
//! exp.add_ao_channel("PXI1Slot4", 0);
//!
//! exp.add_do_device("PXI1Slot6", 1e7);
//! exp.add_do_channel("PXI1Slot6", 0, 0);
//! exp.add_do_channel("PXI1Slot6", 0, 4);
//!
//! // Define synchronization behavior:
//! exp.device_cfg_trig("PXI1Slot3", "PXI1_Trig0", true);
//! exp.device_cfg_ref_clk("PXI1Slot3", "PXI1_Trig7", 1e7, true);
//!
//! exp.device_cfg_trig("PXI1Slot4", "PXI1_Trig0", false);
//! exp.device_cfg_ref_clk("PXI1Slot4", "PXI1_Trig7", 1e7, false);
//!
//! exp.device_cfg_samp_clk_src("PXI1Slot6", "PXI1_Trig7");
//! exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", false);
//!
//! // PXI1Slot3/ao0 starts with a 1s-long 7Hz sine wave with offset 1
//! // and unit amplitude, zero phase. Does not keep its value.
//! exp.sine("PXI1Slot3", "ao0", 0., 1., false, 7., None, None, Some(1.));
//! // Ends with a half-second long 1V constant signal which returns to zero
//! exp.constant("PXI1Slot3", "ao0", 9., 0.5, 1., false);
//!
//! // We can also leave a defined channel empty: the device / channel will simply not be compiled
//!
//! // Both lines of PXI1Slot6 start with a one-second "high" at t=0 and a half-second high at t=9
//! exp.high("PXI1Slot6", "port0/line0", 0., 1.);
//! exp.high("PXI1Slot6", "port0/line0", 9., 0.5);
//! // Alternatively, we can also define the same behavior via go_high/go_low
//! exp.go_high("PXI1Slot6", "port0/line4", 0.);
//! exp.go_low("PXI1Slot6", "port0/line4", 1.);
//!
//! exp.go_high("PXI1Slot6", "port0/line4", 9.);
//! exp.go_low("PXI1Slot6", "port0/line4", 9.5);
//!
//! exp.compile_with_stoptime(10.); // Experiment signal will stop at t=10 now
//! assert_eq!(exp.compiled_stop_time(), 10.);
//! 
//! exp.stream_exp(50., 2); 
//! ```
//! 
//! ### Python
//! Functionally the same code, additionally samples and plots the signal for `PXI1Slot6/port0/line4`. 
//! The primary goal of the `Experiment` object is to expose a complete set of fast rust-implemented methods 
//! for interfacing with a NI experiment. One may easily customize syntactic sugar and higher-level abstractions 
//! by wrapping `nicompiler_backend` module in another layer of python code, 
//! see our [project page](https://github.com/nlyu1/NI-experiment-control) for one such example. 
//! ```Python
//! # Instantiate experiment, define devices and channels
//! from nicompiler_backend import Experiment
//! import matplotlib.pyplot as plt
//!
//! exp = Experiment()
//! exp.add_ao_device(name="PXI1Slot3", samp_rate=1e6)
//! exp.add_ao_channel(name="PXI1Slot3", channel_id=0)
//!
//! ...
//!
//! # Define synchronization behavior
//! exp.device_cfg_trig(name="PXI1Slot3", trig_line="PXI1_Trig0", export_trig=True)
//! exp.device_cfg_ref_clk(name="PXI1Slot3", ref_clk_line="PXI1_Trig7",
//!                        ref_clk_rate=1e7, export_ref_clk=True)
//! ...
//!
//! # Define signal
//! # Arguments of "option" type in rust is converted to optional arguments in python
//! exp.sine(dev_name="PXI1Slot3", chan_name="ao0", t=0., duration=1., keep_val=False,
//!          freq=7., dc_offset=1.)
//! ...
//!
//! exp.compile_with_stoptime(10.)
//! exp.stream_exp(50., 2)
//! ```


use pyo3::prelude::*;

pub mod device;
pub mod experiment;
pub mod nidaqmx;
pub mod utils;

pub use crate::experiment::Experiment;
pub use crate::device::*;
pub use crate::nidaqmx::*;
pub use crate::utils::*;
pub use nicompiler_backend::*;

#[pymodule]
fn niexpctrl_backend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Experiment>()?;
    Ok(())
}
