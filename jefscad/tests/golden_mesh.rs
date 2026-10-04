//! Golden DCEL mesh snapshots — the tier-(b) safety net for the internal mesh.
//!
//! Each primitive is compiled and meshed through the public CSG API, and the
//! internal DCEL half-edge mesh is flattened by
//! [`jefscad::inspect::dump_mesh_csg_node`] into a schema-neutral [`MeshDump`].
//! This test canonicalises that dump to text and compares it against a frozen
//! fixture in `tests/golden_mesh/`.
//!
//! **Why the DCEL and not the `TriMesh`?** The internal half-edge mesh is the
//! source of truth for refinement and booleans; `TriMesh` is only the STL/OBJ
//! export view, derived by `to_trimesh()`. Pinning the DCEL keeps the guard
//! insulated from export changes *and* covers connectivity a triangle soup
//! hides — twin stitching, `brep_ref` attribution, and `is_constraint` flags.
//! It also honestly records today's partial meshing (only planar caps for
//! extrusions/revolutions): those fixtures *should* change when the mesher is
//! intentionally rewritten, which is a real algorithm change, not a refactor.
//!
//! This test is **tier (b)**: it must not be edited to accommodate a refactor.
//! If a snapshot changes, that is a change to the produced mesh and must be
//! called out rather than papered over by regenerating the fixture.
//!
//! To regenerate the fixtures (maintainer action, deliberately manual — there is
//! no in-test write path, so an agent cannot silently rewrite the golden data):
//!
//! ```text
//! cargo test --test golden_mesh print_golden -- --ignored --nocapture
//! ```
//!
//! then split the printed sections into `tests/golden_mesh/<name>.txt`.

use std::fmt::Write as _;

use _jefscad::csg_lang::{CsgNode, NodeRef};
use _jefscad::geom::{Path2D, Point2};
use _jefscad::inspect::{MeshDump, MeshVertexRefDump, dump_mesh_csg_node};

/// Closed CCW square used for single-contour extrusion.
fn square_path() -> Path2D {
    let mut p = Path2D::new();
    p.start_contour(Point2::new(0.0, 0.0))
        .expect("start contour");
    p.line_to(Point2::new(2.0, 0.0));
    p.line_to(Point2::new(2.0, 2.0));
    p.line_to(Point2::new(0.0, 2.0));
    p.line_to_close().expect("close contour");
    p.finish().expect("finish path")
}

/// Square outer contour (CCW) with a square hole (CW).
fn square_with_hole_path() -> Path2D {
    let mut p = Path2D::new();
    p.start_contour(Point2::new(0.0, 0.0)).expect("start outer");
    p.line_to(Point2::new(3.0, 0.0));
    p.line_to(Point2::new(3.0, 3.0));
    p.line_to(Point2::new(0.0, 3.0));
    p.line_to_close().expect("close outer");
    p.start_contour(Point2::new(1.0, 1.0)).expect("start inner");
    p.line_to(Point2::new(1.0, 2.0));
    p.line_to(Point2::new(2.0, 2.0));
    p.line_to(Point2::new(2.0, 1.0));
    p.line_to_close().expect("close inner");
    p.finish().expect("finish path")
}

/// Closed CCW rectangle in the X-Z half-plane for a solid of revolution.
fn revolve_profile_path() -> Path2D {
    let mut p = Path2D::new();
    p.start_contour(Point2::new(1.0, 0.0))
        .expect("start contour");
    p.line_to(Point2::new(2.0, 0.0));
    p.line_to(Point2::new(2.0, 1.0));
    p.line_to(Point2::new(1.0, 1.0));
    p.line_to_close().expect("close contour");
    p.finish().expect("finish path")
}

/// The `(name, node, resolution)` set pinned by the golden mesh fixtures, in
/// deterministic order.
fn golden_nodes() -> Vec<(&'static str, NodeRef, u32)> {
    vec![
        ("cuboid", CsgNode::cuboid(2.0, 3.0, 4.0), 8),
        ("cylinder", CsgNode::cylinder(1.0, 2.0), 8),
        ("cylinder_r16", CsgNode::cylinder(1.0, 2.0), 16),
        ("cone", CsgNode::cone(1.0, 2.0), 8),
        ("sphere", CsgNode::sphere(1.0), 8),
        (
            "cuboid_translated",
            CsgNode::cuboid(2.0, 3.0, 4.0).translate(1.0, 2.0, 3.0),
            8,
        ),
        (
            "cuboid_rotated_z",
            CsgNode::cuboid(2.0, 3.0, 4.0).rot_z(0.5),
            8,
        ),
        ("extrusion_single", CsgNode::extrude(square_path(), 2.0), 8),
        (
            "extrusion_hole",
            CsgNode::extrude(square_with_hole_path(), 2.0),
            8,
        ),
        ("revolve", CsgNode::revolve(revolve_profile_path()), 8),
    ]
}

// ── Canonical formatting ──────────────────────────────────────────────────────

/// Normalise negative zero and round to nine decimals. Deliberately tight:
/// the mesher is deterministic, so a refactor must reproduce identical values.
fn ff(x: f64) -> String {
    let x = if x == 0.0 { 0.0 } else { x };
    format!("{x:.9}")
}

fn p3(p: &[f64; 3]) -> String {
    format!("({},{},{})", ff(p[0]), ff(p[1]), ff(p[2]))
}

fn p2(p: &[f64; 2]) -> String {
    format!("({},{})", ff(p[0]), ff(p[1]))
}

fn ref_str(r: MeshVertexRefDump) -> String {
    match r {
        MeshVertexRefDump::Corner(i) => format!("Corner({i})"),
        MeshVertexRefDump::OnEdge(i) => format!("OnEdge({i})"),
        MeshVertexRefDump::OnFace(i) => format!("OnFace({i})"),
    }
}

fn canon(d: &MeshDump) -> String {
    let mut s = String::new();

    writeln!(s, "vertices {}", d.vertices.len()).unwrap();
    for (i, v) in d.vertices.iter().enumerate() {
        writeln!(
            s,
            "  v {i} pos={} uv={} n={} ref={}",
            p3(&v.pos),
            p2(&v.uv),
            p3(&v.normal),
            ref_str(v.brep_ref)
        )
        .unwrap();
    }

    writeln!(s, "half_edges {}", d.half_edges.len()).unwrap();
    for (i, he) in d.half_edges.iter().enumerate() {
        let twin = match he.twin {
            Some(t) => t.to_string(),
            None => "-".to_string(),
        };
        writeln!(
            s,
            "  he {i} twin={twin} next={} vertex={} face={} constraint={}",
            he.next, he.vertex, he.face, he.is_constraint
        )
        .unwrap();
    }

    writeln!(s, "faces {}", d.faces.len()).unwrap();
    for (i, f) in d.faces.iter().enumerate() {
        writeln!(s, "  f {i} he={f}").unwrap();
    }

    s
}

fn fixture_path(name: &str) -> String {
    format!(
        "{}/tests/golden_mesh/{name}.txt",
        env!("CARGO_MANIFEST_DIR")
    )
}

// ── The guard ─────────────────────────────────────────────────────────────────

#[test]
fn golden_mesh_snapshots() {
    for (name, node, resolution) in golden_nodes() {
        let actual = canon(&dump_mesh_csg_node(&node, resolution));
        let path = fixture_path(name);
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("missing golden fixture {path}: {e}"));
        assert_eq!(
            actual, expected,
            "golden DCEL mismatch for `{name}`: the produced mesh changed. \
             If this is an intentional change, update the fixture deliberately."
        );
    }
}

/// Maintainer-only regenerator: prints canonical dumps to stdout. Deliberately
/// does **not** write fixture files, so there is no in-test path to silently
/// rewrite the golden data.
#[test]
#[ignore = "prints golden text; run with --ignored --nocapture"]
fn print_golden() {
    for (name, node, resolution) in golden_nodes() {
        println!("===== {name} =====");
        print!("{}", canon(&dump_mesh_csg_node(&node, resolution)));
    }
}
