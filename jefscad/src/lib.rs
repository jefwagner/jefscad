#[cfg(feature = "extension-module")]
use pyo3::prelude::*;

#[cfg(feature = "extension-module")]
use pyo3_stub_gen;

mod brep_kernel;
pub(crate) mod brep_compiler;
pub mod csg_lang;
mod geom;
pub mod mesher;
mod py_bindings;

pub use csg_lang::{CsgNode, NodeRef, SelectPolicy};

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

// Exposes stub metadata collected by gen_stub_* annotations.
// The call to StubInfo::from_pyproject_toml must live in *this* crate (not the
// binary) so that the linker includes all inventory-registered items.
// pyproject.toml is in the workspace root, one level above CARGO_MANIFEST_DIR.
#[cfg(feature = "extension-module")]
pub fn stub_info() -> pyo3_stub_gen::Result<pyo3_stub_gen::StubInfo> {
    let pyproject = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("pyproject.toml");
    pyo3_stub_gen::StubInfo::from_pyproject_toml(pyproject)
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
