#[cfg(feature = "extension-module")]
use pyo3::prelude::*;

mod brep_kernel;
mod csg_lang;
mod geom;
mod mesher;
mod py_bindings;

// ---------------------------------------------------------------------------
// HelloWorld — demo struct showing a pyo3 class with a constructor and method
// ---------------------------------------------------------------------------

/// A simple greeting object.
///
/// Parameters
/// ----------
/// name : str
///     The name to include in the greeting.
///
/// Examples
/// --------
/// >>> hw = HelloWorld("Alice")
/// >>> hw.greet()
/// 'Hello, Alice!'
#[cfg(feature = "extension-module")]
#[pyclass]
pub struct HelloWorld {
    name: String,
}

#[cfg(feature = "extension-module")]
#[pymethods]
impl HelloWorld {
    #[new]
    pub fn new(name: String) -> Self {
        HelloWorld { name }
    }

    /// Return a greeting string for this object's name.
    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Add two integers (demo of a plain Python-callable function).
#[cfg(feature = "extension-module")]
#[pyfunction]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

// ---------------------------------------------------------------------------
// Module entry point
// ---------------------------------------------------------------------------

/// The jefscad Rust extension module (jefscad._jefscad).
/// python/jefscad/__init__.py re-exports the public API from here.
#[cfg(feature = "extension-module")]
#[pymodule]
fn _jefscad(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HelloWorld>()?;
    m.add_function(wrap_pyfunction!(add, m)?)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Pure-Rust tests (run with `cargo +nightly test`, no Python needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
