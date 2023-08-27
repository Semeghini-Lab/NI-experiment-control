use crate::nidaqmx::*;
use crate::utils::{Semaphore, StreamCounter};

use std::sync::Arc;

use aacompiler_backend::device::{BaseDevice, Device, DeviceType};
use aacompiler_backend::channel::BaseChannel;

pub trait StreamableDevice: BaseDevice {
    fn stream_task(&self, sem: &Arc<Semaphore>, num_devices: usize, stream_buftime: f64) {
        assert!(self.is_compiled(), "Compile device {} before streaming", self.physical_name());

        // Create task and add channels
        let task = NiTask::new();
        match self.device_type() {
            DeviceType::AODevice => {
                self.compiled_channels().iter().for_each(|chan| {
                    task.create_ao_chan(&format!("/{}/{}", &self.physical_name(), chan.physical_name()));
                });
            },
            DeviceType::DODevice => {
                self.compiled_channels().iter().for_each(|chan| {
                    task.create_do_chan(&format!("/{}/{}", &self.physical_name(), chan.physical_name()));
                });
            },
            _ => panic!("NiDAQ add-channel method of device {} type not implemented", self.physical_name()),
        }

        // Configure buffer and writing methods
        let seq_len = (self.compiled_stop_time() * self.samp_rate()) as usize;
        let buffer_size = (stream_buftime * self.samp_rate() / 1000.) as usize;
        assert!(buffer_size <= seq_len, 
                "Streaming window ({} ticks) should be at most sequence length ({} ticks)", 
                buffer_size, seq_len);
        task.cfg_output_buffer(buffer_size);
        task.disallow_regen();
        let bufwrite = |signal| {
            match self.device_type() {
                DeviceType::AODevice => task.write_analog(&signal),
                DeviceType::DODevice => task.write_digital(&signal.map(|&x| x as u8)),
                _ => panic!("Buf-write method of device {} type not implemented", self.physical_name()), 
            };
        };

        // Synchronization
        task.cfg_sample_clk("", self.samp_rate(), seq_len as u64);
        match (self.is_primary(), self.device_type()) {
            (true, DeviceType::AODevice) => {
                // Primary device: must be AODevice, routs start trigger and 10MHz ref clock
                assert!(self.device_type() == DeviceType::AODevice, 
                        "Primary device {} should be AODevice", self.physical_name());
                task.export_signal(DAQMX_VAL_10MHZREFCLOCK, &format!("/{}/PXI_Trig7", &self.physical_name()));
                task.export_signal(DAQMX_VAL_STARTTRIGGER, 
                                &format!("/{}/{}", &self.physical_name(), &self.trig_line()));
                task.cfg_sample_clk("", self.samp_rate(), seq_len as u64);
            },
            (true, DeviceType::DODevice) => panic!("Primary device {} should be AODevice", self.physical_name()),
            (false, DeviceType::AODevice) => {
                // Secondary AO: lock reference to 10Mhz of primary
                task.cfg_dig_edge_start_trigger(&format!("/{}/{}", &self.physical_name(), &self.trig_line()));
                task.cfg_ref_clk(&format!("/{}/PXI_Trig7", &self.physical_name()), 1e7);
                task.cfg_sample_clk("", self.samp_rate(), seq_len as u64);
            },
            (false, DeviceType::DODevice) => {
                task.cfg_sample_clk("", self.samp_rate(), seq_len as u64);
                assert!(self.samp_rate() == 1e7, "Current synchronization scheme only supports DODevice at 10Mhz");
                task.cfg_dig_edge_start_trigger(&format!("/{}/{}", &self.physical_name(), &self.trig_line()));
                task.cfg_sample_clk(&format!("/{}/PXI_Trig7", &self.physical_name()), 1e7, seq_len as u64);
            },
            _ => {panic!("Synchronization undefined for given device {} primality and type", self.physical_name());}
        };

        let mut counter = StreamCounter::new(seq_len, buffer_size);
        let (mut start_pos, mut end_pos) = counter.tick_next();
        let signal = self.calc_stream_signal(start_pos, end_pos);
        bufwrite(signal);

        if self.is_primary() {
            // println!("Primary is acquiring semaphore {} times", num_devices);
            for i in 0..num_devices {
                sem.acquire();
                // println!("Primary acquired semaphore")
            }
            sem.release(); // Release the semaphore to restore count to 1, in preparation for next run.
        }
        task.start();
        if !self.is_primary() {
            sem.release();
            println!("Device {} started and released semaphore", self.physical_name());
        }
        else{
            println!("Primary device {} started", self.physical_name());
        }

        while end_pos != seq_len {
            (start_pos, end_pos) = counter.tick_next();
            let signal = self.calc_signal_nsamps(start_pos, end_pos, end_pos-start_pos);
            // let shape = signal.dim();
            // let first_element = signal[[0, 0]];
            // println!("{:?}, {}, {}", shape, end_pos, first_element);
            bufwrite(signal);
        }
        task.wait_until_done(stream_buftime * 2. / 1000.);
        task.stop();
    }
} 

impl StreamableDevice for Device {}