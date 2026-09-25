pub mod algorithms;
pub mod cfg;
pub mod error;
pub mod games;
pub mod logging;
pub mod memory;
pub mod observation;
pub mod param;
//pub mod train;
//pub mod cnn;
pub mod worker;

/*
    Rust-Python Bindings of rrr.
    Copyright (C) 2026  T. Liu (touhourl@proton.me) and contributors of thrl project

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use pyo3::prelude::*;

#[pyfunction]
fn init_logging() {
    crate::logging::init_tracing();
}

#[pyfunction]
fn runtime_config_json() -> PyResult<String> {
    serde_json::to_string(&crate::param::RuntimeConfig::global().raw)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn environment_spec_json() -> PyResult<String> {
    let spec = crate::games::spec(crate::param::RuntimeConfig::global())
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
    serde_json::to_string(&spec)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pymodule]
fn rrr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<crate::worker::Collector>()?;
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    m.add_function(wrap_pyfunction!(runtime_config_json, m)?)?;
    m.add_function(wrap_pyfunction!(environment_spec_json, m)?)?;
    Ok(())
}
