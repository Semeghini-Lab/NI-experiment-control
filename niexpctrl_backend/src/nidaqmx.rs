//! Provides a minimal rust wrapper for parts of the NI-DAQmx C library.
//!
//! ## Overview
//!
//! The core of this module is the [`NiTask`] struct which represents an NI-DAQmx task. It encapsulates
//! a handle to an NI-DAQmx task and provides methods that map to various DAQmx C-functions, enabling
//! users to perform operations like creating analog or digital channels, configuring sampling rates,
//! and writing data to channels.
//!
//! Additionally, the module provides utility functions like [`daqmx_call`] and [`reset_ni_device`] to
//! simplify error handling and device interactions.
//!
//! **Refer to implementations of the [`NiTask`] struct to see the wrapped methods and invoked
//! [DAQmx C-functions](https://www.ni.com/docs/en-US/bundle/ni-daqmx-c-api-ref/page/cdaqmx/help_file_title.html)**
//!
//! ## Usage
//!
//! Typical usage involves creating an instance of the `NiTask` struct, configuring it (e.g., setting
//! up channels, setting clock rates), and then invoking operations (e.g., starting the task, writing
//! data). All operations are abstracted through safe Rust methods, ensuring type safety and reducing
//! the likelihood of runtime errors.
//!
//! ## Safety and Error Handling
//!
//! Given that this module interfaces with a C library, many of the calls involve unsafe Rust blocks.
//! To mitigate potential issues, this module provides the `daqmx_call` function that wraps DAQmx
//! C-function calls, checks for errors, and handles them appropriately (e.g., logging and panicking).
//! ***In addition to printing, NI-DAQmx driver errors are saved in `nidaqmx_error.logs` file in the
//! directory of the calling shell.
//!
//! ## Constants and Types
//!
//! To ensure type safety and clarity, the module defines several type aliases (e.g., `CConstStr`,
//! `CUint32`) and constants (e.g., `DAQMX_VAL_RISING`, `DAQMX_VAL_VOLTS`) that map to their C
//! counterparts. These are used throughout the module to ensure that function signatures and calls
//! match their expected types.
//!
//! ## Cleanup and Resource Management
//!
//! The `NiTask` struct implements the `Drop` trait, ensuring that resources (like the DAQmx task handle)
//! are cleaned up properly when an instance goes out of scope. This behavior reduces the chance of
//! resource leaks.
//!
//! ## External Dependencies
//!
//! This module depends on the `libc` crate for C types and the `ndarray` crate for multi-dimensional
//! arrays. It also uses the `std::fs` and `std::io` modules for file operations, specifically for logging
//! errors.
//!
//! ## Example
//!
//! ```ignore
//! # use niexpctrl_backend::*;
//! let task = NiTask::new();
//! task.create_ao_chan("Dev1/ao0");
//! task.cfg_sample_clk("", 1000.0, 1000);
//! // ... other configurations and operations ...
//! task.start();
//! // ... write data, wait, etc. ...
//! task.stop();
//! ```
//!
//! ## Further Reading
//!
//! For more details on the NI-DAQmx C driver and its capabilities, please refer to the
//! [NI-DAQmx C Reference](https://www.ni.com/docs/en-US/bundle/ni-daqmx-c-api-ref/page/cdaqmx/help_file_title.html).

use std::ffi::NulError;
use libc;
use ndarray::Array2;
use std::fs::OpenOptions;
use std::io::Write;

type CConstStr = *const libc::c_char;
type CCharBuf = *mut libc::c_char;
type CFloat64 = libc::c_double;
type CUint32 = libc::c_uint;
type CUint64 = libc::c_ulonglong;
type CBool32 = libc::c_uint;
type CInt32 = libc::c_int;
pub type TaskHandle = *mut libc::c_void;

pub const DAQMX_VAL_RISING: CInt32 = 10280;
pub const DAQMX_VAL_VOLTS: CInt32 = 10348;
pub const DAQMX_VAL_FINITESAMPS: CInt32 = 10178;
pub const DAQMX_VAL_DONOTALLOWREGEN: CInt32 = 10158;
pub const DAQMX_VAL_GROUPBYCHANNEL: CBool32 = 0;
pub const DAQMX_VAL_GROUPBYSCANNUMBER: CBool32 = 1;
pub const DAQMX_VAL_WAITINFINITELY: CFloat64 = -1.0;
pub const DAQMX_VAL_CHANPERLINE: CInt32 = 0;
pub const DAQMX_VAL_CHANFORALLLINES: CInt32 = 1;
pub const DAQMX_VAL_STARTTRIGGER: CInt32 = 12491;
pub const DAQMX_VAL_SAMPLECLOCK: CInt32 = 12487;
pub const DAQMX_VAL_10MHZREFCLOCK: CInt32 = 12536;
pub const DAQMX_VAL_DO_NOT_INVERT_POLARITY: CInt32 = 0;

#[link(name = "NIDAQmx")]
extern "C" {
    fn DAQmxResetDevice(name: CConstStr) -> CInt32;
    fn DAQmxGetExtendedErrorInfo(errorString: CCharBuf, bufferSize: CUint32) -> CInt32;

    fn DAQmxCreateTask(taskName: CConstStr, taskHandle_ptr: &mut TaskHandle) -> CInt32;
    fn DAQmxStartTask(handle: TaskHandle) -> CInt32;
    fn DAQmxStopTask(handle: TaskHandle) -> CInt32;
    fn DAQmxClearTask(handle: TaskHandle) -> CInt32;

    fn DAQmxWaitUntilTaskDone(handle: TaskHandle, timeToWait: CFloat64) -> CInt32;
    fn DAQmxSetWriteRegenMode(handle: TaskHandle, data: CInt32) -> CInt32;
    fn DAQmxCfgSampClkTiming(
        handle: TaskHandle,
        src: CConstStr,
        rate: CFloat64,
        activeEdge: CInt32,
        sampleMode: CInt32,
        sampsPerChan: CUint64,
    ) -> CInt32;
    fn DAQmxCfgOutputBuffer(handle: TaskHandle, numSampsPerChan: CUint32) -> CInt32;

    fn DAQmxCreateAOVoltageChan(
        handle: TaskHandle,
        name: CConstStr,
        assigned_name: CConstStr,
        minVal: CFloat64,
        maxVal: CFloat64,
        units: CInt32,
        customScaleName: CConstStr,
    ) -> CInt32;
    fn DAQmxCreateDOChan(
        handle: TaskHandle,
        lines: CConstStr,
        name: CConstStr,
        lineGrouping: CInt32,
    ) -> CInt32;

    fn DAQmxWriteDigitalU32(
        handle: TaskHandle,
        seqLen: CInt32,
        autoStart: CBool32,
        timeout: CFloat64,
        dataLayout: CBool32,
        writeArray: *const u32,
        sampsPerChanWritten: *mut CInt32,
        reserved: *mut CBool32,
    ) -> CInt32;
    fn DAQmxWriteDigitalLines(
        handle: TaskHandle,
        seqLen: CInt32,
        autoStart: CBool32,
        timeout: CFloat64,
        dataLayout: CBool32,
        writeArray: *const u8,
        sampsPerChanWritten: *mut CInt32,
        reserved: *mut CBool32,
    ) -> CInt32;
    fn DAQmxWriteAnalogF64(
        handle: TaskHandle,
        seqLen: CInt32,
        autoStart: CBool32,
        timeout: CFloat64,
        dataLayout: CBool32,
        writeArray: *const CFloat64,
        sampsPerChanWritten: *mut CInt32,
        reserved: *mut CBool32,
    ) -> CInt32;

    fn DAQmxConnectTerms(sourceTerminal: CConstStr, destinationTerminal: CConstStr, signalModifiers: CInt32) -> CInt32;
    fn DAQmxDisconnectTerms(sourceTerminal: CConstStr, destinationTerminal: CConstStr) -> CInt32;
    fn DAQmxExportSignal(handle: TaskHandle, signalID: CInt32, outputTerminal: CConstStr) -> CInt32;
    fn DAQmxSetRefClkSrc(handle: TaskHandle, src: CConstStr) -> CInt32;
    fn DAQmxSetRefClkRate(handle: TaskHandle, rate: CFloat64) -> CInt32;
    fn DAQmxCfgDigEdgeStartTrig(
        handle: TaskHandle,
        triggerSource: CConstStr,
        triggerEdge: CInt32,
    ) -> CInt32;
    fn DAQmxGetWriteCurrWritePos(handle: TaskHandle, data: *mut CUint64) -> CInt32;
    fn DAQmxGetWriteTotalSampPerChanGenerated(handle: TaskHandle, data: *mut CUint64) -> CInt32;
}

#[derive(Clone)]
pub struct DAQmxError {
    msg: String
}
impl DAQmxError {
    pub fn new(msg: String) -> Self {
        Self {msg}
    }
}
impl ToString for DAQmxError {
    fn to_string(&self) -> String {
        self.msg.clone()
    }
}
impl From<NulError> for DAQmxError {
    fn from(value: NulError) -> Self {
        DAQmxError::new(format!("Failed to convert '{}' to CString", value.to_string()))
    }
}

/// Calls a DAQmx C-function and handles potential errors.
///
/// This function is designed to automate the error handling for National Instruments (NI) DAQmx driver calls.
/// Every DAQmx C-function call returns a `int32` which, if negative, indicates an error.
/// It is used extensively by [`NiTask`] methods.
///
/// # Parameters
///
/// * `func`: A closure that encapsulates the DAQmx driver call. This closure should return a `CInt32`
/// which represents the result of the driver call.
///
/// # Behavior
///
/// If the DAQmx driver call (contained within `func`) returns a negative error code,
/// this function will automatically retrieve the extended error information using `DAQmxGetExtendedErrorInfo`.
/// It then writes the error to a log file named "nidaqmx_error.logs" and finally, panics with the error message.
///
/// # Examples
///
/// ```ignore
/// daqmx_call(|| {
///     // Your DAQmx driver call here
///     DAQmxSomeFunction(param1, param2)
/// });
/// ```
///
/// # Panics
///
/// This function will panic if:
/// * The DAQmx driver call returns a negative error code.
/// * There's a failure in opening or writing to the "nidaqmx_error.logs" file.
pub fn daqmx_call<F: FnOnce() -> CInt32>(func: F) -> Result<(), DAQmxError> {
    let status_code = func();
    if status_code >= 0 {
        Ok(())
    } else {
        let mut err_buff = [0i8; 2048];
        unsafe {
            DAQmxGetExtendedErrorInfo(err_buff.as_mut_ptr(), 2048 as CUint32);
        }
        let error_string = unsafe { std::ffi::CStr::from_ptr(err_buff.as_ptr()) }
            .to_string_lossy()
            .into_owned();

        // Write the error to log file
        let log_to_file = |err_msg: String| -> Result<(), std::io::Error> {
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open("./nidaqmx_error.logs")?;
            writeln!(file, "DAQmx Error: {error_string}")?;
            Ok(())
        };
        if log_to_file(error_string.clone()).is_err() {println!("Failed to write error to nidaqmx_error.logs")};
        // Return the error
        Err(DAQmxError::new(format!("DAQmx Error: {error_string}")))
    }
}

/// Resets a specified National Instruments (NI) device.
///
/// This function attempts to reset the provided NI device by invoking the `DAQmxResetDevice` method.
///
/// # Parameters
///
/// * `name`: A reference to a string slice representing the name of the NI device to be reset.
///
/// # Behavior
///
/// The function first converts the provided device name to a `CString` to ensure compatibility with the C-function call.
/// It then invokes the `daqmx_call` function to safely call the `DAQmxResetDevice` method.
///
/// # Safety
///
/// This function contains an unsafe block due to the direct interaction with the C library, specifically when calling the `DAQmxResetDevice` method.
///
/// # Example
/// ```ignore
/// # use niexpctrl_backend::*;
/// reset_ni_device("PXI1Slot3");
/// ```
///
/// # Panics
///
/// This function will panic if:
/// * There's a failure in converting the device name to a `CString`.
/// * The `DAQmxResetDevice` call returns a negative error code (handled by `daqmx_call`).
///
/// # Note
///
/// Ensure that the device name provided is valid and that the device is accessible when invoking this function.
pub fn reset_device(name: &str) -> Result<(), DAQmxError> {
    let name_cstr = std::ffi::CString::new(name)?;
    daqmx_call(|| unsafe { DAQmxResetDevice(name_cstr.as_ptr()) })
}
pub fn connect_terms(src: &str, dest: &str) -> Result<(), DAQmxError> {
    let src = std::ffi::CString::new(src)?;
    let dest = std::ffi::CString::new(dest)?;
    daqmx_call(|| unsafe { DAQmxConnectTerms(src.as_ptr(), dest.as_ptr(), DAQMX_VAL_DO_NOT_INVERT_POLARITY) })
}
pub fn disconnect_terms(src: &str, dest: &str) -> Result<(), DAQmxError> {
    let src = std::ffi::CString::new(src)?;
    let dest = std::ffi::CString::new(dest)?;
    daqmx_call(|| unsafe { DAQmxDisconnectTerms(src.as_ptr(), dest.as_ptr()) })
}

/// Represents a National Instruments (NI) DAQmx task.
///
/// `NiTask` encapsulates a handle to an NI-DAQmx task, providing a Rust-friendly interface to interact with the task.
/// Creating an instance of this struct corresponds to creating a new NI-DAQmx task. Methods on the struct
/// allow for invoking the associated DAQmx methods on the task.
///
/// The struct primarily holds a task handle, represented by the `handle` field, which is used for internal
/// operations and interactions with the DAQmx C API.
///
/// # NI-DAQmx Reference
///
/// For detailed information about the underlying driver and its associated methods, refer to the
/// [NI-DAQmx C Reference](https://www.ni.com/docs/en-US/bundle/ni-daqmx-c-api-ref/page/cdaqmx/help_file_title.html).
///
/// # Examples
///
/// ```ignore
/// let task = NiTask::new();
/// // task.some_method();
/// ```
///
/// # Note
///
/// Ensure you have the necessary NI-DAQmx drivers and libraries installed and accessible when using this struct and its associated methods.
pub struct NiTask {
    handle: TaskHandle,
}

impl NiTask {
    pub fn new() -> Result<Self, DAQmxError> {
        let mut taskhandle: TaskHandle = std::ptr::null_mut();
        let task_name_cstr = std::ffi::CString::new("")?;
        daqmx_call(|| unsafe { DAQmxCreateTask(task_name_cstr.as_ptr(), &mut taskhandle) })?;
        Ok(Self { handle: taskhandle })
    }

    pub fn clear(&self) -> Result<(), DAQmxError> {
        daqmx_call(|| unsafe { DAQmxClearTask(self.handle) })
    }
    pub fn start(&self) -> Result<(), DAQmxError> {
        daqmx_call(|| unsafe { DAQmxStartTask(self.handle) })
    }
    pub fn stop(&self) -> Result<(), DAQmxError> {
        daqmx_call(|| unsafe { DAQmxStopTask(self.handle) })
    }
    pub fn wait_until_done(&self, timeout: Option<f64>) -> Result<(), DAQmxError> {
        let timeout = match timeout {
            Some(timeout) => timeout as CFloat64,
            None => DAQMX_VAL_WAITINFINITELY,
        };
        daqmx_call(|| unsafe { DAQmxWaitUntilTaskDone(self.handle, timeout) })
    }
    pub fn disallow_regen(&self) -> Result<(), DAQmxError> {
        daqmx_call(|| unsafe { DAQmxSetWriteRegenMode(self.handle, DAQMX_VAL_DONOTALLOWREGEN) })
    }

    pub fn cfg_samp_clk_timing(&self, clk_src: &str, samp_rate: f64, seq_len: u64) -> Result<(), DAQmxError> {
        let src_cstring = std::ffi::CString::new(clk_src)?;
        daqmx_call(|| unsafe {
            DAQmxCfgSampClkTiming(
                self.handle,
                src_cstring.as_ptr(),
                samp_rate as CFloat64,
                DAQMX_VAL_RISING,
                DAQMX_VAL_FINITESAMPS,
                seq_len as CUint64,
            )
        })
    }

    pub fn cfg_output_buffer(&self, buf_size: usize) -> Result<(), DAQmxError> {
        daqmx_call(|| unsafe { DAQmxCfgOutputBuffer(self.handle, buf_size as CUint32) })
    }

    pub fn create_ao_chan(&self, name: &str) -> Result<(), DAQmxError> {
        let name_cstr = std::ffi::CString::new(name)?;
        let assigned_name_cstr = std::ffi::CString::new("")?;
        daqmx_call(|| unsafe {
            DAQmxCreateAOVoltageChan(
                self.handle,
                name_cstr.as_ptr(),
                assigned_name_cstr.as_ptr(),
                -10.,
                10.,
                DAQMX_VAL_VOLTS,
                std::ptr::null(),
            )
        })
    }

    pub fn create_do_chan(&self, name: &str) -> Result<(), DAQmxError> {
        let name_cstr = std::ffi::CString::new(name)?;
        let assigned_name_cstr = std::ffi::CString::new("")?;
        daqmx_call(|| unsafe {
            DAQmxCreateDOChan(
                self.handle,
                name_cstr.as_ptr(),
                assigned_name_cstr.as_ptr(),
                DAQMX_VAL_CHANFORALLLINES,
            )
        })
    }

    pub fn write_digital_port(&self, samp_arr: &Array2<u32>, timeout: Option<f64>) -> Result<usize, DAQmxError> {
        let timeout = match timeout {
            Some(timeout) => timeout as CFloat64,
            None => DAQMX_VAL_WAITINFINITELY,
        };
        let mut nwritten: CInt32 = 0;
        daqmx_call(|| unsafe {
            DAQmxWriteDigitalU32(
                self.handle,
                samp_arr.shape()[1] as CInt32,
                false as CBool32,
                timeout,
                DAQMX_VAL_GROUPBYCHANNEL,
                samp_arr.as_ptr(),
                &mut nwritten as *mut CInt32,
                std::ptr::null_mut(),
            )
        })?;
        Ok(nwritten as usize)
    }

    pub fn write_digital_lines(&self, samp_arr: &Array2<u8>, timeout: Option<f64>) -> Result<usize, DAQmxError> {
        let timeout = match timeout {
            Some(timeout) => timeout as CFloat64,
            None => DAQMX_VAL_WAITINFINITELY,
        };
        let mut nwritten: CInt32 = 0;
        daqmx_call(|| unsafe {
            DAQmxWriteDigitalLines(
                self.handle,
                samp_arr.shape()[1] as CInt32,
                false as CBool32,
                timeout,
                DAQMX_VAL_GROUPBYCHANNEL,
                samp_arr.as_ptr(),
                &mut nwritten as *mut CInt32,
                std::ptr::null_mut(),
            )
        })?;
        Ok(nwritten as usize)
    }

    pub fn write_analog(&self, samp_arr: &Array2<f64>, timeout: Option<f64>) -> Result<usize, DAQmxError> {
        let timeout = match timeout {
            Some(timeout) => timeout as CFloat64,
            None => DAQMX_VAL_WAITINFINITELY,
        };
        let mut nwritten: CInt32 = 0;
        daqmx_call(|| unsafe {
            DAQmxWriteAnalogF64(
                self.handle,
                samp_arr.shape()[1] as CInt32,
                false as CBool32,
                timeout,
                DAQMX_VAL_GROUPBYCHANNEL,
                samp_arr.as_ptr(),
                &mut nwritten as *mut CInt32,
                std::ptr::null_mut(),
            )
        })?;
        Ok(nwritten as usize)
    }

    pub fn set_ref_clk_rate(&self, rate: f64) -> Result<(), DAQmxError> {
        daqmx_call(|| unsafe { DAQmxSetRefClkRate(self.handle, rate as CFloat64) })
    }

    pub fn set_ref_clk_src(&self, src: &str) -> Result<(), DAQmxError> {
        let clk_src_cstr = std::ffi::CString::new(src)?;
        daqmx_call(|| unsafe { DAQmxSetRefClkSrc(self.handle, clk_src_cstr.as_ptr()) })
    }

    pub fn cfg_ref_clk(&self, src: &str, rate: f64) -> Result<(), DAQmxError> {
        self.set_ref_clk_rate(rate)?;
        self.set_ref_clk_src(src)?;
        Ok(())
    }

    pub fn cfg_dig_edge_start_trigger(&self, trigger_source: &str) -> Result<(), DAQmxError> {
        let trigger_source_cstr = std::ffi::CString::new(trigger_source)?;
        daqmx_call(|| unsafe {
            DAQmxCfgDigEdgeStartTrig(self.handle, trigger_source_cstr.as_ptr(), DAQMX_VAL_RISING)
        })
    }

    pub fn get_write_current_write_pos(&self) -> Result<u64, DAQmxError> {
        let mut data: CUint64 = 0;
        daqmx_call(|| unsafe { DAQmxGetWriteCurrWritePos(self.handle, &mut data as *mut CUint64) })?;
        Ok(data as u64)
    }

    pub fn export_signal(&self, signal_id: CInt32, output_terminal: &str) -> Result<(), DAQmxError> {
        let output_terminal_cstr = std::ffi::CString::new(output_terminal)?;
        daqmx_call(|| unsafe {
            DAQmxExportSignal(self.handle, signal_id, output_terminal_cstr.as_ptr())
        })
    }

    pub fn get_write_total_samp_per_chan_generated(&self) -> Result<u64, DAQmxError> {
        let mut data: CUint64 = 0;
        daqmx_call(|| unsafe {
            DAQmxGetWriteTotalSampPerChanGenerated(self.handle, &mut data as *mut CUint64)
        })?;
        Ok(data as u64)
    }
}

// Define deletion behavior
impl Drop for NiTask {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}
