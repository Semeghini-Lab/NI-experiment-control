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
//! memory overhead while retaining signal integrity.
//!
//! ### 2. Device-Centric Abstraction
//! NI drivers typically interface at the device level, with software "Task" entities corresponding to specific device channels.
//! Modern experiments, however, often require capabilities that exceed a single NI card. Using a NI experimental control system
//! consisting of multiple devices necessitates managing multiple device tasks concurrently, a problem fraught with complexity.
//! Ideally, researchers should interface with the entire system holistically rather than grappling with individual devices
//! and their concurrent tasks.
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
//! Coupled with an optional high-level Python wrapper (currently under development), researchers can design experiments
//! in an expressive language, leaving the Rust backend to handle streaming and concurrency.
//!
//! Currently, this crate supports analogue and digital output tasks, along with synchronization between NI devices through
//! shared start-triggers, sampling clocks, or phase-locked reference clocks.
//!
//! ## Crate Structure
//!
//! ### Experiment
//! An [`Experiment`] in `nicompiler_backend` is conceptualized as a collection
//! of devices, each identified by its physical name as recognized by the NI driver.
//!
//! ### Device
//! Each [`Device`] corresponds to a specific piece of NI hardware within the control system. Devices maintain metadata like
//! physical names, sampling rates, trigger behaviors, and more (detailed in [`Device`]). For implementation details, refer
//! to the [`BaseDevice`] trait and the [`device`] module. Furthermore, devices comprise a collection of channels, each indexed
//! by its physical name.
//!
//! ### Channel
//! A [`Channel`] represents a distinct physical channel on an NI device. Channels manage a list of non-overlapping [`InstrBook`],
//! which, post-compilation, can be sampled to produce floating-point signals.
//!
//! ### Instruction
//! Each [`InstrBook`] contains an [`Instruction`] paired with edit-time metadata like `start_pos`, `end_pos`, and `keep_val`.
//! An [`Instruction`] is defined by an instruction type ([`InstrType`]) and a set of parameters stored as key-value pairs.
//!
//! We invite you to delve deeper into the crate, explore its capabilities, and join us in refining and extending this
//! endeavor to make experimental control systems efficient and researcher-friendly.

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
