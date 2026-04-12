//! Python bindings for the jefscad CSG modeling API (via pyo3).

#[cfg(feature = "extension-module")]
use pyo3::prelude::*;

#[cfg(feature = "extension-module")]
use pyo3_stub_gen::derive::*;

#[cfg(feature = "extension-module")]
use std::sync::Arc;

#[cfg(feature = "extension-module")]
use crate::csg_lang::{CsgNode, SelectPolicy};

#[cfg(feature = "extension-module")]
use crate::mesher::{MeshOptions, TriMesh, mesh_solid, write_stl, write_obj};

// ---------------------------------------------------------------------------
// Python-visible Mesh class
// ---------------------------------------------------------------------------

/// A triangle mesh produced by tessellating a CSG solid.
///
/// Obtain one via `Node.mesh(resolution=32)`. Export with `save_stl` or `save_obj`.
#[cfg(feature = "extension-module")]
#[gen_stub_pyclass]
#[pyclass(name = "Mesh")]
pub struct PyMesh {
    inner: TriMesh,
}

#[cfg(feature = "extension-module")]
#[gen_stub_pymethods]
#[pymethods]
impl PyMesh {
    /// Number of triangles in the mesh.
    #[getter]
    fn triangle_count(&self) -> usize {
        self.inner.triangles.len()
    }

    /// Number of vertices in the mesh.
    #[getter]
    fn vertex_count(&self) -> usize {
        self.inner.vertices.len()
    }

    /// Write the mesh to a binary STL file at `path`.
    fn save_stl(&self, path: &str) -> PyResult<()> {
        write_stl_file_py(&self.inner, path)
    }

    /// Write the mesh to a Wavefront OBJ file at `path`.
    fn save_obj(&self, path: &str) -> PyResult<()> {
        write_obj_file_py(&self.inner, path)
    }

    fn __repr__(&self) -> String {
        format!(
            "Mesh(triangles={}, vertices={})",
            self.inner.triangles.len(),
            self.inner.vertices.len(),
        )
    }
}

#[cfg(feature = "extension-module")]
fn write_stl_file_py(mesh: &TriMesh, path: &str) -> PyResult<()> {
    let mut f = std::fs::File::create(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    write_stl(mesh, &mut f)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

#[cfg(feature = "extension-module")]
fn write_obj_file_py(mesh: &TriMesh, path: &str) -> PyResult<()> {
    let mut f = std::fs::File::create(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    write_obj(mesh, &mut f)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

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

    // --- mesh ---------------------------------------------------------------

    /// Tessellate this node into a triangle mesh.
    ///
    /// Args:
    ///     resolution: Number of segments per full circle (default 32).
    ///                 Higher values give smoother curves at the cost of more triangles.
    ///
    /// Returns:
    ///     A `Mesh` object with `save_stl` and `save_obj` export methods.
    #[pyo3(signature = (resolution=32))]
    fn mesh(&self, resolution: u32) -> PyMesh {
        use crate::brep_compiler::compile_csg_node;
        use crate::brep_kernel::SolidModelingContext;
        let mut ctx = SolidModelingContext::new();
        let sid = compile_csg_node(&mut ctx, &self.inner);
        let tri_mesh = mesh_solid(&ctx, sid, &MeshOptions { resolution, ..MeshOptions::default() });
        PyMesh { inner: tri_mesh }
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
    m.add_class::<PyMesh>()?;
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
