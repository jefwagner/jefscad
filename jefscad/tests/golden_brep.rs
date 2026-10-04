//! Golden b-rep snapshots — the tier-(b) safety net for the Phase 0-c schema
//! migration.
//!
//! Each primitive is compiled through the public CSG API and flattened by
//! [`jefscad::inspect::dump_csg_node`] into a schema-neutral [`BRepDump`]. This
//! test canonicalises that dump to text and compares it against a frozen fixture
//! in `tests/golden/`. The fixtures pin the *observable* topology and geometry of
//! every primitive, so 0-c can move convenience refs into `Context` side-tables
//! only if the dumped content stays bit-for-bit (within formatting) identical.
//!
//! This test is **tier (b)**: it must not be edited to accommodate a refactor.
//! If a refactor changes a snapshot, that is a change to the compiled B-rep, and
//! must be called out rather than papered over by regenerating the fixture.
//!
//! To regenerate the fixtures (maintainer action, deliberately manual — there is
//! no in-test write path, so an agent cannot silently rewrite the golden data):
//!
//! ```text
//! cargo test --test golden_brep print_golden -- --ignored --nocapture
//! ```
//!
//! then split the printed sections into `tests/golden/<name>.txt`.

use std::fmt::Write as _;

use _jefscad::csg_lang::{CsgNode, NodeRef};
use _jefscad::geom::{Path2D, Point2};
use _jefscad::inspect::{BRepDump, dump_csg_node};

/// Closed CCW unit-ish square used for single-contour extrusion.
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

/// Square outer contour (CCW) with a square hole (CW) — the glyph-like profile.
fn square_with_hole_path() -> Path2D {
    let mut p = Path2D::new();
    // Outer, CCW (positive signed area).
    p.start_contour(Point2::new(0.0, 0.0)).expect("start outer");
    p.line_to(Point2::new(3.0, 0.0));
    p.line_to(Point2::new(3.0, 3.0));
    p.line_to(Point2::new(0.0, 3.0));
    p.line_to_close().expect("close outer");
    // Inner, CW (negative signed area).
    p.start_contour(Point2::new(1.0, 1.0)).expect("start inner");
    p.line_to(Point2::new(1.0, 2.0));
    p.line_to(Point2::new(2.0, 2.0));
    p.line_to(Point2::new(2.0, 1.0));
    p.line_to_close().expect("close inner");
    p.finish().expect("finish path")
}

/// Closed CCW rectangle in the X-Z half-plane (u = radius, v = height) for a
/// solid of revolution; revolved into an annular tube.
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

/// CCW half-disc: an upper semicircular arc closed by a diameter line.
fn arc_disc_path() -> Path2D {
    let mut p = Path2D::new();
    p.start_contour(Point2::new(2.0, 0.0))
        .expect("start contour");
    p.arc_to(Point2::new(1.0, 0.0), std::f64::consts::PI);
    p.line_to_close().expect("close contour");
    p.finish().expect("finish path")
}

/// CCW region under a quadratic bezier bump.
fn quad_bump_path() -> Path2D {
    let mut p = Path2D::new();
    p.start_contour(Point2::new(2.0, 0.0))
        .expect("start contour");
    p.quad_to(Point2::new(1.0, 1.5), Point2::new(0.0, 0.0));
    p.line_to_close().expect("close contour");
    p.finish().expect("finish path")
}

/// CCW region under a cubic bezier bump.
fn cubic_bump_path() -> Path2D {
    let mut p = Path2D::new();
    p.start_contour(Point2::new(2.0, 0.0))
        .expect("start contour");
    p.cubic_to(
        Point2::new(1.5, 1.5),
        Point2::new(0.5, 1.5),
        Point2::new(0.0, 0.0),
    );
    p.line_to_close().expect("close contour");
    p.finish().expect("finish path")
}

/// The primitive set pinned by the golden fixtures, in deterministic order.
fn golden_nodes() -> Vec<(&'static str, NodeRef)> {
    vec![
        ("cuboid", CsgNode::cuboid(2.0, 3.0, 4.0)),
        ("cylinder", CsgNode::cylinder(1.0, 2.0)),
        ("cone", CsgNode::cone(1.0, 2.0)),
        ("sphere", CsgNode::sphere(1.0)),
        ("extrusion_single", CsgNode::extrude(square_path(), 2.0)),
        (
            "extrusion_hole",
            CsgNode::extrude(square_with_hole_path(), 2.0),
        ),
        ("extrusion_arc", CsgNode::extrude(arc_disc_path(), 2.0)),
        ("extrusion_quad", CsgNode::extrude(quad_bump_path(), 2.0)),
        ("extrusion_cubic", CsgNode::extrude(cubic_bump_path(), 2.0)),
        ("revolve", CsgNode::revolve(revolve_profile_path())),
        (
            "cuboid_translated",
            CsgNode::cuboid(2.0, 3.0, 4.0).translate(1.0, 2.0, 3.0),
        ),
        (
            "cuboid_rotated_z",
            CsgNode::cuboid(2.0, 3.0, 4.0).rot_z(0.5),
        ),
    ]
}

// ── Canonical formatting ──────────────────────────────────────────────────────

/// Normalise negative zero and round to nine decimals. Deterministic by
/// construction; the compiled values should be bit-identical across the 0-c
/// refactor, so this is deliberately tight.
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

fn p3_list(ps: &[[f64; 3]]) -> String {
    ps.iter().map(p3).collect::<Vec<_>>().join(",")
}

fn p2_list(ps: &[[f64; 2]]) -> String {
    ps.iter().map(p2).collect::<Vec<_>>().join(",")
}

fn canon(d: &BRepDump) -> String {
    let mut s = String::new();

    writeln!(s, "solids {}", d.solids.len()).unwrap();
    for (i, x) in d.solids.iter().enumerate() {
        writeln!(
            s,
            "  so {i} outer={} inners={:?}",
            x.outer_shell, x.inner_shells
        )
        .unwrap();
    }

    writeln!(s, "shells {}", d.shells.len()).unwrap();
    for (i, x) in d.shells.iter().enumerate() {
        writeln!(s, "  sh {i} outer={} faces={:?}", x.is_outer, x.faces).unwrap();
    }

    writeln!(s, "faces {}", d.faces.len()).unwrap();
    for (i, x) in d.faces.iter().enumerate() {
        writeln!(
            s,
            "  f {i} surface={} sense={} outer={} inners={:?}",
            x.surface, x.sense, x.outer_loop, x.inner_loops
        )
        .unwrap();
    }

    writeln!(s, "loops {}", d.loops.len()).unwrap();
    for (i, x) in d.loops.iter().enumerate() {
        writeln!(s, "  l {i} outer={} coedges={:?}", x.is_outer, x.coedges).unwrap();
    }

    writeln!(s, "coedges {}", d.coedges.len()).unwrap();
    for (i, x) in d.coedges.iter().enumerate() {
        writeln!(
            s,
            "  ce {i} edge={} orient={} pcurve={} uv=[{}]",
            x.edge,
            x.orientation,
            x.pcurve,
            p2_list(&x.pcurve_samples)
        )
        .unwrap();
    }

    writeln!(s, "edges {}", d.edges.len()).unwrap();
    for (i, x) in d.edges.iter().enumerate() {
        writeln!(
            s,
            "  e {i} curve={} v0={} v1={} t={}..{} xyz=[{}]",
            x.curve3,
            x.v0,
            x.v1,
            ff(x.t0),
            ff(x.t1),
            p3_list(&x.samples)
        )
        .unwrap();
    }

    writeln!(s, "vertices {}", d.vertices.len()).unwrap();
    for (i, x) in d.vertices.iter().enumerate() {
        writeln!(s, "  v {i} {}", p3(x)).unwrap();
    }

    writeln!(s, "surfaces {}", d.surfaces.len()).unwrap();
    for (i, x) in d.surfaces.iter().enumerate() {
        writeln!(s, "  s {i} {} samples=[{}]", x.kind, p3_list(&x.samples)).unwrap();
    }

    writeln!(s, "curves3 {}", d.curves3.len()).unwrap();
    for (i, x) in d.curves3.iter().enumerate() {
        writeln!(s, "  c3 {i} {x}").unwrap();
    }

    writeln!(s, "curves2 {}", d.curves2.len()).unwrap();
    for (i, x) in d.curves2.iter().enumerate() {
        writeln!(s, "  c2 {i} {x}").unwrap();
    }

    s
}

fn fixture_path(name: &str) -> String {
    format!("{}/tests/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"))
}

// ── The guard ─────────────────────────────────────────────────────────────────

#[test]
fn golden_brep_snapshots() {
    for (name, node) in golden_nodes() {
        let actual = canon(&dump_csg_node(&node));
        let path = fixture_path(name);
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("missing golden fixture {path}: {e}"));
        assert_eq!(
            actual, expected,
            "golden b-rep mismatch for `{name}`: the compiled B-rep changed. \
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
    for (name, node) in golden_nodes() {
        println!("===== {name} =====");
        print!("{}", canon(&dump_csg_node(&node)));
    }
}
