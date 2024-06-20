mod device;
mod experiment;
mod nidaqmx;
mod utils;
mod worker_cmd_chan;

use niexpctrl_backend::*;

/* fn main() {
    let mut exp = Experiment::new();
    // Define devices and associated channels
    exp.add_ao_device("PXI1Slot3", 1e6);
    exp.add_ao_channel("PXI1Slot3", 0, 0.);

    exp.add_ao_device("PXI1Slot4", 1e6);
    exp.add_ao_channel("PXI1Slot4", 0, 0.);

    exp.add_do_device("PXI1Slot6", 1e7);
    exp.add_do_channel("PXI1Slot6", 0, 0, 0.);
    exp.add_do_channel("PXI1Slot6", 0, 4, 0.);

    // Define synchronization behavior:
    exp.device_cfg_trig("PXI1Slot3", "PXI_Trig0", true);
    exp.device_cfg_ref_clk("PXI1Slot3", "PXI_Trig7", 1e7, true);

    exp.device_cfg_trig("PXI1Slot4", "PXI_Trig0", false);
    exp.device_cfg_ref_clk("PXI1Slot4", "PXI_Trig7", 1e7, false);

    exp.device_cfg_samp_clk_src("PXI1Slot6", "PXI_Trig7");
    exp.device_cfg_trig("PXI1Slot6", "PXI_Trig0", false);

    // PXI1Slot3/ao0 starts with a 1s-long 7Hz sine wave with offset 1
    // and unit amplitude, zero phase. Does not keep its value.
    exp.sine("PXI1Slot3", "ao0", 0., 1., false, 7., None, None, Some(1.));
    // Ends with a half-second long 1V constant signal which returns to zero
    exp.constant("PXI1Slot3", "ao0", 9., 0.5, 1., false);

    // We can also leave a defined channel empty: the device / channel will simply not be compiled

    // Both lines of PXI1Slot6 start with a one-second "high" at t=0 and a half-second high at t=9
    exp.high("PXI1Slot6", "port0/line0", 0., 1.);
    exp.high("PXI1Slot6", "port0/line0", 9., 0.5);
    // Alternatively, we can also define the same behavior via go_high/go_low
    exp.go_high("PXI1Slot6", "port0/line4", 0.);
    exp.go_low("PXI1Slot6", "port0/line4", 1.);

    exp.go_high("PXI1Slot6", "port0/line4", 9.);
    exp.go_low("PXI1Slot6", "port0/line4", 9.5);

    exp.compile_with_stoptime(10.); // Experiment signal will stop at t=10 now
    assert_eq!(exp.compiled_stop_time(), 10.);

    exp.stream_exp(50., 2);
} */


/* fn main() {
    let mut exp = Experiment::new();
    // Define devices and associated channels
    exp.add_ao_device("PXI1Slot3", 1e6);
    exp.add_ao_channel("PXI1Slot3", 10, 0.);

    exp.linramp("PXI1Slot3", "ao10", 0., 1., 0., 1., true);

    exp.compile(None);
    // exp.stream_exp(50., 1);
} */

fn main() {
    let mut streamer = crate::Experiment::new();

    streamer.add_ao_device("Dev2", 400e3);
    streamer.add_ao_device("Dev3", 1e6);
    streamer.add_ao_channel("Dev2", 0, 0.0);
    streamer.add_ao_channel("Dev3", 0, 0.0);
    let _ = streamer.reset_all();

    // Hardware sync settings
    let TRIG_LINE: String = "RTSI0".to_string();
    let REF_CLK_LINE: String = "RTSI1".to_string();
    // - streamer-wide
    streamer.set_starts_last(Some("Dev2".to_string()));
    streamer.set_ref_clk_provider(Some(("Dev2".to_string(), REF_CLK_LINE.clone())));
    // - Dev2
    streamer.devices_().get_mut("Dev2").unwrap().set_start_trig_out(Some(TRIG_LINE.clone()));
    // - Dev3
    streamer.devices_().get_mut("Dev3").unwrap().set_start_trig_in(Some(TRIG_LINE.clone()));
    streamer.devices_().get_mut("Dev3").unwrap().set_ref_clk_in(Some(REF_CLK_LINE.clone()));
    //  export start_trig for convenience (to trigger oscilloscope)
    streamer.devices_().get_mut("Dev3").unwrap().set_start_trig_out(Some("PFI0".to_string()));

    // Add instructions
    // - Dev2
    streamer.go_constant("Dev2", "ao0", 0.0, 1.0);
    streamer.sine("Dev2", "ao0", 1.0, 1.0, false, 10.0, Some(1.5), None, None);
    streamer.constant("Dev2", "ao0", 3.0, 1.0, -1.0);
    // - Dev3
    streamer.constant("Dev3", "ao0", 0.5, 2.0, 1.0);

    streamer.compile(None);

    for i in 0..3 {
        println!("\n\n ============= launching run {i} =============== \n\n");
        let run_res = streamer.run(5, 150.0);
        println!("============= result of run {i}: =============== ");
        match run_res {
            Ok(()) => println!("Run was successfully completed"),
            Err(msg) => {
                println!("Run failed. Error message:\n{msg}");
                break
            },
        };

    };
}
