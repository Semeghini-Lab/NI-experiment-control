mod device;
mod experiment;
mod nidaqmx;
mod utils;

use crate::experiment::Experiment;

use nicompiler_backend::*;

fn main() {
    let mut exp = Experiment::new();
    let samp_rate = 1e6;
    exp.add_ao_device("PXI1Slot3", 1e6, None, Some("PXI_Trig0"), 
    Some(true), Some("PXI_Trig7"), Some(false), Some(1e7));
    exp.add_ao_device("PXI1Slot4", 1e6, None, Some("PXI_Trig0"), 
    Some(false), Some("PXI_Trig7"), Some(true), Some(1e7));
    exp.add_do_device("PXI1Slot6", 1e7, Some("PXI_Trig7"), Some("PXI_Trig0"), 
    Some(false), None, None, None);
    exp.reset_devices();

    exp.add_ao_channel("PXI1Slot3", 0);
    exp.sine("PXI1Slot3", "ao0", 0.0, 1., true, 2.0, Some(5.), None, None);
    exp.constant("PXI1Slot3", "ao0", 10., 1. - 1e-6, 5., false);

    exp.add_ao_channel("PXI1Slot4", 0);
    exp.sine("PXI1Slot4", "ao0", 0.0, 1., true, 2.0, Some(5.), None, None);
    exp.constant("PXI1Slot4", "ao0", 10., 1. - 1e-6, 5., false);

    exp.add_do_channel("PXI1Slot6", 0, 1);
    exp.add_do_channel("PXI1Slot6", 0, 0);

    exp.high("PXI1Slot6", "port0/line1", 0., 1.);
    exp.high("PXI1Slot6", "port0/line0", 0., 1.);

    exp.high("PXI1Slot6", "port0/line1", 10., 1. - 2e-6);
    exp.high("PXI1Slot6", "port0/line0", 10., 1. - 2e-6);

    exp.compile_with_stoptime(11.);
    exp.stream_exp(50., 2);
}
