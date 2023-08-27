use pyo3::prelude::*;
// use pyo3::wrap_pyfunction;

pub mod channel;
pub mod device;
pub mod experiment;
pub mod instruction;
pub mod utils;

pub use channel::*;
pub use device::*;
pub use experiment::*;
pub use instruction::*;
pub use utils::*;

#[pymodule]
fn nicompiler_backend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Experiment>()?;
    Ok(())
}
