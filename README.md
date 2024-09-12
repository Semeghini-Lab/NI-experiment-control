# NI-experiment-control

A Python library offering an experiment-level abstraction of National Instrument (NI) devices, powered by a Rust backend.

National Instrument (NI) has consistently been a top choice for constructing experimental control systems, thanks to its versatile, cost-effective, extensible hardware, and comprehensive documentation. Their detailed guides range from system design ([NI-DAQmx Documentation](https://www.ni.com/docs/en-US/bundle/ni-daqmx/page/daqhelp/daqhelp.html)) to APIs for both [ANSI C](https://www.ni.com/docs/en-US/bundle/ni-daqmx-c-api-ref/page/cdaqmx/help_file_title.html) and [Python](https://nidaqmx-python.readthedocs.io).

## Challenges with Existing Implementations

### 1. Streaming Deficiency

The NI driver, although powerful, requires pre-sampling of output signals and their relay to the device's output-buffer. This becomes a challenge, especially for long-duration experiments that also demand high time-resolution. Streaming the signal, where a part of it is sampled and relayed while the earlier segment gets executed, can address this, reducing memory overhead and ensuring signal integrity.

### 2. Device-level Abstraction

NI drivers usually operate at the device level, associating software "task" entities to specific device channels. Modern experiments might demand the integration of multiple NI cards, complicating task management. A holistic system-level interface would be more intuitive than managing individual devices and tasks.

### 3. Trade-offs between High vs. Low-Level Implementation

While low-level implementations ensure versatility and performance, they complicate development. On the other hand, a Python-based approach simplifies development but can run into performance issues, particularly with concurrent multi-device streaming.

## Introducing `NI-experiment-control`

Our project aims to bridge these challenges. At its heart, it taps into Rust's performance and safety, coupled with its seamless C and Python interfacing. By integrating with the NI-DAQmx C driver library and offering a Python API through `PyO3`, we aim for a balance between performance and ease of use. The support for analogue and digital output tasks, along with synchronization capabilities, makes this project comprehensive for NI device integration.

### Why Rust?
The crux of our solution lies in the `nicompiler_backend` and `niexpctrl_backend` crates, designed in Rust. This choice allows us to:

- **Performance**: Rust, being a systems programming language, gives us close-to-metal performance, ensuring our backend's efficiency.
- **Safety**: Rust's strict compiler ensures memory safety without a garbage collector, preventing many common bugs.
- **Interoperability**: Rust's seamless interfacing with both C and Python allows for a robust backend with a flexible frontend.

## Structure of this project

### Python library wrapper (under development): `niexpctrl`

Located within the `niexpctrl` directory, this optional Python module provides user-friendly wrappers around the Python methods exposed by `nicompiler_backend` and, optionally, `niexpctrl_backend`. It's a comprehensive suite for designing, visualizing, and streaming multi-device NI output tasks.

### Experiment design: `nicompiler_backend`

`nicompiler_backend` is the core of our solution. Located within the `nicompiler_backend` subfolder, it serves as the bridge between NI hardware and our Python library. By leveraging [PyO3](https://github.com/PyO3/pyo3), this backend offers a Python-accessible `Experiment` class optimized for multi-device NI experiments. This library may be used on a device without physical connection to NI systems. Refer to the compiler backend for all general implementations. 

### Streaming capability: `niexpctrl_backend`

`niexpctrl_backend` is a lightweight extension built upon the `nicompiler_backend` implementing concrete streaming behavior. It can only be built on a Windows device with NI-DAQmx driver installed. 

For developers and those interested in a deep dive:

- **Rust API**: For comprehensive details on how the backend functions, you can explore up-to-date documentation by executing `cargo doc --open` within the `nicompiler_backend` or `niexpctrl_backend` directory. Alternatively, check out the published documentation for the [compiler](https://docs.rs/nicompiler_backend/latest/nicompiler_backend/) or the [`streaming extension`](https://docs.rs/niexpctrl_backend/latest/niexpctrl_backend/)
- **Python API**: This backend's methods have been integrated into our Python library. Check out the `niexpctrl` directory for a higher-level Python wrapper. 

## Installation

The instructions below are tailored for Windows, given NI's extensive driver support for the platform.

### Installing Rust 

1. **Download the Installer:** Navigate to the [official Rust download page](https://www.rust-lang.org/tools/install) and download the `rustup-init.exe`.
   
2. **Run the Installer:** Execute the `rustup-init.exe` and follow the on-screen instructions. This process will install Rust's package manager `cargo` as well.

3. **Verify Installation:** After installation, open a new command prompt and  verify installiation: 


    ```rustc --version && cargo --version```
4. **Update PATH (if necessary):** If you encounter errors indicating `rustc` or `cargo` is not recognized, ensure that the Rust binaries are added to your system's PATH. The installer typically does this, but in case it doesn't, add `C:\Users\<YOUR_USERNAME>\.cargo\bin` to your system's PATH.

## Example
As mentioned, users have the freedom to design and run experiments using: 
1. The higher-level `expctrl` Python wrapper library. 
2. Directly using PyO3 bindings for `nicompiler_backend` or `niexpctrl_backend` in Python. 
3. Using `nicompiler_backend` or `niexpctrl_backend` in Rust. 

We consider the simplest example of having one analogue-output (AO) and digital-output (DO) card synchronized via start triggers. 

### Python bindings
```Python
from nicompiler_backend import Experiment
#  Import the niexpctrl_backend to enable streaming on physical devices in 
#     addition to experiment design
#  from niexpctrl_backend import Experiment

exp = Experiment()
#  Define devices and associated channels
exp.add_ao_device(name="PXI1Slot3", samp_rate=1e6)
exp.add_ao_channel(name="PXI1Slot3", channel_id=0)

exp.add_do_device(name="PXI1Slot6", samp_rate=1e7)
exp.add_do_channel(name="PXI1Slot6", port_id=0, line_id=0)

exp.device_cfg_trig(name="PXI1Slot3", trig_line="PXI1_Trig0", export_trig=True)
exp.device_cfg_trig(name="PXI1Slot6", trig_line="PXI1_Trig0", export_trig=False)

# PXI1Slot3/ao0 starts with a 1s-long 7Hz sine wave with offset 1
#   and unit amplitude, zero phase. Does not keep its value.
exp.sine(dev_name="PXI1Slot3", chan_name="ao0", t=0., duration=1., keep_val=False,
         freq=7., dc_offset=1.)
# Ends with a half-second long 1V constant signal which returns to zero
exp.constant(dev_name="PXI1Slot3", chan_name="ao0", t=9., duration=0.5, value=1., keep_val=False)

# PXI1Slot6/port0/line0 start with a one-second "high" at t=0 and a half-second high at t=9
exp.high("PXI1Slot6", "port0/line0", t=0., duration=1.)
exp.high("PXI1Slot6", "port0/line0", t=9., duration=.5)

# Compile the experiment with stoptime=10s
exp.compile_with_stoptime(10.)
# Enabled by importing niexpctrl_backend in place of nicompiler_backend
# exp.stream_exp(stream_buftime=50., nreps=2)
```

### Rust
```Rust 
use nicompiler_backend::*;
// use niexpctrl_backend::*; 

fn main() {
    let mut exp = Experiment::new();
    exp.add_ao_device("PXI1Slot3", 1e6);
    exp.add_ao_channel("PXI1Slot3", 0); 

    exp.add_do_device("PXI1Slot6", 1e7);
    exp.add_do_channel("PXI1Slot6", 0, 0);

    exp.device_cfg_trig("PXI1Slot3", "PXI1_Trig0", true);
    exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", false);

    exp.sine("PXI1Slot3", "ao0", 0., 1., false, 7., None, None, Some(1.));
    exp.constant("PXI1Slot3", "ao0", 9., 0.5, 1., false);

    exp.high("PXI1Slot6", "port0/line0", 0., 1.);
    exp.high("PXI1Slot6", "port0/line0", 9., 0.5);

    exp.compile_with_stoptime(10.);

    // Allowed when using `niexpctrl_backend` instead of `nicompiler_backend`:
    //   Streams the experiment with a streaming buffer of 50ms twice
    // exp.stream_exp(50., 2); 
}
```


### Installing `nicompiler_backend`

1. Clone the repository and navigate to it.
2. Activate your desired Python (Anaconda) environment.
3. Navigate to the `nicompiler_backend` directory and execute `maturin develop --release` to install to the shell's python environment (ignore any warnings).
4. Run `pip show nicompiler_backend` to verify installation.

### Installing `niexpctrl_backend`

1. First, install the experiment compiler.
2. Download and install the [NI-DAQmx](https://www.ni.com/en/support/downloads/drivers/download.ni-daq-mx.html#484356) driver. 
3. Verify the presence of the NIDAQmx static library:
   ````C:/Program Files (x86)/National Instruments/Shared/ExternalCompilerSupport/C/lib64/msvc/NIDAQmx.lib````
   
   If not found, update the linker arguments in `niexpctrl_backend/.cargo/config.toml` with the correct path. 
4. Navigate to the `niexpctrl_backend` directory and run `maturin develop --release` to install to the shell's python environment (ignore any warnings).
5. Run `pip show niexpctrl_backend` to verify installation.
