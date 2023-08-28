pub mod channel;
pub mod device;
pub mod experiment;
pub mod instruction;
pub mod utils;

use nicompiler_backend::*;

// fn main() {
//     let mut exp = Experiment::new();
//     exp.add_do_device("PXI1Slot6", 1e6, None, None, None, None, None, None);
//     exp.add_do_channel("PXI1Slot6", 0, 0);
//     exp.add_do_channel("PXI1Slot6", 0, 4);
//     exp.high("PXI1Slot6", "port0/line0", 1., 4.);
//     exp.high("PXI1Slot6", "port0/line4", 2., 5.);
//     exp.compile_with_stoptime(10.);
//     println!(
//         "{:?}",
//         exp.devices()
//             .get("PXI1Slot6")
//             .unwrap()
//             .calc_signal_nsamps(0, 10, 10, true, false)
//     );
// }

fn main() {
    let mut exp = Experiment::new();
    exp.add_do_device("PXI1Slot6", 1e6, None, None, None, None, None, None);
    exp.add_do_channel("PXI1Slot6", 0, 0);
    exp.high("PXI1Slot6", "port0/line0", 1., 4.);
    exp.compile();
    assert_eq!(exp.compiled_stop_time(), 4.);
    exp.compile_with_stoptime(5.); // Experiment signal will stop at t=5 now
    println!("{}", exp.compiled_stop_time());
    // assert_eq!(exp.compiled_stop_time(), 5.);
}
