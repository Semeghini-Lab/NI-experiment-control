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
use std::sync::mpsc::{Sender, Receiver, SendError, RecvError};
use ndarray::Array2;

use nicompiler_backend::*;
use crate::worker_cmd_chan::{CmdRecvr, WorkerCmd};

pub struct WorkerError {
    msg: String
}
impl From<SendError<()>> for WorkerError {
    fn from(_value: SendError<()>) -> Self {
        Self {
            msg: "Worker thread encountered SendError".to_string()
        }
    }
}
impl From<RecvError> for WorkerError {
    fn from(_value: RecvError) -> Self {
        Self {
            msg: "Worker encountered RecvError".to_string()
        }
    }
}
impl From<DAQmxError> for WorkerError {
    fn from(value: DAQmxError) -> Self {
        Self{msg: value.to_string()}
    }
}
impl From<String> for WorkerError {
    fn from(value: String) -> Self {
        Self {
            msg: format!("Worker thread encountered the following error: \n{value}")
        }
    }
}
impl ToString for WorkerError {
    fn to_string(&self) -> String {
        self.msg.clone()
    }
}

pub enum StartSync {
    Primary(Vec<Receiver<()>>),
    Secondary(Sender<()>),
    None
}

pub struct StreamBundle {
    task_type: TaskType,
    ni_task: NiTask,
    counter: StreamCounter,
    buf_write_timeout: Option<f64>,  // Some(finite_timeout_in_seconds) or None - wait infinitely
}
impl StreamBundle {
    fn write_buf(&self, samp_arr: Array2<f64>) -> Result<usize, DAQmxError> {
        match self.task_type {
            TaskType::AO => self.ni_task.write_analog(
                &samp_arr,
                self.buf_write_timeout.clone()
            ),
            TaskType::DO => self.ni_task.write_digital_port(
                &samp_arr.map(|&x| x as u32),
                self.buf_write_timeout.clone()
            ),
        }
    }
}

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
    /* === The original version ===
    fn stream_task(
        &self,
        sem: &Arc<Semaphore>,
        num_devices: usize,
        bufsize_ms: f64,
        nreps: usize,
    ) {
        let mut timer1 = TickTimer::new();
        let mut timer2 = TickTimer::new();

        assert!(
            self.is_compiled(),
            "Compile device {} before streaming",
            self.name()
        );

        // (Not-done) trick: in principle, calculation of the first signal can be done independently of daqmx setup
        // Still have to figure out how to do in rust.
        let buf_dur = bufsize_ms / 1000.0;
        let seq_len = self.total_samps();
        let buf_size = std::cmp::min(
            seq_len,
            (buf_dur * self.samp_rate()).round() as usize,
        );
        let mut counter = StreamCounter::new(seq_len, buf_size);
        let (mut start_pos, mut end_pos) = counter.tick_next();

        // DAQmx Setup
        let task = NiTask::new();
        self.cfg_task_channels(&task);

        // Configure buffer, writing method, clock and sync
        task.cfg_output_buffer(buf_size);
        task.disallow_regen();
        let bufwrite = |signal| {
            match self.task_type() {
                TaskType::AO => task.write_analog(&signal),
                TaskType::DO => task.write_digital_port(&signal.map(|&x| x as u32)),
            };
        };
        self.cfg_clk_sync(&task, &seq_len);
        timer1.tick_print(&format!(
            "{} cfg (task channels, buffers, clk & sync)",
            self.name()
        ));

        // Obtain the first signal (optional: from parallel thread), and do first bufwrite
        let signal = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
        timer1.tick_print(&format!("{} calc initial sample chunk", self.name()));
        bufwrite(signal);
        timer1.tick_print(&format!("{} initial bufwrite", self.name()));

        for _rep in 0..nreps {
            // For every repetition, make sure the primary device starts last
            match self.export_trig() {
                // The primary device waits for all others to flag they started their tasks
                Some(true) => {
                    (0..num_devices).for_each(|_| sem.acquire());
                    sem.release(); // Release the semaphore to restore count to 1, in preparation for the next run.
                },
                _ => {}
            }
            task.start();
            timer2.tick_print(&format!("{} start (restart) overhead", self.name()));
            match self.export_trig() {
                // All non-primary devices (both trigger users, and the ones not using trigger at all)
                // should flag they have started their task
                Some(true) => {},
                _ => sem.release()
            }

            // Main streaming loop
            while end_pos != seq_len {
                (start_pos, end_pos) = counter.tick_next();
                let signal_stream = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
                bufwrite(signal_stream);
                // FixMe: add timeout = max(1 second, 2*buf_dur) to avoid deadlocks (hardware bug - trigger not connected -> deadlock).
                //  Also add option to specify WaitInfinitely for advanced cases (external sample clock freezing and external trigger)
            }

            // Finishing this streaming run:
            if nreps > 1 {
                // If we're on repeat: don't wait for the task to finish, calculate and write the next chunk
                (start_pos, end_pos) = counter.tick_next();
                let signal_next_start = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
                task.wait_until_done(buf_dur * 10.0);
                timer2.tick_print(&format!("{} end", self.name()));
                task.stop();
                bufwrite(signal_next_start);
            } else {
                task.wait_until_done(buf_dur * 10.0);
                timer2.tick_print(&format!("{} end", self.name()));
                task.stop();
            }
        }
    }
    */

    fn worker_loop(
        &mut self,
        bufsize_ms: f64,
        mut cmd_recvr: CmdRecvr,
        report_sendr: Sender<()>,
        start_sync: StartSync,
    ) -> Result<(), WorkerError> {
        let mut stream_bundle = self.cfg_run_(bufsize_ms)?;
        report_sendr.send(())?;

        loop {
            match cmd_recvr.recv()? {
                WorkerCmd::Stream(calc_next) => {
                    self.stream_run_(&mut stream_bundle, &start_sync, calc_next)?;
                    report_sendr.send(())?;
                },
                WorkerCmd::Clear => {
                    break
                }
            }
        };
        Ok(())
    }
    fn cfg_run_(&self, bufsize_ms: f64) -> Result<StreamBundle, WorkerError> {
        let buf_dur = bufsize_ms / 1000.0;
        let buf_write_timeout = match self.get_min_bufwrite_timeout() {
            Some(min_timeout) => Some(f64::max(10.0*buf_dur, min_timeout)),
            None => None,
        };

        let seq_len = self.total_samps();
        let buf_size = std::cmp::min(
            seq_len,
            (buf_dur * self.samp_rate()).round() as usize,
        );
        let mut counter = StreamCounter::new(seq_len, buf_size);

        // DAQmx Setup
        let task = NiTask::new()?;
        self.create_task_channels(&task)?;
        task.cfg_output_buffer(buf_size)?;
        task.disallow_regen()?;
        self.cfg_clk_sync(&task, seq_len)?;

        // Bundle NiTask, StreamCounter, buf_write_timeout, and task_type together for convenience:
        let mut stream_bundle = StreamBundle {
            task_type: self.task_type(),
            ni_task: task,
            counter,
            buf_write_timeout,
        };

        // Calc and write the initial sample chunk into the buffer
        let (start_pos, end_pos) = stream_bundle.counter.tick_next().unwrap();
        let samp_arr = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
        stream_bundle.write_buf(samp_arr)?;

        // FixMe [after Device move to streamer crate]:
        //  store NiTask+StreamCounter in internal fields instead of returning and passing to stream_/close_run_()
        Ok(stream_bundle)
    }
    fn stream_run_(&self, stream_bundle: &mut StreamBundle, start_sync: &StartSync, calc_next: bool) -> Result<(), WorkerError> {
        // Synchronise task start with other threads
        match start_sync {
            StartSync::Primary(recvr_vec) => {
                for recvr in recvr_vec {
                    recvr.recv()?
                };
                stream_bundle.ni_task.start()?;
            },
            StartSync::Secondary(sender) => {
                stream_bundle.ni_task.start()?;
                sender.send(())?;
            },
            StartSync::None => stream_bundle.ni_task.start()?
        };

        // Main streaming loop
        while let Some((start_pos, end_pos)) = stream_bundle.counter.tick_next() {
            let samp_arr = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
            stream_bundle.write_buf(samp_arr)?;
        }

        // Now need to wait for the final sample chunk to be generated out by the card before stopping the task.
        // In the mean time, we can calculate the initial chunk for the next repetition in the case we are on repeat.
        if !calc_next {
            stream_bundle.ni_task.wait_until_done(stream_bundle.buf_write_timeout.clone())?;
            stream_bundle.ni_task.stop()?;
        } else {
            stream_bundle.counter.reset();
            let (start_pos, end_pos) = stream_bundle.counter.tick_next().unwrap();
            let samp_arr = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);

            stream_bundle.ni_task.wait_until_done(stream_bundle.buf_write_timeout.clone())?;
            stream_bundle.ni_task.stop()?;

            stream_bundle.write_buf(samp_arr)?;
        }
        Ok(())
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
    fn create_task_channels(&self, task: &NiTask) -> Result<(), DAQmxError> {
        match self.task_type() {
            TaskType::AO => {
                // Require compiled, streamable channels
                for chan in self.compiled_channels(true, false).iter() {
                    task.create_ao_chan(&format!("/{}/{}", self.name(), chan.name()))?;
                };
            }
            TaskType::DO => {
                for chan in self.compiled_channels(true, false).iter() {
                    task.create_do_chan(&format!("/{}/{}", self.name(), chan.name()))?;
                };
            }
        };
        Ok(())
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
    fn cfg_clk_sync(&self, task: &NiTask, seq_len: usize) -> Result<(), DAQmxError> {
        // (1) Sample clock timing mode (includes sample clock source). Additionally, config samp_clk_out
        let samp_clk_src = self.get_samp_clk_in().unwrap_or("".to_string());
        task.cfg_samp_clk_timing(
            &samp_clk_src,
            self.samp_rate(),
            seq_len as u64
        )?;
        if let Some(term) = self.get_samp_clk_out() {
            task.export_signal(
                DAQMX_VAL_SAMPLECLOCK,
                &format!("/{}/{}", self.name(), term)
            )?
        };
        // (2) Start trigger:
        if let Some(term) = self.get_start_trig_in() {
            task.cfg_dig_edge_start_trigger(&format!("/{}/{}", self.name(), term))?
        };
        if let Some(term) = self.get_start_trig_out() {
            task.export_signal(
                DAQMX_VAL_STARTTRIGGER,
                &format!("/{}/{}", self.name(), term)
            )?
        };
        // (3) Reference clock
        /*  Only handling ref_clk import here.

        The "easily accessible" static ref_clk export from a single card should have already been done
        by the Streamer if user specified `ref_clk_provider`.
        Not providing the "easy access" to exporting ref_clk from more than one card on purpose.

        (Reminder: we are using static ref_clk export (as opposed to task-based export) to be able to always use
        the same card as the clock reference source even if this card does not run this time)

        NIDAQmx allows exporting 10MHz ref_clk from more than one card. And this even has a realistic use case
        of chained clock locking when a given card both locks to external ref_clk and exports its own
        reference for use by another card.

        The risk is that the user may do ref_clk export and forget to add pulses to this card. In such case
        the reference signal will show up but it will not be locked to the input reference
        since locking is only done on the per-task basis. This may lead to very hard-to-find footguns
        because it is hard to distinguish between locked and free-running 10MHz signals.

        For that reason, we still leave room for arbitrary (static) export from any number of cards,
        but only expose it through the "advanced" function `nidaqmx::connect_terms()`.
        */
        if let Some(term) = self.get_ref_clk_in() {
            task.set_ref_clk_src(&format!("/{}/{}", self.name(), term))?;
            task.set_ref_clk_rate(10.0e6)?;
        };

        Ok(())
    }
}

impl StreamableDevice for Device {}
