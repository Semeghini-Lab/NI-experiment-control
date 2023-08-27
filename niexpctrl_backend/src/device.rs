use crate::nidaqmx::*;
use crate::utils::{Semaphore, StreamCounter};

use std::sync::Arc;

use nicompiler_backend::*;

pub trait StreamableDevice: BaseDevice + Sync + Send {
    fn stream_task(&self, sem: &Arc<Semaphore>, num_devices: usize, stream_buftime: f64, nreps: usize) {
        let mut timer = TickTimer::new();
        let mut timer_ = TickTimer::new();

        assert!(
            self.is_compiled(),
            "Compile device {} before streaming",
            self.physical_name()
        );

        // Trick: in principle, calculation of the first signal can be done independently of daqmx setup
        // Still have to figure out how to do in rust.
        let seq_len = (self.compiled_stop_time() * self.samp_rate()) as usize;
        let buffer_size = std::cmp::min(seq_len, (stream_buftime * self.samp_rate() / 1000.) as usize);
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
        timer.tick_print(&format!("{} cfg (task channels, buffers, clk & sync)", self.physical_name()));

        // Obtain the first signal (optional: from parallel thread), and do first bufwrite
        let signal = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
        timer.tick_print(&format!("{} wait to receive signal", self.physical_name()));
        bufwrite(signal);
        timer.tick_print(&format!("{} bufwrite", self.physical_name()));

        for _rep in 0..nreps {
            // For every repetition, make sure the primary device starts last
            if self.is_primary() {
                (0..num_devices).for_each(|_| sem.acquire());
                sem.release(); // Release the semaphore to restore count to 1, in preparation for the next run.
            }
            task.start();
            timer_.tick_print(&format!("{} start (restart) overhead", self.physical_name()));
            if !self.is_primary() {
                sem.release();
            }
            // Main chunk for streaming
            while end_pos != seq_len {
                (start_pos, end_pos) = counter.tick_next();
                let signal_stream = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
                bufwrite(signal_stream);
            }
            if nreps > 1 {
                // If we're on repeat: don't wait for the task to finish, calculate and write the next chunk
                (start_pos, end_pos) = counter.tick_next();
                let signal_next_start = self.calc_signal_nsamps(start_pos, end_pos, end_pos - start_pos, true, false);
                task.wait_until_done(stream_buftime * 2. / 1000.);
                timer_.tick_print(&format!("{} end", self.physical_name()));
                task.stop();
                bufwrite(signal_next_start);
            }
            else {
                task.wait_until_done(stream_buftime * 2. / 1000.);
                timer_.tick_print(&format!("{} end", self.physical_name()));
                task.stop();
            }
        }
    }

    fn cfg_task_channels(&self, task: &NiTask) {
        match self.task_type() {
            TaskType::AO => {
                // Require compiled, streamable channels
                self.compiled_channels(true, false).iter().for_each(|chan| {
                    task.create_ao_chan(&format!(
                        "/{}/{}",
                        &self.physical_name(),
                        chan.physical_name()
                    ));
                });
            }
            TaskType::DO => {
                self.compiled_channels(true, false).iter().for_each(|chan| {
                    task.create_do_chan(&format!(
                        "/{}/{}",
                        &self.physical_name(),
                        chan.physical_name()
                    ));
                });
            }
        }
    }

    fn cfg_clk_sync(&self, task: &NiTask, seq_len: &usize) {
        let seq_len = *seq_len as u64;
        match (self.is_primary(), self.task_type()) {
            (true, TaskType::AO) => {
                // Primary device: must be AO, routes start trigger and 10MHz ref clock
                assert!(self.task_type() == TaskType::AO,
                    "Primary device {} should be AO",
                    self.physical_name()
                );
                let ref_clk_out= format!("/{}/PXI_Trig7", &self.physical_name());
                task.export_signal(
                    DAQMX_VAL_10MHZREFCLOCK,
                    &ref_clk_out,
                );
                task.export_signal(
                    DAQMX_VAL_STARTTRIGGER,
                    &format!("/{}/{}", &self.physical_name(), &self.trig_line()),
                );
                task.cfg_sample_clk("", self.samp_rate(), seq_len);
            }
            (true, TaskType::DO) => panic!("Primary device {} should be AO", self.physical_name()),
            (false, TaskType::AO) => {
                task.cfg_dig_edge_start_trigger(&format!(
                    "/{}/{}",
                    &self.physical_name(),
                    &self.trig_line()
                ));
                let ref_clk_src = format!("/{}/PXI_Trig7", &self.physical_name());
                task.cfg_ref_clk(&ref_clk_src, 1e7);
                task.cfg_sample_clk("", self.samp_rate(), seq_len);
            }
            (false, TaskType::DO) => {
                assert!(
                    self.samp_rate() == 1e7,
                    "Current synchronization scheme only supports DO at 10Mhz"
                );
                task.cfg_dig_edge_start_trigger(&format!(
                    "/{}/{}",
                    &self.physical_name(),
                    &self.trig_line()
                ));
                let samp_clk_src = format!("/{}/PXI_Trig7", &self.physical_name());
                task.cfg_sample_clk(
                    &samp_clk_src,
                    1e7,
                    seq_len,
                );
            }
        };
    }
}

impl StreamableDevice for Device {}
