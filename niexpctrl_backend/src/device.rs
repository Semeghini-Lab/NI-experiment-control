//! Implements the [`StreamableDevice`] trait for [`nicompiler_backend::Device`] to support streaming
//! operations on NI hardware.
//!
//! This module serves as a bridge between the backend logic and the NI-DAQ hardware, ensuring seamless
//! streaming operations and synchronized behavior across devices. It implements the specifications of
//! [`nicompiler_backend::device`] to translates compiled instructions within a device into NI-DAQmx driver
//! instructions, while ensuring synchronized and efficient streaming.
//!
//! ## Overview:
//!
//! The [`StreamableDevice`] trait extends the [`nicompiler_backend::BaseDevice`] trait,
//! adding [`StreamableDevice::stream_task`] that allow for the streaming of instruction signals
//! onto specified NI-DAQ devices.
//!
//! ## Key Components:
//!
//! - [`StreamableDevice`] Trait: The primary trait that encapsulates the extended functionality. It defines methods
//!   to stream signals, configure task channels, and set up synchronization and clocking.
//! - Helper Methods: Helper methods like `cfg_task_channels` and `cfg_clk_sync` within the trait.
//!   simplify the device configuration process.
//!
//! ## Features:
//!
//! - **Streaming**: The primary feature, allowing for the streaming of instruction signals to NI-DAQ devices.
//! - **Synchronization**: Ensures that multiple devices can operate in a synchronized manner, especially
//!   crucial when there's a primary and secondary device setup.
//! - **Clock Configuration**: Sets up the sample clock, start trigger, and reference clocking for devices.
//! - **Task Channel Configuration**: Configures the task channels based on the device's task type.

use crate::nidaqmx::*;
use crate::utils::{Semaphore, StreamCounter};

use std::sync::Arc;

use nicompiler_backend::*;

/// The `StreamableDevice` trait extends the [`nicompiler_backend::BaseDevice`] trait of [`nicompiler_backend::Device`]
/// to provide additional functionality for streaming tasks.
pub trait StreamableDevice: BaseDevice + Sync + Send {
    /// Streams an instruction signal to the specified NI-DAQ device.
    ///
    /// This method is responsible for streaming an instruction signal to a National Instruments (NI) DAQ device
    /// represented by `self`. It sets up a new NI-DAQmx task, configures synchronization methods and buffer,
    /// writes the initial chunk of the sequence into the driver buffer, and starts the task, causing the device
    /// to output the signal.
    ///
    /// # Parameters
    ///
    /// * `sem`: A semaphore used to synchronize the start triggers between multiple devices. Ensures that threads
    ///   for secondary devices always start listening for triggers before the primary device starts and exports
    ///   its start trigger.
    /// * `num_devices`: The total number of NI-DAQ devices involved in the streaming process.
    /// * `stream_buftime`: Duration (in milliseconds) specifying the length of the streaming buffer.
    /// * `nreps`: Number of repetitions for streaming the sequence. Streaming a sequence multiple times in a single
    ///   call using `nreps` is more efficient than multiple separate calls.
    ///
    /// # Behavior
    ///
    /// 1. Asserts that the device has been compiled using `is_compiled`.
    /// 2. Initializes a new `NiTask` and configures the device channels.
    /// 3. Configures the buffer, writing method, clock, and synchronization.
    /// 4. Writes the initial chunk of the sequence into the driver buffer.
    /// 5. Starts the task, causing the device to output the signal.
    /// 6. Continuously streams chunks of the sequence to the device until the entire sequence has been streamed.
    /// 7. If `nreps` > 1, the sequence is streamed the specified number of times.
    ///
    /// The method uses a `TickTimer` to measure the time taken for various operations, which can be helpful for
    /// performance analysis.
    ///
    /// # Safety and Synchronization
    ///
    /// The method uses a semaphore (`sem`) to ensure synchronization between multiple devices. Specifically, it ensures
    /// that secondary devices start listening for triggers before the primary device starts and exports its start trigger.
    ///
    /// # Note
    ///
    /// The method relies on various helper functions and methods, such as `is_compiled`, `cfg_task_channels`, and
    /// `calc_signal_nsamps`, to achieve its functionality. Ensure that all dependencies are correctly set up and
    /// that the device has been properly compiled before calling this method.
    fn stream_task(
        &self,
        sem: &Arc<Semaphore>,
        num_devices: usize,
        stream_buftime: f64,
        nreps: usize,
    ) {
        let mut timer = TickTimer::new();
        let mut timer_ = TickTimer::new();

        assert!(
            self.is_compiled(),
            "Compile device {} before streaming",
            self.name()
        );

        // (Not-done) trick: in principle, calculation of the first signal can be done independently of daqmx setup
        // Still have to figure out how to do in rust.
        let seq_len = self.total_samps();  // ToDo: the previous approach was dangerous, truncation may strip the final tick off. Consider using `self.compiled_stop_pos()` or `.round()`
        let buffer_size = std::cmp::min(
            seq_len,
            (stream_buftime * self.samp_rate() / 1000.) as usize,
        );
        let mut counter = StreamCounter::new(seq_len, buffer_size);
        let (mut start_pos, mut end_pos) = counter.tick_next();

        // DAQmx Setup
        let task = NiTask::new();
        self.cfg_task_channels(&task);

        // Configure buffer, writing method, clock and sync
        task.cfg_output_buffer(buffer_size);
        task.disallow_regen();
        let bufwrite = |signal| {
            match self.task_type() {
                TaskType::AO => task.write_analog(&signal),
                TaskType::DO => task.write_digital_port(&signal.map(|&x| x as u32)),
            };
        };
        self.cfg_clk_sync(&task, &seq_len);
        timer.tick_print(&format!(
            "{} cfg (task channels, buffers, clk & sync)",
            self.name()
        ));

        // Obtain the first signal (optional: from parallel thread), and do first bufwrite
        let signal = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
        timer.tick_print(&format!("{} wait to receive signal", self.name()));
        bufwrite(signal);
        timer.tick_print(&format!("{} bufwrite", self.name()));

        for _rep in 0..nreps {
            // For every repetition, make sure the primary device starts last
            if self.export_trig().unwrap_or(false) {
                (0..num_devices).for_each(|_| sem.acquire());
                sem.release(); // Release the semaphore to restore count to 1, in preparation for the next run.
            }
            task.start();
            timer_.tick_print(&format!("{} start (restart) overhead", self.name()));
            if !self.export_trig().unwrap_or(true) {
                sem.release();
            }
            // Main chunk for streaming
            while end_pos != seq_len {
                (start_pos, end_pos) = counter.tick_next();
                let signal_stream =
                    self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
                bufwrite(signal_stream);
            }
            if nreps > 1 {
                // If we're on repeat: don't wait for the task to finish, calculate and write the next chunk
                (start_pos, end_pos) = counter.tick_next();
                let signal_next_start =
                    self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
                task.wait_until_done(stream_buftime * 10. / 1000.);
                timer_.tick_print(&format!("{} end", self.name()));
                task.stop();
                bufwrite(signal_next_start);
            } else {
                task.wait_until_done(stream_buftime * 10. / 1000.);
                timer_.tick_print(&format!("{} end", self.name()));
                task.stop();
            }
            if self.export_ref_clk().unwrap_or(false) { 
                (0..num_devices).for_each(|_| sem.acquire());
            }
            sem.release();
        }
    }

    /// Helper function that configures the task channels for the device.
    ///
    /// This method is a helper utility designed to configure the task channels based on the device's `task_type`.
    /// It invokes the corresponding DAQmx driver method to set up the channels, ensuring they are correctly initialized
    /// for subsequent operations. This method is invoked by [`StreamableDevice::stream_task`].
    ///
    /// # Parameters
    ///
    /// * `task`: A reference to the `NiTask` instance representing the task to be configured.
    ///
    /// # Behavior
    ///
    /// Depending on the device's `task_type`, the method will:
    /// * For `TaskType::AO`: Iterate through the compiled, streamable channels and invoke the
    /// `create_ao_chan` method for each channel.
    /// * For `TaskType::DO`: Iterate through the compiled, streamable channels and invoke the
    /// `create_do_chan` method for each channel.
    ///
    /// The channel names are constructed using the format `/{device_name}/{channel_name}`.
    fn cfg_task_channels(&self, task: &NiTask) {
        match self.task_type() {
            TaskType::AO => {
                // Require compiled, streamable channels
                self.compiled_channels(true, false).iter().for_each(|chan| {
                    task.create_ao_chan(&format!("/{}/{}", &self.name(), chan.name()));
                });
            }
            TaskType::DO => {
                self.compiled_channels(true, false).iter().for_each(|chan| {
                    task.create_do_chan(&format!("/{}/{}", &self.name(), chan.name()));
                });
            }
        }
    }

    /// Configures the synchronization and clock behavior for the device.
    ///
    /// This method sets up the synchronization behavior of the device, ensuring that its operation is correctly
    /// coordinated with other devices or tasks. It configures the sample clock, start trigger, and reference clocking.
    /// This method is invoked by [`StreamableDevice::stream_task`].
    ///
    /// Refer to [`nicompiler_backend::Device`] for a detailed explanation of synchronization mechanisms and their importance.
    ///
    /// # Parameters
    ///
    /// * `task`: A reference to the `NiTask` instance representing the task to be synchronized.
    /// * `seq_len`: A reference to the length of the sequence for which synchronization is required.
    ///
    /// # Behavior
    ///
    /// 1. Configures the sample clock using the provided `samp_clk_src` and `samp_rate`.
    /// 2. If the device has a trigger line, it configures the start trigger. Primary devices will export the start trigger,
    ///    while secondary devices will configure their tasks to expect the start trigger.
    /// 3. Configures reference clocking based on the device's `ref_clk_line`. Devices that import the reference clock will
    ///    configure it accordingly, while others will export the signal.
    fn cfg_clk_sync(&self, task: &NiTask, seq_len: &usize) {
        let seq_len = *seq_len as u64;
        // Configure sample clock first
        let samp_clk_src = self.samp_clk_src().unwrap_or("");
        task.cfg_sample_clk(samp_clk_src, self.samp_rate(), seq_len);
        // Configure start trigger: primary devices export, while secondary devices configure task
        // to expect start trigger
        if let Some(trig_line) = self.trig_line() {
            match self.export_trig().unwrap() {
                true => task.export_signal(
                    DAQMX_VAL_STARTTRIGGER,
                    &format!("/{}/{}", &self.name(), trig_line),
                ),
                false => {
                    task.cfg_dig_edge_start_trigger(&format!("/{}/{}", &self.name(), trig_line,))
                }
            }
        };
        // Configure reference clock behavior
        if let Some(ref_clk_line) = self.ref_clk_line() {
            match self.export_ref_clk().unwrap() {
                false => task.cfg_ref_clk(ref_clk_line, self.ref_clk_rate().unwrap()),
                true => {},  // task.export_signal(DAQMX_VAL_10MHZREFCLOCK, ref_clk_line)
            };
        }
    }
}

impl StreamableDevice for Device {}
