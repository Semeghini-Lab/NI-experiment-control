# NI-experiment-control
A python library for experiment-level abstraction of National Instrument (NI) devices with a rust backend. 

# National Instrument (NI) Integration

National Instrument (NI) has long been a preferred choice for building experimental control systems, owing to the versatility, cost-effectiveness, extensibility, and robust documentation of its hardware. Their substantial documentation spans from system design [NI-DAQmx Documentation](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/daqhelp/daqhelp.html) to APIs for both [ANSI C](https://www.ni.com/docs/en-US/bundle/ni-daqmx-c-api-ref/page/cdaqmx/help_file_title.html) and [Python](https://nidaqmx-python.readthedocs.io).

## Challenges with Existing Implementations

### 1. Streaming Deficiency
The NI driver, while versatile, demands that output signals be pre-sampled and relayed to the device's output-buffer. Consider an experiment that runs for an extended duration (e.g., 10 minutes) and requires high time-resolution (e.g., 1MHz for 10 analogue f64 channels). Pre-sampling the entire waveform becomes both computationally demanding and memory-intensive (requiring around `~44.7Gb` for storage). A more practical approach would be streaming the signal, where a fraction of the signal is sampled and relayed while the preceding chunk is executed. This approach reduces memory overhead while retaining signal integrity.

### 2. Device-Centric Abstraction
NI drivers typically interface at the device level, with software "task" entities corresponding to specific device channels. Modern experiments, however, often require capabilities that exceed a single NI card. Using a NI experimental control system consisting of multiple devices necessitates managing multiple device tasks concurrently, a problem fraught with complexity. Ideally, researchers should interface with the entire system holistically rather than grappling with individual devices and their concurrent tasks.

### 3. Trade-offs between High vs. Low-Level Implementation
Low-level system implementations promise versatility and performance but at the expense of development ease. Conversely, a Python-based solution encourages rapid development but may be marred by performance bottlenecks, especially when dealing with concurrent streaming across multiple devices.

## Introducing `NI-experiment-control`

This project is designed to bridge these challenges. At its core, it leverages the performance and safety guarantees of Rust as well as its convenient interface with C and python. By interfacing seamlessly with the NI-DAQmx C driver library and providing a Python API via `PyO3`, we seek the best of both worlds. Coupled with an optional high-level Python wrapper module, researchers can design experiments in an expressive language, leaving the Rust backend to handle streaming and concurrency.

Currently, this crate supports analogue and digital output tasks, along with synchronization between NI devices through shared start-triggers, sampling clocks, or phase-locked reference clocks.

## Code Structure

### `niexpctrl`
The `niexpctrl` folder features an optional python module of the same name providing convenient wrappers around the python methods exposed in `nicompiler_backend` and, optionally, `niexpctrl_backend` if it is found. It provides a complete set of functionalities for conveniently designing, visualizing, and streaming multi-device NI output tasks. 

### `nicompiler_backend`
The `nicompiler_backend` folder features a rust crate of the same name. Via `PyO3`, it exposes a python-accessible `Experiment` 
class through which to define a multi-device NI experiment. The `nicompiler_backend` may also be used as a standalone python or rust library in an experiment-design environment without NI devices. 

### `niexpctrl_backend`
The `niexpctrl_backend` crate extends on the `nicompiler_backend` crate to provide a python-accessible `Experiment` class which can additionally be streamed to NI devices. 

## Installation
The following installation instruction is for Windows, for which NI provides the most comprehensive driver support. 
### Installing Rust
(fill in details here)
### Installing the experiment compiler
1. clone the repository and enter the repository
2. Activate the python (anaconda) environment in which you wish to install `nicompiler_backend`. 
3. Run `cd nicompiler_backend && make export_optimized` to install the `nicompiler_backend`.
4. [placeholder for installing `niexpctrl`]

### Installing the experimental control component
1. Install the experiment compiler.
2. 