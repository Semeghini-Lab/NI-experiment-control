pub mod channel;
pub mod device;
pub mod experiment;
pub mod instruction;
pub mod utils;

use crate::channel::*;
use crate::device::*;
use crate::experiment::*;
use crate::instruction::*;
use crate::utils::*;

fn main() {
    // let mut dev = Device::new("PXI1Slot6", "PXI_Trig7", DeviceType::DODevice, true, 1e7);
    // dev.add_channel("port0/line0");
    // dev.add_channel("port0/line4");

    let mut exp = Experiment::new();
    exp.add_do_device("PXI1Slot6", "PXI_Trig7", true, 1.);
    exp.add_do_channel("PXI1Slot6", 0, 0);
    exp.add_do_channel("PXI1Slot6", 0, 4);
    exp.high("PXI1Slot6", "port0/line0", 1., 4.);
    exp.high("PXI1Slot6", "port0/line4", 2., 5.);
    exp.compile_with_stoptime(10.);
    println!(
        "{:?}",
        exp.devices()
            .get("PXI1Slot6")
            .unwrap()
            .calc_stream_signal(0, 10)
    )
}
