mod experiment;
mod device; 
mod utils;
mod nidaqmx;

use crate::experiment::Experiment;
use crate::device::StreamableDevice;
use aacompiler_backend::experiment::BaseExperiment;
use crate::utils::Semaphore;

fn main() {
    let mut exp = Experiment::new();
    let samp_rate = 1e6;
    exp.add_ao_device("PXI1Slot3", "PXI_Trig0", true, samp_rate);
    exp.add_ao_device("PXI1Slot4", "PXI_Trig0", false, samp_rate);
    exp.add_do_device("PXI1Slot6", "PXI_Trig0", false, 1e7);
    exp.reset_devices();
    
    exp.add_ao_channel("PXI1Slot3", 0);
    exp.sine("PXI1Slot3", "ao0", 0.0, 1., true, 2.0, Some(5.), None, None);
    exp.constant("PXI1Slot3", "ao0", 10., 1.-1e-6, 5., false);
    
    exp.add_ao_channel("PXI1Slot4", 0);
    exp.sine("PXI1Slot4", "ao0", 0.0, 1., true, 2.0, Some(5.), None, None);
    exp.constant("PXI1Slot4", "ao0", 10., 1.-1e-6, 5., false);
    
    exp.add_do_channel("PXI1Slot6", 0, 1);
    exp.add_do_channel("PXI1Slot6", 0, 0);
    
    exp.high("PXI1Slot6", "port0/line1", 0., 1.);
    exp.high("PXI1Slot6", "port0/line0", 0., 1.);
    
    exp.high("PXI1Slot6", "port0/line1", 10., 1.-2e-6);
    exp.high("PXI1Slot6", "port0/line0", 10., 1.-2e-6);
    
    exp.compile_with_stoptime(11.);
    exp.stream_exp(100.);
}

// mod device;
// mod utils;
// mod experiment;
// mod nidaqmx;
// use crate::utils::StreamCounter;

// fn main() {
//     let mut counter = StreamCounter::new(100, 10);
    
//     for _ in 0..12 {
//         let (prev, next) = counter.tick_next();
//         println!("Previous: {}, Next: {}", prev, next);
//     }
// }