//! Generates `_jefscad.pyi` type stub file from the PyO3 annotations in the library.
//!
//! Run with:
//!     cargo +nightly run --bin stub_gen --features extension-module

fn main() -> pyo3_stub_gen::Result<()> {
    let stub = _jefscad::stub_info()?;
    stub.generate()?;
    Ok(())
}
