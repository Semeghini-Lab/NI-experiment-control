use pyo3::prelude::*;

mod device;
mod utils;
mod experiment;
mod nidaqmx;

use crate::experiment::Experiment;

#[pymodule]
fn aaexpctrl_backend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Experiment>()?;
    Ok(())
}