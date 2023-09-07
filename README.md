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

### Python library wrapper: `niexpctrl`

Located within the `niexpctrl` directory, this optional Python module provides user-friendly wrappers around the Python methods exposed by `nicompiler_backend` and, optionally, `niexpctrl_backend`. It's a comprehensive suite for designing, visualizing, and streaming multi-device NI output tasks.

### Experiment design: `nicompiler_backend`

`nicompiler_backend` is the core of our solution. Located within the `nicompiler_backend` subfolder, it serves as the bridge between NI hardware and our Python library. By leveraging [PyO3](https://github.com/PyO3/pyo3), this backend offers a Python-accessible `Experiment` class optimized for multi-device NI experiments.

For developers and those interested in a deep dive:

- **Rust API**: For comprehensive details on how the backend functions, you can explore the crate's documentation by executing `cargo doc --open` within its directory. Alternatively, check out the [published documentation](https://docs.rs/nicompiler_backend/0.2.0/nicompiler_backend/#).
- **Python API**: This backend's methods have been seamlessly integrated into our Python library. Check out the `niexpctrl` directory for a higher-level Python wrapper. 

## Installation

The instructions below are tailored for Windows, given NI's extensive driver support for the platform.

### Installing Rust 

1. **Download the Installer:** Navigate to the [official Rust download page](https://www.rust-lang.org/tools/install) and download the `rustup-init.exe`.
   
2. **Run the Installer:** Execute the `rustup-init.exe` and follow the on-screen instructions. This process will install Rust's package manager `cargo` as well.

3. **Verify Installation:** After installation, open a new command prompt and  verify installiation: 


    ```rustc --version && cargo --version```
4. **Update PATH (if necessary):** If you encounter errors indicating `rustc` or `cargo` is not recognized, ensure that the Rust binaries are added to your system's PATH. The installer typically does this, but in case it doesn't, add `C:\Users\<YOUR_USERNAME>\.cargo\bin` to your system's PATH.



### Installing `nicompiler_backend`

1. Clone the repository and navigate to it.
2. Activate your desired Python (Anaconda) environment.
3. Navigate to the `nicompiler_backend` directory and execute `make export_optimized` to install.
4. (Placeholder for `niexpctrl` installation steps)

### Installing `niexpctrl_backend`

1. First, install the experiment compiler.
2. Download and install the [NI-DAQmx](https://www.ni.com/en/support/downloads/drivers/download.ni-daq-mx.html#484356) driver.
3. Verify the presence of the NIDAQmx static library:


   ````C:/Program Files (x86)/National Instruments/Shared/ExternalCompilerSupport/C/lib64/msvc/NIDAQmx.lib````
   
   If not found, update the linker arguments in `niexpctrl_backend/.cargo/config.toml` with the correct path. 
4. Navigate to the `niexpctrl_backend` directory and run `make export_optimized` to install. 
5. (Placeholder for `niexpctrl` installation steps)

## Extend functionalities