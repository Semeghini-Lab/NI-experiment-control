use numpy;
use pyo3::prelude::*;
use std::sync::Arc;
use crossbeam::thread;
use std::collections::HashMap;

use aacompiler_backend::*;

use crate::nidaqmx::*;
use crate::device::*;
use crate::utils::Semaphore;

#[pyclass]
pub struct Experiment {
    devices: HashMap<String, Device>,
}

impl_exp_boilerplate!(Experiment);

#[pymethods]
impl Experiment {
    pub fn stream_exp(&self, stream_buftime: f64) {
        match thread::scope(|s| {
            let sem_shared = Arc::new(Semaphore::new(1));
            for dev in self.compiled_devices() {
                let sem_clone = sem_shared.clone(); // Clone outside of the closure
                s.spawn(move |_| {
                    dev.stream_task(&sem_clone, self.compiled_devices().len(), stream_buftime);
                });
            }
        }) {
            Ok(_) => {}
            Err(_) => {
                panic!("stream_failed, in case of ni-error, error-message is written to nidaqmx_error.logs");
            }
        }    
    }

    pub fn reset_device(&mut self, dev_name: &str){
        self.device_op(dev_name, |_dev| {reset_ni_device(dev_name)});
    }

    pub fn reset_devices(&self){
        self.devices().values().for_each(|dev| {reset_ni_device(dev.physical_name())});
    }
}

// if self.is_primary:
// self.task.export_signals.export_signal(
//     signal_id=nidaqmx.constants.Signal.TEN_MHZ_REF_CLOCK,
//     output_terminal=f'/{self.device_name}/PXI_Trig7'
// )
// # Primary cards must export their start-trigger to h5-indicated export port
// self.task.export_signals.export_signal(nidaqmx.constants.Signal.START_TRIGGER, self.trig_line)
// self.task.timing.cfg_samp_clk_timing(rate=self.samp_rate, source="", samps_per_chan=self.samp_num)
// self.proc_status['sync_status'] = f'Primary device {self.device_name} routed triggers to {self.trig_line} ' \
//                                 f"and 10Mhz ref-clock to {f'/{self.device_name}/PXI_Trig7'}"
// elif self.device_group.attrs['dev_type'] == 'AODevice':

// # Secondary cards listen for start trigger along h5-indicated input port
// self.task.triggers.start_trigger.cfg_dig_edge_start_trig(self.trig_line)
// self.task.timing.ref_clk_src = f'/{self.device_name}/PXI_Trig7'
// self.task.timing.ref_clk_rate = 1e7
// self.task.timing.cfg_samp_clk_timing(rate=self.samp_rate, source="", samps_per_chan=self.samp_num)
// self.proc_status['sync_status'] = f'Secondary AODevice {self.device_name} listening to triggers {self.trig_line} ' \
//                                 f"and reference phase-locked to /{self.device_name}/PXI_Trig7"
// elif self.device_group.attrs['dev_type'] == 'DODevice':
// self.task.triggers.start_trigger.cfg_dig_edge_start_trig(self.trig_line)
// assert self.samp_rate == 1e7
// self.task.timing.cfg_samp_clk_timing(rate=self.samp_rate, source=f"/{self.device_name}/PXI_Trig7", samps_per_chan=self.samp_num)
// self.proc_status['sync_status'] = f'Secondary DODevice {self.device_name} listening to triggers {self.trig_line} ' \
//                                 f"and imports clock from /{self.device_name}/PXI_Trig7"
// else:
// raise RuntimeError