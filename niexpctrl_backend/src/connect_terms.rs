mod nidaqmx;
use nidaqmx::*;

fn main() {
    connect_terms("/Dev2/10MHzRefClock", "/Dev2/RTSI7");
    println!("Connected terminals");
}