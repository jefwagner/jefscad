#[cfg(feature = "extension-module")]
use pyo3::prelude::*;

mod brep_kernel;
pub(crate) mod csg_lang;
mod geom;
mod mesher;
mod py_bindings;

// ---------------------------------------------------------------------------
// Module entry point
// ---------------------------------------------------------------------------

/// The jefscad Rust extension module (jefscad._jefscad).
/// python/jefscad/__init__.py re-exports the public API from here.
#[cfg(feature = "extension-module")]
#[pymodule]
fn _jefscad(m: &Bound<'_, PyModule>) -> PyResult<()> {
    py_bindings::register(m)
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
