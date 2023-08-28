use pyo3::prelude::*;

pub mod device;
pub mod experiment;
pub mod nidaqmx;
pub mod utils;

use crate::device::*;
pub use crate::experiment::Experiment;
use crate::experiment::*;
use crate::nidaqmx::*;
use crate::utils::*;
use nicompiler_backend::*;

#[pymodule]
fn niexpctrl_backend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Experiment>()?;
    Ok(())
}
