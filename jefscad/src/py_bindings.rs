//! Python bindings for the jefscad CSG modeling API (via pyo3).

#[cfg(feature = "extension-module")]
use pyo3::prelude::*;

#[cfg(feature = "extension-module")]
use pyo3_stub_gen::derive::*;

#[cfg(feature = "extension-module")]
use std::sync::Arc;

#[cfg(feature = "extension-module")]
use crate::csg_lang::{CsgNode, SelectPolicy};

// ---------------------------------------------------------------------------
// Python-visible Node class
// ---------------------------------------------------------------------------

/// A CSG node: a primitive or operation with an associated transform stack.
///
/// Create nodes with the module-level constructors (`sphere`, `cuboid`, etc.)
/// and chain transforms with the methods below. All transform methods return
/// a **new** Node; the original is never mutated.
#[cfg(feature = "extension-module")]
#[gen_stub_pyclass]
#[pyclass(name = "Node")]
pub struct PyNode {
    pub(crate) inner: crate::csg_lang::NodeRef,
}

#[cfg(feature = "extension-module")]
#[gen_stub_pymethods]
#[pymethods]
impl PyNode {
    // --- introspection ------------------------------------------------------

    /// Unique provenance ID for this node (assigned at construction time).
    #[getter]
    fn prov_id(&self) -> u64 {
        self.inner.prov_id
    }

    /// Geometry hash: equal for structurally identical shapes regardless of
    /// authoring order or Arc identity.
    #[getter]
    fn geom_id(&self) -> u64 {
        self.inner.geom_id
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
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
    ///
    /// Args:
    ///     axis: Rotation axis as `[x, y, z]`. Need not be a unit vector; normalised internally.
    ///     angle_rad: Rotation angle in radians.
    fn rot_aa(&self, axis: [f64; 3], angle_rad: f64) -> PyNode {
        PyNode { inner: self.inner.rot_aa(axis, angle_rad) }
    }
}

// ---------------------------------------------------------------------------
// Primitive constructor functions
// ---------------------------------------------------------------------------

/// Create a sphere with radius `r`, centered at the origin.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
fn sphere(r: f64) -> PyNode {
    PyNode { inner: CsgNode::sphere(r) }
}

/// Create an axis-aligned cuboid with one corner at the origin and the opposite corner at `(dx, dy, dz)`.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
fn cuboid(dx: f64, dy: f64, dz: f64) -> PyNode {
    PyNode { inner: CsgNode::cuboid(dx, dy, dz) }
}

/// Create a cylinder with radius `r` and height `h`.
/// The base circle lies in the z = 0 plane centered at the origin; the top is at z = h.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
fn cylinder(r: f64, h: f64) -> PyNode {
    PyNode { inner: CsgNode::cylinder(r, h) }
}

/// Create a cone with base radius `r` and height `h`.
/// The base circle lies in the z = 0 plane centered at the origin; the apex is at z = h.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
fn cone(r: f64, h: f64) -> PyNode {
    PyNode { inner: CsgNode::cone(r, h) }
}

// ---------------------------------------------------------------------------
// Op constructor functions
// ---------------------------------------------------------------------------

/// Return the boolean union of the given nodes: `union(a, b, c, ...)`.
///
/// Raises `ValueError` if no nodes are provided.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*children))]
fn union(children: Vec<Bound<'_, PyNode>>) -> PyResult<PyNode> {
    if children.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "union requires at least one child",
        ));
    }
    let refs: Vec<_> = children.iter().map(|n| Arc::clone(&n.borrow().inner)).collect();
    Ok(PyNode { inner: CsgNode::union(refs) })
}

/// Return the intersection (common volume) of the given nodes: `intersection(a, b, c, ...)`.
///
/// Raises `ValueError` if no nodes are provided.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*children))]
fn intersection(children: Vec<Bound<'_, PyNode>>) -> PyResult<PyNode> {
    if children.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "intersection requires at least one child",
        ));
    }
    let refs: Vec<_> = children.iter().map(|n| Arc::clone(&n.borrow().inner)).collect();
    Ok(PyNode { inner: CsgNode::intersection(refs) })
}

/// Subtract volumes from a base shape: `difference(base, sub1, sub2, ...)`.
///
/// Raises `ValueError` if no subtracted nodes are provided.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (base, *subtract))]
fn difference(base: Bound<'_, PyNode>, subtract: Vec<Bound<'_, PyNode>>) -> PyResult<PyNode> {
    if subtract.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "difference requires at least one node to subtract",
        ));
    }
    let base_ref = Arc::clone(&base.borrow().inner);
    let sub_refs: Vec<_> = subtract.iter().map(|n| Arc::clone(&n.borrow().inner)).collect();
    Ok(PyNode { inner: CsgNode::difference(base_ref, sub_refs) })
}

/// Select the single largest connected component of `node` by volume.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
fn select_largest(node: Bound<'_, PyNode>) -> PyNode {
    PyNode { inner: CsgNode::select(Arc::clone(&node.borrow().inner), SelectPolicy::LargestByVolume) }
}

/// Select the connected component of `node` whose centroid is closest to `point`.
/// `point` is a 3-element sequence `[x, y, z]`.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
fn select_closest_to(node: Bound<'_, PyNode>, point: [f64; 3]) -> PyNode {
    PyNode { inner: CsgNode::select(Arc::clone(&node.borrow().inner), SelectPolicy::ClosestToPoint { point }) }
}

/// Select the connected component of `node` that contains `point`.
/// `point` is a 3-element sequence `[x, y, z]`.
#[cfg(feature = "extension-module")]
#[gen_stub_pyfunction]
#[pyfunction]
fn select_contains(node: Bound<'_, PyNode>, point: [f64; 3]) -> PyNode {
    PyNode { inner: CsgNode::select(Arc::clone(&node.borrow().inner), SelectPolicy::ContainsPoint { point }) }
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
    m.add_function(wrap_pyfunction!(union, m)?)?;
    m.add_function(wrap_pyfunction!(intersection, m)?)?;
    m.add_function(wrap_pyfunction!(difference, m)?)?;
    m.add_function(wrap_pyfunction!(select_largest, m)?)?;
    m.add_function(wrap_pyfunction!(select_closest_to, m)?)?;
    m.add_function(wrap_pyfunction!(select_contains, m)?)?;
    Ok(())
}
