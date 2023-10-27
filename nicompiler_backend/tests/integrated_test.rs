use nicompiler_backend::Experiment;
use nicompiler_backend::BaseExperiment;

// fn main() {
//     let mut exp = Experiment::new();
//     // Define devices and associated channels
//     exp.add_ao_device("PXI1Slot3", 1e6);
//     exp.add_ao_channel("PXI1Slot3", 0);

//     exp.add_ao_device("PXI1Slot4", 1e6);
//     exp.add_ao_channel("PXI1Slot4", 0);

//     exp.add_do_device("PXI1Slot6", 1e7);
//     exp.add_do_channel("PXI1Slot6", 0, 0);
//     exp.add_do_channel("PXI1Slot6", 0, 4);

//     // Define synchronization behavior (refer to the "Device" struct for more information)
//     exp.device_cfg_trig("PXI1Slot3", "PXI1_Trig0", true);
//     exp.device_cfg_ref_clk("PXI1Slot3", "PXI1_Trig7", 1e7, true);

//     exp.device_cfg_trig("PXI1Slot4", "PXI1_Trig0", false);
//     exp.device_cfg_ref_clk("PXI1Slot4", "PXI1_Trig7", 1e7, false);

//     exp.device_cfg_samp_clk_src("PXI1Slot6", "PXI1_Trig7");
//     exp.device_cfg_trig("PXI1Slot6", "PXI1_Trig0", false);

//     // PXI1Slot3/ao0 starts with a 1s-long 7Hz sine wave with offset 1
//     // and unit amplitude, zero phase. Does not keep its value.
//     exp.sine("PXI1Slot3", "ao0", 0., 1., false, 7., None, None, Some(1.));
//     // Ends with a half-second long 1V constant signal which returns to zero
//     exp.constant("PXI1Slot3", "ao0", 9., 0.5, 1., false);

//     // We can also leave a defined channel empty: the device / channel will simply not be compiled

//     // Both lines of PXI1Slot6 start with a one-second "high" at t=0 and a half-second high at t=9
//     exp.high("PXI1Slot6", "port0/line0", 0., 1.);
//     exp.high("PXI1Slot6", "port0/line0", 9., 0.5);
//     // Alternatively, we can also define the same behavior via go_high/go_low
//     exp.go_high("PXI1Slot6", "port0/line4", 0.);
//     exp.go_low("PXI1Slot6", "port0/line4", 1.);

//     exp.go_high("PXI1Slot6", "port0/line4", 9.);
//     exp.go_low("PXI1Slot6", "port0/line4", 9.5);

//     // Compile the experiment: this will stop the experiment at the last edit-time plus one tick
//     exp.compile();

//     // We can compile again with a specific stop_time (and add instructions in between)
//     exp.compile_with_stoptime(10.); // Experiment signal will stop at t=10 now
//     assert_eq!(exp.compiled_stop_time(), 10.);
// }

#[test]
fn empty_compile() {
    let mut exp = Experiment::new();
    // Define devices and associated channels
    exp.add_do_device("PXI1Slot6", 1e7);
    exp.add_do_channel("PXI1Slot6", 0, 0);

    exp.compile();
    println!("Compiled!");
}

#[test]
#[should_panic(expected="There is no channel with streamable=false, editable=false")]
fn empty_calc_signal() {
    let mut exp = Experiment::new();
    // Define devices and associated channels
    exp.add_do_device("PXI1Slot6", 1e7);
    exp.add_do_channel("PXI1Slot6", 0, 0);

    exp.compile();
    exp.device_calc_signal_nsamps("PXI1Slot6", 0, 10, 100, false, false);
}

#[test]
fn test_reset_tick() {
    let mut exp = Experiment::new();
    // Define devices and associated channels
    exp.add_do_device("PXI1Slot6", 10.);
    exp.add_do_channel("PXI1Slot6", 0, 0);
    exp.add_do_channel("PXI1Slot6", 0, 1);

    exp.high("PXI1Slot6", "port0/line0", 0., 1.);
    exp.go_high("PXI1Slot6", "port0/line1", 0.);
    exp.compile_with_stoptime(5.);

    // Calculate from t=0 ~ 5
    let sig = exp.device_calc_signal_nsamps("PXI1Slot6", 0, 50, 50, false, true);
    assert!(sig[[0, 9]] == 1. && sig[[0, 10]] == 0.); // go_high takes effect on the tick corresponding to specified time. 
    assert!(sig[[1, 9]] == 1. && sig[[1, 10]] == 1.); 
    
    let reset_tick_time = exp.add_reset_tick();
    // Reset tick happens at the earliest unspecified interval across all channels
    assert!(reset_tick_time == 1.0); 
    exp.compile_with_stoptime(5.);
    let sig = exp.device_calc_signal_nsamps("PXI1Slot6", 0, 50, 50, false, true);
    assert!(sig[[0, 9]] == 1. && sig[[0, 10]] == 0.); 
    assert!(sig[[1, 9]] == 1. && sig[[1, 10]] == 0.); // Also zeros channel 1 at t=1
    // println!("{:?}, reset_tick_time={}", sig, reset_tick_time);
}