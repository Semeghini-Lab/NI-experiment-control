use pyo3::prelude::*;

pub mod device;
pub mod experiment;
pub mod nidaqmx;
pub mod utils;

pub use crate::nidaqmx::*;
pub use crate::experiment::*;
pub use crate::device::*;
pub use crate::utils::*;
pub use crate::experiment::Experiment;
use nicompiler_backend::*;

#[pymodule]
fn niexpctrl_backend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Experiment>()?;
    Ok(())
}
