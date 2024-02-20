mod nidaqmx;
use nidaqmx::*;

fn main() {
    connect_terms("/Dev1/10MHzRefClock", "/Dev1/RTSI7");
    println!("Connected terminals");
}