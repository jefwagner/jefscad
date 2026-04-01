//! Python bindings for the jefscad CSG modeling API (via pyo3).

#[cfg(feature = "extension-module")]
use pyo3::prelude::*;

#[cfg(feature = "extension-module")]
use crate::csg_lang::CsgNode;

// ---------------------------------------------------------------------------
// Python-visible Node class
// ---------------------------------------------------------------------------

/// A CSG node: a primitive or operation with an associated transform stack.
///
/// Create nodes with the module-level constructors (`sphere`, `cuboid`, etc.)
/// and chain transforms with the methods below. All transform methods return
/// a **new** Node; the original is never mutated.
#[cfg(feature = "extension-module")]
#[pyclass(name = "Node")]
pub struct PyNode {
    pub(crate) inner: crate::csg_lang::NodeRef,
}

#[cfg(feature = "extension-module")]
#[pymethods]
impl PyNode {
    // --- introspection ------------------------------------------------------

    /// Unique provenance ID for this node (assigned at construction time).
    #[getter]
    fn prov_id(&self) -> u64 {
        self.inner.prov_id
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    // --- transform methods --------------------------------------------------

    /// Return a new Node translated by (dx, dy, dz).
    fn translate(&self, dx: f64, dy: f64, dz: f64) -> PyNode {
        PyNode { inner: self.inner.translate(dx, dy, dz) }
    }

    /// Return a new Node scaled by (sx, sy, sz).
    fn scale(&self, sx: f64, sy: f64, sz: f64) -> PyNode {
        PyNode { inner: self.inner.scale(sx, sy, sz) }
    }

    /// Return a new Node rotated around the X axis by `angle_rad` (right-hand rule).
    fn rot_x(&self, angle_rad: f64) -> PyNode {
        PyNode { inner: self.inner.rot_x(angle_rad) }
    }

    /// Return a new Node rotated around the Y axis by `angle_rad` (right-hand rule).
    fn rot_y(&self, angle_rad: f64) -> PyNode {
        PyNode { inner: self.inner.rot_y(angle_rad) }
    }

    /// Return a new Node rotated around the Z axis by `angle_rad` (right-hand rule).
    fn rot_z(&self, angle_rad: f64) -> PyNode {
        PyNode { inner: self.inner.rot_z(angle_rad) }
    }

    /// Return a new Node rotated around `axis` by `angle_rad` (right-hand rule).
    /// `axis` may be any non-zero 3-element sequence; it is normalised internally.
    fn rot_aa(&self, axis: [f64; 3], angle_rad: f64) -> PyNode {
        PyNode { inner: self.inner.rot_aa(axis, angle_rad) }
    }
}

// ---------------------------------------------------------------------------
// Primitive constructor functions
// ---------------------------------------------------------------------------

#[cfg(feature = "extension-module")]
#[pyfunction]
fn sphere(r: f64) -> PyNode {
    PyNode { inner: CsgNode::sphere(r) }
}

#[cfg(feature = "extension-module")]
#[pyfunction]
fn cuboid(dx: f64, dy: f64, dz: f64) -> PyNode {
    PyNode { inner: CsgNode::cuboid(dx, dy, dz) }
}

#[cfg(feature = "extension-module")]
#[pyfunction]
fn cylinder(r: f64, h: f64) -> PyNode {
    PyNode { inner: CsgNode::cylinder(r, h) }
}

#[cfg(feature = "extension-module")]
#[pyfunction]
fn cone(r: f64, h: f64) -> PyNode {
    PyNode { inner: CsgNode::cone(r, h) }
}

// ---------------------------------------------------------------------------
// Module registration (called from lib.rs)
// ---------------------------------------------------------------------------

#[cfg(feature = "extension-module")]
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNode>()?;
    m.add_function(wrap_pyfunction!(sphere, m)?)?;
    m.add_function(wrap_pyfunction!(cuboid, m)?)?;
    m.add_function(wrap_pyfunction!(cylinder, m)?)?;
    m.add_function(wrap_pyfunction!(cone, m)?)?;
    Ok(())
}
