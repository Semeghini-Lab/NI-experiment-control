use pyo3::prelude::*;

mod device;
mod experiment;
mod nidaqmx;
mod utils;

use crate::experiment::Experiment;

#[pymodule]
fn niexpctrl_backend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Experiment>()?;
    Ok(())
}
