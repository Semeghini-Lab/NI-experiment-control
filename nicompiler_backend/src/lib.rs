//! # National Instrument (NI) Integration with `nicompiler_backend`
//!
//! National Instrument (NI) has long been a preferred choice for building experimental control systems, owing to the
//! versatility, cost-effectiveness, extensibility, and robust documentation of its hardware. Their substantial
//! documentation spans from system design ([NI-DAQmx Documentation](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/daqhelp/daqhelp.html))
//! to APIs for both [ANSI C](https://www.ni.com/docs/en-US/bundle/ni-daqmx-c-api-ref/page/cdaqmx/help_file_title.html) and
//! [Python](https://nidaqmx-python.readthedocs.io).
//!
//! While NI provides fine-grained control over its hardware, existing drivers present the following challenges:
//!
//! ## Challenges with Existing Implementations
//!
//! ### 1. Streaming Deficiency
//! The NI driver, while versatile, demands that output signals be pre-sampled and relayed to the device's output-buffer.
//! Consider an experiment that runs for an extended duration (e.g., 10 minutes) and requires high time-resolution (e.g.,
//! 1MHz for 10 analogue f64 channels). Pre-sampling the entire waveform becomes both computationally demanding and
//! memory-intensive (requiring around ~44.7Gb for storage). A more practical approach would be streaming the signal,
//! where a fraction of the signal is sampled and relayed while the preceding chunk is executed. This approach reduces
//! memory and computational overhead while maintaining signal integrity.
//!
//! ### 2. Device-Centric Abstraction
//! NI drivers typically interface at the device level, with software "task" entities corresponding to specific device channels.
//! Modern experiments, however, often require capabilities that exceed a single NI card. Using a NI experimental control system
//! consisting of multiple devices necessitates managing multiple device tasks concurrently, a problem fraught with complexity.
//! Ideally, researchers should interface with the entire system holistically rather than grappling with individual devices
//! and their concurrent tasks. See [`Device`] for more details on synchronization.
//!
//! ### 3. Trade-offs between High vs. Low-Level Implementation
//! Low-level system implementations promise versatility and performance but at the expense of development ease. Conversely,
//! a Python-based solution encourages rapid development but may be marred by performance bottlenecks, especially when dealing
//! with concurrent streaming across multiple devices.
//!
//! ## Introducing `nicompiler_backend`
//!
//! `nicompiler_backend` is designed to bridge these challenges. At its core, it leverages the performance and safety
//! guarantees of Rust as well as its convenient interface with C and python. By interfacing seamlessly with the NI-DAQmx C
//! driver library and providing a Python API via `PyO3`, `nicompiler_backend` offers the best of both worlds.
//! Coupled with an optional high-level Python wrapper, researchers can design experiments
//! in an expressive language, leaving the Rust backend to handle streaming and concurrency.
//!
//! Currently, this crate supports analogue and digital output tasks, along with synchronization between NI devices through
//! shared start-triggers, sampling clocks, or phase-locked reference clocks.
//!
//! ## Example usage
//! ```
//! use nicompiler_backend::*;
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
//! // Compile the experiment: this will stop the experiment at the last edit-time plus one tick
//! exp.compile();
//!
//! // We can compile again with a specific stop_time (and add instructions in between)
//! exp.compile_with_stoptime(10.); // Experiment signal will stop at t=10 now
//! assert_eq!(exp.compiled_stop_time(), 10.);
//! ```
//!
//! # Navigating the Crate
//!
//! The `nicompiler_backend` crate is organized into primary modules - [`experiment`], [`device`], [`channel`], and [`instruction`].
//! Each serves specific functionality within the crate. Here's a quick guide to help you navigate:
//!
//! ### [`experiment`] Module: Your Starting Point
//!
//! If you're a typical user, you'll likely spend most of your time here.
//!
//! - **Overview**: An [`Experiment`] is viewed as a collection of devices, each identified by its name as recognized by the NI driver.
//! - **Usage**: The `Experiment` object is the primary entity exposed to Python. It provides methods for experiment-wide, device-wide, and channel-wide operations.
//! - **Key Traits & Implementations**: Refer to the [`BaseExperiment`] trait for Rust methods and usage examples. For Python method signatures, check the direct implementations in [`Experiment`], which simply wrap `BaseExperiment` implementations.
//!
//! ### [`device`] Module: Delving into Devices
//!
//! If you're keen on understanding or customizing device-specific details, this module is for you.
//!
//! - **Overview**: Each [`Device`] relates to a unique piece of NI hardware in the control system. It contains essential metadata such as physical names, sampling rates, and trigger behaviors.
//! - **Key Traits & Implementations**: See the [`BaseDevice`] trait and the entire [`device`] module for more insights. Devices also hold a set of channels, each referred to by its physical name.
//!
//! ### [`channel`] Module: Channel Instructions & Behaviors
//!
//! Ideal for those wanting to understand how instructions are managed or need to design a new [`TaskType`] as well as `TaskType`-specific customized channel behavior.
//!
//! - **Overview**: A [`Channel`] signifies a specific physical channel on an NI device. It administers a series of non-overlapping [`InstrBook`] which, after compilation, can be sampled to render floating-point signals.
//!
//! ### [`instruction`] Module: Deep Dive into Instructions
//!
//! For those interested in the intricacies of how instructions are defined and executed.
//!
//! - **Overview**: Each [`InstrBook`] holds an [`Instruction`] coupled with edit-time metadata, like `start_pos`, `end_pos`, and `keep_val`. An [`Instruction`] is crafted from an instruction type ([`InstrType`]) and a set of parameters in key-value pairs.
//!
//! We encourage users to explore each module to fully grasp the capabilities and structure of the crate. Whether you're here for a quick setup or to contribute, the `nicompiler_backend` crate is designed to cater to both needs.

use pyo3::prelude::*;
// use pyo3::wrap_pyfunction;

pub mod channel;
pub mod device;
pub mod experiment;
pub mod instruction;
pub mod utils;

pub use channel::*;
pub use device::*;
pub use experiment::*;
pub use instruction::*;
pub use utils::*;

#[pymodule]
fn nicompiler_backend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Experiment>()?;
    Ok(())
}
