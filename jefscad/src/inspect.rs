//! Schema-neutral, read-only snapshots of a compiled B-rep.
//!
//! The golden b-rep safety net (`jefscad/tests/golden_brep.rs`) needs to observe
//! primitive compilation without depending on the internal b-rep schema. This
//! module is that observation boundary: it compiles a [`CsgNode`] and flattens
//! the resulting arena into plain data ([`BRepDump`]) made only of numbers,
//! indices, and type names.
//!
//! The dump deliberately uses only the **defining** part of the b-rep: the
//! top-down traversal (`Solid → Shell → Face → Loop → CoEdge → Edge → Vertex`)
//! and the geometric content it reaches. It never reads a convenience up-ref
//! (`face.shell`, `loop.face`, `coedge.face`, `shell.solid`, `edge.coedges`,
//! `vertex.tol`) — those are exactly what the Phase 0-c refactor moves into
//! `Context` side-tables, so avoiding them keeps this inspection boundary
//! stable across that migration.
//!
//! Geometry is captured by sampling rather than by dumping struct fields: each
//! surface is evaluated on a small fixed UV grid, each edge's curve at its
//! `[t0, t1]` endpoints and midpoint, and each coedge's pcurve likewise. This
//! keeps [`BRepDump`] decoupled from the concrete geometric struct layouts.
//! NURBS variants are tagged but not sampled — their `eval` is still `todo!`.
//!
//! The module also snapshots the internal mesher output: [`dump_mesh_csg_node`]
//! builds the DCEL half-edge mesh via the mesher and flattens it into a
//! [`MeshDump`]. This captures the *internal* mesh — the source of truth for
//! refinement and booleans — not the presentation `TriMesh` used for STL/OBJ
//! export.

use crate::brep_compiler::compile_csg_node;
use crate::brep_kernel::{FaceSense, Orientation, SolidModelingContext};
use crate::csg_lang::CsgNode;
use crate::geom::{Curve2, Curve2Kind, Curve3, Curve3Kind, Surface, SurfaceKind};
use crate::mesher::{HalfEdgeMesh, MeshOptions, MeshVertexRef, build_dcel};

/// Fixed UV grid used to fingerprint a surface (see module docs).
const SURFACE_UV: [(f64, f64); 5] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.31, 0.57)];

/// A plain-data snapshot of one compiled B-rep arena.
///
/// Arenas are indexed by their `Id.0` value, so e.g. `coedges[i]` corresponds to
/// `CoEdgeId(i)`. Cross-references between arenas are stored as those indices.
/// Field order is top-down (`solids` first) for readable canonical output.
#[derive(Debug, Clone, PartialEq)]
pub struct BRepDump {
    pub solids: Vec<SolidDump>,
    pub shells: Vec<ShellDump>,
    pub faces: Vec<FaceDump>,
    pub loops: Vec<LoopDump>,
    pub coedges: Vec<CoEdgeDump>,
    pub edges: Vec<EdgeDump>,
    pub vertices: Vec<[f64; 3]>,
    pub surfaces: Vec<SurfaceDump>,
    pub curves3: Vec<String>,
    pub curves2: Vec<String>,
}

/// One [`Solid`](crate::brep_kernel::Solid): outer shell plus void shells.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidDump {
    pub outer_shell: usize,
    pub inner_shells: Vec<usize>,
}

/// One [`Shell`](crate::brep_kernel::Shell): face list and outer/void flag.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellDump {
    pub is_outer: bool,
    pub faces: Vec<usize>,
}

/// One [`Face`](crate::brep_kernel::Face): surface, sense, and bounding loops.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceDump {
    pub surface: usize,
    pub sense: String,
    pub outer_loop: usize,
    pub inner_loops: Vec<usize>,
}

/// One [`Loop`](crate::brep_kernel::Loop): ordered coedge list.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopDump {
    pub is_outer: bool,
    pub coedges: Vec<usize>,
}

/// One [`CoEdge`](crate::brep_kernel::CoEdge): underlying edge, orientation, and
/// sampled pcurve.
#[derive(Debug, Clone, PartialEq)]
pub struct CoEdgeDump {
    pub edge: usize,
    pub orientation: String,
    pub pcurve: usize,
    /// Pcurve samples at the edge's `t0`, midpoint, and `t1`; empty for NURBS.
    pub pcurve_samples: Vec<[f64; 2]>,
}

/// One [`Edge`](crate::brep_kernel::Edge): curve, endpoints, parameter range,
/// and sampled curve geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeDump {
    pub curve3: usize,
    pub v0: usize,
    pub v1: usize,
    pub t0: f64,
    pub t1: f64,
    /// Curve samples at `t0`, midpoint, and `t1`; empty for NURBS/SSI.
    pub samples: Vec<[f64; 3]>,
}

/// One [`Surface`](crate::geom::Surface): type name plus UV-grid samples.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceDump {
    pub kind: String,
    /// Samples on [`SURFACE_UV`]; empty for NURBS.
    pub samples: Vec<[f64; 3]>,
}

/// Compile `node` in a fresh context and return a plain-data snapshot of the
/// resulting B-rep arena.
///
/// This is the public observation entry point for the golden safety net. Its
/// signature and the shape of [`BRepDump`] are intended to be stable across the
/// Phase 0-c schema migration, which only changes `dump_context`'s internals.
pub fn dump_csg_node(node: &CsgNode) -> BRepDump {
    let mut ctx = SolidModelingContext::new();
    let _root = compile_csg_node(&mut ctx, node);
    dump_context(&ctx)
}

/// Flatten every arena in `ctx` into a [`BRepDump`], in top-down order.
fn dump_context(ctx: &SolidModelingContext) -> BRepDump {
    let solids = ctx
        .solids
        .iter()
        .map(|s| SolidDump {
            outer_shell: s.outer.0,
            inner_shells: s.inners.iter().map(|sh| sh.0).collect(),
        })
        .collect();

    let shells = ctx
        .shells
        .iter()
        .map(|s| ShellDump {
            is_outer: s.is_outer,
            faces: s.faces.iter().map(|f| f.0).collect(),
        })
        .collect();

    let faces = ctx
        .faces
        .iter()
        .map(|f| FaceDump {
            surface: f.surface.0,
            sense: sense_name(f.sense).to_string(),
            outer_loop: f.outer.0,
            inner_loops: f.inners.iter().map(|l| l.0).collect(),
        })
        .collect();

    let loops = ctx
        .loops
        .iter()
        .map(|l| LoopDump {
            is_outer: l.is_outer,
            coedges: l.coedges.iter().map(|c| c.0).collect(),
        })
        .collect();

    let coedges = ctx
        .coedges
        .iter()
        .map(|ce| {
            let edge = ctx.get_edge(ce.edge);
            let pcurve = ctx.get_curve2(ce.pcurve);
            CoEdgeDump {
                edge: ce.edge.0,
                orientation: orientation_name(ce.orientation).to_string(),
                pcurve: ce.pcurve.0,
                pcurve_samples: sample_pcurve(pcurve, edge.t0, edge.t1),
            }
        })
        .collect();

    let edges = ctx
        .edges
        .iter()
        .map(|e| {
            let curve = ctx.get_curve3(e.curve3);
            EdgeDump {
                curve3: e.curve3.0,
                v0: e.v0.0,
                v1: e.v1.0,
                t0: e.t0,
                t1: e.t1,
                samples: sample_curve3(curve, e.t0, e.t1),
            }
        })
        .collect();

    let vertices = ctx
        .vertices
        .iter()
        .map(|v| [v.point.x, v.point.y, v.point.z])
        .collect();

    let surfaces = ctx.surfaces.iter().map(dump_surface).collect();

    let curves3 = ctx
        .curves3
        .iter()
        .map(curve3_name)
        .map(str::to_string)
        .collect();

    let curves2 = ctx
        .curves2
        .iter()
        .map(curve2_name)
        .map(str::to_string)
        .collect();

    BRepDump {
        solids,
        shells,
        faces,
        loops,
        coedges,
        edges,
        vertices,
        surfaces,
        curves3,
        curves2,
    }
}

// ── Naming ────────────────────────────────────────────────────────────────────

fn sense_name(sense: FaceSense) -> &'static str {
    match sense {
        FaceSense::Aligned => "Aligned",
        FaceSense::AntiAligned => "AntiAligned",
    }
}

fn orientation_name(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Forward => "Forward",
        Orientation::Reverse => "Reverse",
    }
}

fn surface_name(kind: &SurfaceKind) -> &'static str {
    match kind {
        SurfaceKind::Plane(_) => "Plane",
        SurfaceKind::Cylinder(_) => "Cylinder",
        SurfaceKind::Cone(_) => "Cone",
        SurfaceKind::Sphere(_) => "Sphere",
        SurfaceKind::Extrusion(_) => "Extrusion",
        SurfaceKind::Revolution(_) => "Revolution",
        SurfaceKind::Nurbs(_) => "Nurbs",
    }
}

fn curve3_name(kind: &Curve3Kind) -> &'static str {
    match kind {
        Curve3Kind::Line3(_) => "Line3",
        Curve3Kind::CircularArc3(_) => "CircularArc3",
        Curve3Kind::QuadraticBezier3(_) => "QuadraticBezier3",
        Curve3Kind::CubicBezier3(_) => "CubicBezier3",
        Curve3Kind::Polyline3(_) => "Polyline3",
        Curve3Kind::Nurbs(_) => "Nurbs",
        Curve3Kind::Ssi(_) => "Ssi",
    }
}

fn curve2_name(kind: &Curve2Kind) -> &'static str {
    match kind {
        Curve2Kind::Line2(_) => "Line2",
        Curve2Kind::CircularArc2(_) => "CircularArc2",
        Curve2Kind::QuadraticBezier2(_) => "QuadraticBezier2",
        Curve2Kind::CubicBezier2(_) => "CubicBezier2",
        Curve2Kind::Polyline2(_) => "Polyline2",
        Curve2Kind::Nurbs(_) => "Nurbs",
    }
}

// ── Sampling ──────────────────────────────────────────────────────────────────

fn dump_surface(kind: &SurfaceKind) -> SurfaceDump {
    let samples = match kind {
        // NURBS `eval` is not implemented yet; tag only.
        SurfaceKind::Nurbs(_) => Vec::new(),
        _ => SURFACE_UV
            .iter()
            .map(|&(u, v)| {
                let p = kind.eval(u, v);
                [p.x, p.y, p.z]
            })
            .collect(),
    };
    SurfaceDump {
        kind: surface_name(kind).to_string(),
        samples,
    }
}

/// Sample a 3-D curve at `t0`, the midpoint, and `t1`. Empty for NURBS/SSI.
fn sample_curve3(kind: &Curve3Kind, t0: f64, t1: f64) -> Vec<[f64; 3]> {
    match kind {
        Curve3Kind::Nurbs(_) | Curve3Kind::Ssi(_) => Vec::new(),
        _ => [t0, 0.5 * (t0 + t1), t1]
            .iter()
            .map(|&t| {
                let p = kind.eval(t);
                [p.x, p.y, p.z]
            })
            .collect(),
    }
}

/// Sample a pcurve at `t0`, the midpoint, and `t1`. Empty for NURBS.
fn sample_pcurve(kind: &Curve2Kind, t0: f64, t1: f64) -> Vec<[f64; 2]> {
    match kind {
        Curve2Kind::Nurbs(_) => Vec::new(),
        _ => [t0, 0.5 * (t0 + t1), t1]
            .iter()
            .map(|&t| {
                let p = kind.eval(t);
                [p.u, p.v]
            })
            .collect(),
    }
}

// ── Mesh (DCEL) inspection ────────────────────────────────────────────────────

/// Compile `node` and snapshot the internal DCEL half-edge mesh at `resolution`.
///
/// This is the mesh-side observation entry point for the golden safety net. It
/// is stable across the Phase 0-c schema migration: only `dump_dcel`'s internals
/// follow the mesher's field access, while this signature and the shape of
/// [`MeshDump`] stay fixed.
pub fn dump_mesh_csg_node(node: &CsgNode, resolution: u32) -> MeshDump {
    let mut ctx = SolidModelingContext::new();
    let root = compile_csg_node(&mut ctx, node);
    let opts = MeshOptions {
        resolution,
        ..MeshOptions::default()
    };
    let dcel = build_dcel(&ctx, root, &opts);
    dump_dcel(&dcel)
}

/// Flatten the internal [`HalfEdgeMesh`] into a plain-data [`MeshDump`].
fn dump_dcel(dcel: &HalfEdgeMesh) -> MeshDump {
    let vertices = dcel
        .vertices
        .iter()
        .map(|v| MeshVertexDump {
            pos: v.pos,
            uv: v.uv,
            normal: v.normal,
            brep_ref: match v.brep_ref {
                MeshVertexRef::Corner(id) => MeshVertexRefDump::Corner(id.0),
                MeshVertexRef::OnEdge(id) => MeshVertexRefDump::OnEdge(id.0),
                MeshVertexRef::OnFace(id) => MeshVertexRefDump::OnFace(id.0),
            },
        })
        .collect();

    let half_edges = dcel
        .half_edges
        .iter()
        .map(|he| HalfEdgeDump {
            twin: he.twin.map(|t| t.0),
            next: he.next.0,
            vertex: he.vertex.0,
            face: he.face.0,
            is_constraint: he.is_constraint,
        })
        .collect();

    let faces = dcel.faces.iter().map(|f| f.half_edge.0).collect();

    MeshDump {
        vertices,
        half_edges,
        faces,
    }
}

/// A plain-data snapshot of the internal DCEL half-edge mesh.
///
/// Arenas are indexed by position, mirroring `HalfEdgeMesh`'s `Vec`s, so
/// `half_edges[i]` corresponds to `HalfEdgeId(i)`. Cross-references are stored
/// as those indices.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshDump {
    pub vertices: Vec<MeshVertexDump>,
    pub half_edges: Vec<HalfEdgeDump>,
    /// Representative half-edge index per [`DcelFace`](crate::mesher::DcelFace).
    pub faces: Vec<usize>,
}

/// One internal mesh vertex: position, UV, normal, and B-rep attribution.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshVertexDump {
    pub pos: [f64; 3],
    pub uv: [f64; 2],
    pub normal: [f64; 3],
    pub brep_ref: MeshVertexRefDump,
}

/// Plain-data form of [`MeshVertexRef`](crate::mesher::MeshVertexRef).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshVertexRefDump {
    /// B-rep topological vertex index.
    Corner(usize),
    /// B-rep edge index.
    OnEdge(usize),
    /// B-rep face index.
    OnFace(usize),
}

/// One directed half-edge: connectivity plus the constraint flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HalfEdgeDump {
    pub twin: Option<usize>,
    pub next: usize,
    pub vertex: usize,
    pub face: usize,
    pub is_constraint: bool,
}
