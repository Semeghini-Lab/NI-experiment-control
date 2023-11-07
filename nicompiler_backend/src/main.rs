pub mod channel;
pub mod device;
pub mod experiment;
pub mod instruction;
pub mod utils;

use nicompiler_backend::*;

// fn main() {
//     let mut exp = Experiment::new();
//     // Define devices and associated channels
//     exp.add_do_device("PXI1Slot6", 10.);
//     exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
//     exp.add_do_channel("PXI1Slot6", 0, 1, 0.);

//     exp.high("PXI1Slot6", "port0/line0", 0., 1.);
//     exp.go_high("PXI1Slot6", "port0/line1", 0.);
//     exp.compile_with_stoptime(5.);

//     // Calculate from t=0 ~ 5
//     let sig = exp.device_calc_signal_nsamps("PXI1Slot6", 0, 50, 50, false, true);
//     assert!(sig[[0, 9]] == 1. && sig[[0, 10]] == 0.); // go_high takes effect on the tick corresponding to specified time. 
//     assert!(sig[[1, 9]] == 1. && sig[[1, 10]] == 1.); 
    
//     let reset_tick_time = exp.add_reset_tick();
//     // Reset tick happens at the earliest unspecified interval across all channels
//     assert!(reset_tick_time == 1.0); 
//     exp.compile_with_stoptime(5.);
//     let sig = exp.device_calc_signal_nsamps("PXI1Slot6", 0, 50, 50, false, true);
//     assert!(sig[[0, 9]] == 1. && sig[[0, 10]] == 0.); 
//     assert!(sig[[1, 9]] == 1. && sig[[1, 10]] == 0.); // Also zeros channel 1 at t=1
//     // println!("{:?}, reset_tick_time={}", sig, reset_tick_time);
// }

fn main() {
    let mut exp = Experiment::new();
    exp.add_do_device("PXI1Slot6", 1e6);
    exp.add_do_channel("PXI1Slot6", 0, 4, 0.);
    exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", false);
    exp.go_high("PXI1Slot6", "port0/line4", 0.5);
    exp.compile_with_stoptime(1.); // Panics here
}

// fn main() {
//     let mut exp = Experiment::new();
//     exp.add_do_device("PXI1Slot6", 1e6);
//     exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
//     exp.add_do_channel("PXI1Slot6", 0, 4, 0.);
//     exp.high("PXI1Slot6", "port0/line0", 1., 4.); // stop time at 5
//     assert_eq!(exp.edit_stop_time(false), 5.);
//     exp.high("PXI1Slot6", "port0/line4", 0., 6.); // stop time at 6
//     assert_eq!(exp.edit_stop_time(false), 6.);
// }