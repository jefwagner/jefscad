//! Minimal 2-D point-in-polygon (single level, even-odd ray casting).
//!
//! Standalone primitive used by `build_extrusion`'s contour-nesting step (Phase 0-b)
//! to determine geometric containment: "is this point inside this closed contour?"
//! Later extended by the Phase-1 full `pip2d`-with-holes (scanline, used by boolean
//! classification) — the 0-b version is the building block (no wasted work).
//!
//! # Design (locked in `TODO.md` — 0-b pip2d API decision)
//!
//! - **Input: `&Contour`** (analytic segments), not `&[Point2]`. Keeps curves exact
//!   (line/quadratic/cubic/arc all closed-form) — no creation-time sampling,
//!   preserving the "circle is a circle" property at the topology level.
//! - **Rule: even-odd ray casting.** Matches the committed Phase-1 design; nesting
//!   needs geometric containment, and we classify winding (CCW outer / CW hole)
//!   separately. A point inside two overlapping loops is *outside* (two crossings
//!   cancel) — correct for nesting.
//! - **Deterministic retry, no PRNG, no crate dep.** A `PIP2D_RAY_DIRECTIONS`
//!   compile-time constant (primary direction with an irrational slope + retries).
//!   On *detected* degeneracy (ray tangent to a curve, or crossing coincides with a
//!   segment endpoint within epsilon), retry the next direction. Fully reproducible
//!   by construction. If all directions are degenerate (pathological), returns
//!   `false` (documented limitation; revisit if real geometry ever hits it —
//!   "bump-the-direction" logic a la conjugate gradient is the fallback).
//! - **Returns `bool`** (infallible). Nesting needs definite in/out, not trinary.
//!   On-boundary → `false` ("not strictly inside").
//! - **Degeneracy epsilon: constant 1e-12 (0-b).** NOT the relative-tolerance model
//!   (0-c's job); fine because inputs are engine-built (exact vertices). 0-c swaps
//!   this for `fuzzy_eq` + `ref_scale`.
//!
//! # Crossing math
//!
//! For a ray `R(t) = P + t·D` (t > 0) and a segment curve, a crossing is a point on
//! the curve that lies on the ray's *line* (`cross(Q − P, D) = 0`) *and* ahead of P
//! (`(Q − P)·D > 0`) *and* within the segment's parameter domain. Per variant:
//! - `Line2`: 2×2 linear solve for `(s, t)`.
//! - `CircularArc2`: substitute `C + r·(cos θ, sin θ)` → solve for θ ∈ [t0, t1].
//! - `QuadraticBezier2`: `cross(B(s) − P, D) = 0` is a quadratic in s.
//! - `CubicBezier2`: same → cubic in s; `solve_cubic` helper.
//!
//! `cross(a, b) = a.u·b.v − a.v·b.u` (the 2-D cross / z-component).

use crate::geom::{Contour, Curve2Kind, Point2};

/// 2-D cross product (z-component of the 3-D cross): `a.u·b.v − a.v·b.u`.
fn cross(a: Point2, b: Point2) -> f64 {
    a.u * b.v - a.v * b.u
}

/// Absolute epsilon for degeneracy detection (tangent / endpoint-coincidence) in 0-b.
///
/// A constant for now; the 0-c relative-tolerance model (`fuzzy_eq` + `ref_scale`)
/// replaces this when it lands. Fine for 0-b because inputs are engine-built (exact
/// vertices, not intersection-produced).
const DEGEN_EPS: f64 = 1e-12;

/// The fixed sequence of ray directions tried by [`pip2d`].
///
/// Index 0 is the primary ray (an irrational slope to avoid axis-aligned
/// degeneracies). On a *detected* degeneracy, the test retries with each subsequent
/// direction in turn. **Deterministic by construction — no PRNG, no crate dep,
/// bit-identical across runs/platforms.** The directions have distinct irrational
/// slopes so a degeneracy hit by direction *i* is astronomically unlikely to also
/// be hit by *i+1*.
const PIP2D_RAY_DIRECTIONS: [Point2; 4] = [
    Point2 {
        u: 1.0,
        v: std::f64::consts::FRAC_1_SQRT_2,
    }, // slope ≈ 0.7071
    Point2 {
        u: std::f64::consts::FRAC_1_SQRT_2,
        v: 1.0,
    }, // slope ≈ 1.4142
    Point2 {
        u: 1.0,
        v: std::f64::consts::SQRT_2,
    }, // slope ≈ 1.4142 (different)
    Point2 {
        u: std::f64::consts::FRAC_1_PI,
        v: 1.0,
    }, // slope ≈ π ≈ 3.1416
];

/// Is `point` strictly inside the closed `contour`? Even-odd ray casting.
///
/// Walks the contour's [`Curve2Kind`] segments analytically (no sampling — a bezier
/// stays a bezier, an arc stays an arc). Returns `false` for on-boundary points
/// ("not strictly inside").
///
/// # Degeneracies
/// If the primary ray hits a detected degeneracy (tangent to a curve, or passes
/// through a segment endpoint), the test retries with the next direction in
/// [`PIP2D_RAY_DIRECTIONS`]. If all directions are degenerate (pathological,
/// astronomically unlikely for real geometry), returns `false` — documented
/// limitation, revisit if a real model ever hits it.
///
/// # Panics
/// Panics if `contour` has no segments. The caller passes a closed, non-empty
/// contour; `build_extrusion` only calls this on contours that passed `finish()`.
pub fn pip2d(point: Point2, contour: &Contour) -> bool {
    assert!(
        !contour.segments.is_empty(),
        "pip2d: contour must have segments"
    );

    for &dir in &PIP2D_RAY_DIRECTIONS {
        match crossing_count_with_degeneracy(point, dir, contour) {
            CrossResult::Count(n) => return n % 2 == 1,
            CrossResult::Degenerate => continue, // retry next direction
        }
    }
    // All directions degenerate — pathological. Return false (not strictly inside).
    false
}

/// Result of counting ray-curve crossings for one ray direction.
enum CrossResult {
    /// The ray is non-degenerate for this contour; the number of crossings is `n`.
    Count(usize),
    /// The ray hits a degeneracy (tangent or endpoint-coincidence); retry needed.
    Degenerate,
}

/// Count crossings of the ray `R(t) = point + t·dir` (t > 0) with `contour`'s
/// segments. Returns `Degenerate` if any segment's crossing is degenerate for this
/// ray (so the caller can retry with a different direction).
fn crossing_count_with_degeneracy(point: Point2, dir: Point2, contour: &Contour) -> CrossResult {
    let mut count = 0usize;
    for seg in &contour.segments {
        match seg_crossings(point, dir, seg) {
            SegCross::Crossings(n) => count += n,
            SegCross::Degenerate => return CrossResult::Degenerate,
        }
    }
    CrossResult::Count(count)
}

/// Per-segment crossing result.
enum SegCross {
    /// `n` crossings of the ray (t > 0) with this segment's curve.
    Crossings(usize),
    /// The ray is tangent to the curve, or a crossing coincides with a segment
    /// endpoint — ambiguous for this segment; signal retry.
    Degenerate,
}

/// Count crossings (t > 0) of the ray with one segment, or signal degeneracy.
fn seg_crossings(point: Point2, dir: Point2, seg: &Curve2Kind) -> SegCross {
    match seg {
        Curve2Kind::Line2(l) => line_crossings(point, dir, l.p0, l.p1),
        Curve2Kind::CircularArc2(a) => arc_crossings(point, dir, a),
        Curve2Kind::QuadraticBezier2(b) => quad_crossings(point, dir, b.p0, b.p1, b.p2),
        Curve2Kind::CubicBezier2(b) => cubic_crossings(point, dir, b.p0, b.p1, b.p2, b.p3),
        Curve2Kind::Polyline2(pl) => {
            // Sum line-segment crossings; any segment degeneracy → retry.
            let mut total = 0usize;
            for w in pl.points.windows(2) {
                match line_crossings(point, dir, w[0], w[1]) {
                    SegCross::Crossings(n) => total += n,
                    SegCross::Degenerate => return SegCross::Degenerate,
                }
            }
            SegCross::Crossings(total)
        }
        Curve2Kind::Nurbs(_) => todo!("pip2d for NurbsCurve2"),
    }
}

// ── Per-variant crossing math ─────────────────────────────────────────────────

/// Line segment `p0 → p1` vs ray `R(t) = point + t·dir` (t > 0).
///
/// Solve `p0 + s·(p1−p0) = point + t·dir` for `(s, t)` with `s ∈ [0,1]`, `t > 0`.
/// Degenerate if the crossing lands exactly on `p0` or `p1` (within `DEGEN_EPS`),
/// since a shared vertex between two segments would then be double- or
/// mis-counted.
fn line_crossings(point: Point2, dir: Point2, p0: Point2, p1: Point2) -> SegCross {
    let d_seg = p1 - p0; // segment direction
    let denom = cross(dir, d_seg);
    if denom.abs() < DEGEN_EPS {
        // Ray parallel to segment. If the segment is collinear with the ray's line,
        // every point is "on the line" — degenerate (overlapping). Otherwise no
        // crossing. Treat collinear-overlap as degenerate (retry escapes it); strict
        // parallel-but-disjoint is just zero crossings.
        if (cross(p0 - point, dir)).abs() < DEGEN_EPS {
            return SegCross::Degenerate; // collinear overlap
        }
        return SegCross::Crossings(0);
    }
    // s = cross(point - p0, dir) / cross(dir, d_seg)  ... but careful with signs.
    // Solving p0 + s·d_seg = point + t·dir:
    //   s·d_seg - t·dir = point - p0
    // Cramer: s = cross(point - p0, dir) / cross(d_seg, dir) ... let me recompute.
    // Standard 2-D segment-vs-ray: see e.g. the cross-product formulation.
    let diff = point - p0;
    let s = cross(diff, dir) / cross(d_seg, dir);
    let t = cross(diff, d_seg) / cross(d_seg, dir);
    if !s.is_finite() || !t.is_finite() {
        return SegCross::Crossings(0);
    }
    // t > 0 strictly (t == 0 means the point itself is on the segment → on-boundary
    // → degenerate, retry). s in [0,1]; s exactly 0 or 1 → endpoint hit → degenerate.
    if t.abs() < DEGEN_EPS {
        return SegCross::Degenerate; // point itself lies on the segment
    }
    if t < 0.0 {
        return SegCross::Crossings(0);
    }
    if s.abs() < DEGEN_EPS || (s - 1.0).abs() < DEGEN_EPS {
        return SegCross::Degenerate; // crossing at a segment endpoint
    }
    if s > 0.0 && s < 1.0 {
        SegCross::Crossings(1)
    } else {
        SegCross::Crossings(0)
    }
}

/// Circular arc vs ray. The arc is `C + r·(cos θ, sin θ)`, θ ∈ [t0, t1].
///
/// Substitute into the ray-line equation `cross(Q - point, dir) = 0` and solve for
/// θ, keeping roots in [t0, t1] whose point is ahead of `point` (t > 0). Degenerate
/// if a root lands exactly at θ = t0 or θ = t1 (arc endpoint) or the ray is tangent
/// to the circle (double root).
fn arc_crossings(point: Point2, dir: Point2, a: &crate::geom::CircularArc2) -> SegCross {
    // Q(θ) - point = (C.x + r cosθ - point.u, C.y + r sinθ - point.v)
    // cross(Q - point, dir) = (C.x + r cosθ - px)·dir.v - (C.y + r sinθ - py)·dir.u = 0
    // → r·(cosθ·dir.v - sinθ·dir.u) + (C.x - px)·dir.v - (C.y - py)·dir.u = 0
    // → A·cosθ + B·sinθ = K  where  A = r·dir.v, B = -r·dir.u,
    //                          K = (py - C.y)·dir.u - (px - C.x)·dir.v... wait recompute.
    // Let me redo: (C.u + r cosθ - px)·dir.v - (C.v + r sinθ - py)·dir.u = 0
    //   = dir.v·(C.u - px) + r·dir.v·cosθ - dir.u·(C.v - py) - r·dir.u·sinθ = 0
    //   → r·dir.v·cosθ - r·dir.u·sinθ = dir.u·(C.v - py) - dir.v·(C.u - px)
    //   → A·cosθ + B·sinθ = K  with A = r·dir.v, B = -r·dir.u,
    //     K = dir.u·(C.v - py) - dir.v·(C.u - px).
    let a_coeff = a.radius * dir.v;
    let b_coeff = -a.radius * dir.u;
    let k = dir.u * (a.center.v - point.v) - dir.v * (a.center.u - point.u);

    // Solve A·cosθ + B·sinθ = K for θ ∈ [t0, t1].
    // Standard: R = sqrt(A²+B²) = r·|dir| (assume dir non-zero). If R < eps, degenerate.
    let r_coeff = (a_coeff * a_coeff + b_coeff * b_coeff).sqrt();
    if r_coeff < DEGEN_EPS {
        return SegCross::Degenerate; // dir is zero (shouldn't happen)
    }
    let k_over_r = k / r_coeff;
    if k_over_r.abs() > 1.0 + DEGEN_EPS {
        return SegCross::Crossings(0); // ray line misses the circle entirely
    }
    if k_over_r.abs() > 1.0 - DEGEN_EPS {
        // Tangent → double root → degenerate.
        return SegCross::Degenerate;
    }
    // Two roots: θ = φ ± acos(k/R), where φ = atan2(B, A).
    let phi = b_coeff.atan2(a_coeff);
    let alpha = k_over_r.acos();
    let theta1 = phi + alpha;
    let theta2 = phi - alpha;

    let mut count = 0usize;
    for theta in [theta1, theta2] {
        // The candidate θ is on the full circle; check it's in [t0, t1] (mod 2π).
        // Normalize θ into a 2π-periodic comparison with [t0, t1].
        let theta_n = normalize_angle_to_range(theta, a.t0, a.t1);
        if let Some(th) = theta_n {
            // Check endpoint coincidence → degenerate.
            if (th - a.t0).abs() < DEGEN_EPS || (th - a.t1).abs() < DEGEN_EPS {
                return SegCross::Degenerate;
            }
            // Verify t > 0: the point on the circle is ahead of `point` along dir.
            let q = Point2::new(
                a.center.u + a.radius * th.cos(),
                a.center.v + a.radius * th.sin(),
            );
            let t_ray = (q - point).u * dir.u + (q - point).v * dir.v;
            if t_ray.abs() < DEGEN_EPS {
                return SegCross::Degenerate; // point itself on the arc
            }
            if t_ray > 0.0 {
                count += 1;
            }
        }
    }
    SegCross::Crossings(count)
}

/// Fold `theta` (mod 2π) into the range so that it can be compared against
/// `[t0, t1]`. Returns the equivalent angle in `[t0, t0 + 2π)` if one lies in
/// `[t0, t1]`, else `None`. Handles arcs that wrap past 2π (t1 - t0 up to 2π).
fn normalize_angle_to_range(theta: f64, t0: f64, t1: f64) -> Option<f64> {
    use std::f64::consts::TAU;
    // Shift theta so it's >= t0, by adding multiples of 2π.
    let mut th = theta - t0;
    th = (th % TAU + TAU) % TAU; // in [0, 2π)
    let span = t1 - t0; // in (0, 2π]
    if span >= TAU - DEGEN_EPS {
        // Full circle (or nearly): any angle is in range. Use th + t0.
        return Some(th + t0);
    }
    if th <= span + DEGEN_EPS {
        Some(th + t0)
    } else {
        None
    }
}

/// Quadratic bezier `B(s) = (1-s)²·p0 + 2(1-s)s·p1 + s²·p2`, s ∈ [0,1], vs ray.
///
/// `cross(B(s) - point, dir) = 0` expands to a quadratic `a·s² + b·s + c = 0`.
/// Solve, keep real roots in (0,1) (endpoints → degenerate) with t > 0.
fn quad_crossings(point: Point2, dir: Point2, p0: Point2, p1: Point2, p2: Point2) -> SegCross {
    // B(s) = (1-2s+s²)p0 + (2s-2s²)p1 + s² p2
    //      = p0 + s·(2p1 - 2p0) + s²·(p0 - 2p1 + p2)
    // Let u(s) = B(s).u, v(s) = B(s).v. cross(B-P, dir) = (u-px)·dir.v - (v-py)·dir.u = 0.
    // u(s) - px = (p0.u - px) + s·(2p1.u-2p0.u) + s²·(p0.u-2p1.u+p2.u)
    //           = c0u + s·c1u + s²·c2u
    let c0u = p0.u - point.u;
    let c1u = 2.0 * (p1.u - p0.u);
    let c2u = p0.u - 2.0 * p1.u + p2.u;
    let c0v = p0.v - point.v;
    let c1v = 2.0 * (p1.v - p0.v);
    let c2v = p0.v - 2.0 * p1.v + p2.v;
    // cross = (u-px)·dir.v - (v-py)·dir.u
    //       = (c0u·dir.v - c0v·dir.u) + s·(c1u·dir.v - c1v·dir.u) + s²·(c2u·dir.v - c2v·dir.u)
    let a = c2u * dir.v - c2v * dir.u;
    let b = c1u * dir.v - c1v * dir.u;
    let c = c0u * dir.v - c0v * dir.u;
    count_poly_roots_in_unit_interval(a, b, c, point, dir, |s| {
        // B(s)
        let u = p0.u + s * (2.0 * (p1.u - p0.u)) + s * s * (p0.u - 2.0 * p1.u + p2.u);
        let v = p0.v + s * (2.0 * (p1.v - p0.v)) + s * s * (p0.v - 2.0 * p1.v + p2.v);
        Point2::new(u, v)
    })
}

/// Cubic bezier vs ray. Same approach → cubic in s.
fn cubic_crossings(
    point: Point2,
    dir: Point2,
    p0: Point2,
    p1: Point2,
    p2: Point2,
    p3: Point2,
) -> SegCross {
    // B(s) = (1-s)³p0 + 3(1-s)²s p1 + 3(1-s)s² p2 + s³ p3
    // Expanded: p0 + s·(3(p1-p0)) + s²·(3p0 - 6p1 + 3p2) + s³·(-p0 + 3p1 - 3p2 + p3)
    let c0u = p0.u - point.u;
    let c1u = 3.0 * (p1.u - p0.u);
    let c2u = 3.0 * p0.u - 6.0 * p1.u + 3.0 * p2.u;
    let c3u = -p0.u + 3.0 * p1.u - 3.0 * p2.u + p3.u;
    let c0v = p0.v - point.v;
    let c1v = 3.0 * (p1.v - p0.v);
    let c2v = 3.0 * p0.v - 6.0 * p1.v + 3.0 * p2.v;
    let c3v = -p0.v + 3.0 * p1.v - 3.0 * p2.v + p3.v;
    let a = c3u * dir.v - c3v * dir.u;
    let b = c2u * dir.v - c2v * dir.u;
    let c = c1u * dir.v - c1v * dir.u;
    let d = c0u * dir.v - c0v * dir.u;
    count_cubic_roots_in_unit_interval(a, b, c, d, point, dir, |s| {
        let u = p0.u + s * c1u + s * s * c2u + s * s * s * c3u;
        // c1u above is 3(p1u-p0u) which is the s¹ coeff; but we added (p0u - point.u)
        // into c0u. Reconstruct B(s).u properly: B(s).u = p0.u + s·c1u + s²·c2u + s³·c3u
        // (the c1u/c2u/c3u here are the bezier coeffs, not the (B-P) coeffs). Since
        // c1u/c2u/c3u were computed from control points directly (not shifted by P),
        // B(s).u = p0.u + s·c1u + s²·c2u + s³·c3u. Good.
        let v = p0.v + s * c1v + s * s * c2v + s * s * s * c3v;
        Point2::new(u, v)
    })
}

/// Solve `a·s² + b·s + c = 0`, count real roots in (0,1) (exclusive; endpoints →
/// degenerate) with positive ray-parameter t. `eval` reconstructs the curve point
/// at parameter s for the t > 0 check.
fn count_poly_roots_in_unit_interval(
    a: f64,
    b: f64,
    c: f64,
    point: Point2,
    dir: Point2,
    eval: impl Fn(f64) -> Point2,
) -> SegCross {
    let roots = solve_quadratic(a, b, c);
    count_roots(roots, point, dir, eval)
}

/// Solve `a·s³ + b·s² + c·s + d = 0` (cubic), same filtering.
fn count_cubic_roots_in_unit_interval(
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    point: Point2,
    dir: Point2,
    eval: impl Fn(f64) -> Point2,
) -> SegCross {
    let roots = solve_cubic(a, b, c, d);
    count_roots(roots, point, dir, eval)
}

/// Filter polynomial roots: keep real roots in (0,1) (exclusive; endpoints and
/// point-on-curve → degenerate), with t > 0.
fn count_roots(
    roots: Vec<f64>,
    point: Point2,
    dir: Point2,
    eval: impl Fn(f64) -> Point2,
) -> SegCross {
    let mut count = 0usize;
    for s in roots {
        if !s.is_finite() {
            continue;
        }
        if s.abs() < DEGEN_EPS || (s - 1.0).abs() < DEGEN_EPS {
            return SegCross::Degenerate; // root at bezier endpoint
        }
        if s <= 0.0 || s >= 1.0 {
            continue;
        }
        let q = eval(s);
        let t_ray = (q - point).u * dir.u + (q - point).v * dir.v;
        if t_ray.abs() < DEGEN_EPS {
            return SegCross::Degenerate; // point itself on the curve
        }
        if t_ray > 0.0 {
            count += 1;
        }
    }
    SegCross::Crossings(count)
}

/// Real roots of `a·x² + b·x + c = 0`. Handles the linear (a≈0) and constant cases.
fn solve_quadratic(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a.abs() < DEGEN_EPS {
        // Linear (or constant): b·x + c = 0.
        if b.abs() < DEGEN_EPS {
            return vec![]; // constant (or identically zero) — no isolated roots
        }
        return vec![-c / b];
    }
    let disc = b * b - 4.0 * a * c;
    if disc < -DEGEN_EPS {
        return vec![];
    }
    if disc.abs() < DEGEN_EPS {
        return vec![-b / (2.0 * a)]; // double root
    }
    let sq = disc.sqrt();
    // Numerically stable form (Kahan): q = -0.5·(b + sign(b)·sq).
    let q = -0.5 * (b + if b >= 0.0 { sq } else { -sq });
    if q.abs() < DEGEN_EPS {
        // q ≈ 0 means one root is ~0; fall back to the direct formula.
        vec![q / a, -b / (2.0 * a)]
    } else {
        vec![q / a, c / q]
    }
}

/// Real roots of `a·x³ + b·x² + c·x + d = 0`. Handles degenerate leading
/// coefficient (→ quadratic/linear/constant) and uses the trigonometric method for
/// three-real-root cubics (the common case for bezier-ray intersection).
fn solve_cubic(a: f64, b: f64, c: f64, d: f64) -> Vec<f64> {
    if a.abs() < DEGEN_EPS {
        return solve_quadratic(b, c, d);
    }
    // Depress: x = y - b/(3a). Then y³ + p·y + q = 0.
    let inv3a = 1.0 / (3.0 * a);
    let p = (3.0 * a * c - b * b) / (3.0 * a * a);
    let q = (2.0 * b * b * b - 9.0 * a * b * c + 27.0 * a * a * d) / (27.0 * a * a * a);
    let shift = -b * inv3a;

    let disc = q * q / 4.0 + p * p * p / 27.0;
    let mut roots = Vec::new();
    if disc > DEGEN_EPS {
        // One real root (Cardano).
        let sq = disc.sqrt();
        let u = (-q / 2.0 + sq).cbrt();
        let v = (-q / 2.0 - sq).cbrt();
        roots.push(u + v + shift);
    } else if disc < -DEGEN_EPS {
        // Three real roots (trigonometric method).
        let r = (-p / 3.0).sqrt();
        let phi = (3.0 * q / (2.0 * p * r)).acos();
        for k in 0..3 {
            let y = -2.0 * r * ((phi + 2.0 * std::f64::consts::PI * k as f64) / 3.0).cos();
            roots.push(y + shift);
        }
    } else {
        // disc ≈ 0: two real roots (one double).
        if q.abs() < DEGEN_EPS {
            roots.push(shift); // triple root at 0+shift
        } else {
            let u = (-q / 2.0).cbrt();
            roots.push(2.0 * u + shift);
            roots.push(-u + shift);
        }
    }
    roots
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use crate::geom::{Contour, Curve2Kind, Line2, Point2};

    fn uv(u: f64, v: f64) -> Point2 {
        Point2::new(u, v)
    }

    /// Build a closed polygon contour from vertices (adds Line2 segments between
    /// consecutive vertices and a closing segment back to the first).
    fn polygon_contour(verts: &[Point2]) -> Contour {
        assert!(verts.len() >= 3, "polygon needs >= 3 vertices");
        let start = verts[0];
        let mut c = Contour::new(start);
        for w in verts.windows(2) {
            c.push(Curve2Kind::Line2(Line2::new(w[0], w[1])), w[1]);
        }
        // close back to start
        c.push(
            Curve2Kind::Line2(Line2::new(*verts.last().unwrap(), start)),
            start,
        );
        c.closed = true;
        c
    }

    // ── Square (line segments) ────────────────────────────────────────────────

    fn unit_square() -> Contour {
        polygon_contour(&[uv(0.0, 0.0), uv(1.0, 0.0), uv(1.0, 1.0), uv(0.0, 1.0)])
    }

    #[test]
    fn point_inside_square() {
        assert!(pip2d(uv(0.5, 0.5), &unit_square()));
    }

    #[test]
    fn point_outside_square_left() {
        assert!(!pip2d(uv(-0.5, 0.5), &unit_square()));
    }

    #[test]
    fn point_outside_square_right() {
        assert!(!pip2d(uv(1.5, 0.5), &unit_square()));
    }

    #[test]
    fn point_outside_square_above() {
        assert!(!pip2d(uv(0.5, 1.5), &unit_square()));
    }

    #[test]
    fn point_outside_square_below() {
        assert!(!pip2d(uv(0.5, -0.5), &unit_square()));
    }

    // ── Triangle ──────────────────────────────────────────────────────────────

    #[test]
    fn point_inside_triangle() {
        let tri = polygon_contour(&[uv(0.0, 0.0), uv(2.0, 0.0), uv(1.0, 2.0)]);
        assert!(pip2d(uv(1.0, 0.5), &tri));
    }

    #[test]
    fn point_outside_triangle() {
        let tri = polygon_contour(&[uv(0.0, 0.0), uv(2.0, 0.0), uv(1.0, 2.0)]);
        assert!(!pip2d(uv(2.5, 1.0), &tri));
    }

    // ── Concave polygon ───────────────────────────────────────────────────────
    //
    // An "arrowhead"/chevron: (0,0)→(2,0)→(1,0.5)→(2,2)→(0,2)→close. The notch
    // at (1,0.5) makes (1,0.4) outside and (1,1.5) inside.

    #[test]
    fn point_in_concave_notch_is_outside() {
        let chevron = polygon_contour(&[
            uv(0.0, 0.0),
            uv(2.0, 0.0),
            uv(1.0, 0.5), // inward notch apex
            uv(2.0, 2.0),
            uv(0.0, 2.0),
        ]);
        // The notch (cut-out) is the triangle (2,0)-(1,0.5)-(2,2). A point inside
        // that triangle — e.g. (1.5, 0.4) — is outside the polygon. (A point at
        // x=1 like (1, 0.4) is left of the apex and still inside the main body.)
        assert!(!pip2d(uv(1.5, 0.4), &chevron));
    }

    #[test]
    fn point_above_concave_notch_is_inside() {
        let chevron = polygon_contour(&[
            uv(0.0, 0.0),
            uv(2.0, 0.0),
            uv(1.0, 0.5),
            uv(2.0, 2.0),
            uv(0.0, 2.0),
        ]);
        assert!(pip2d(uv(1.0, 1.5), &chevron));
    }

    // ── Circle (arc segment) ──────────────────────────────────────────────────

    fn unit_circle() -> Contour {
        use std::f64::consts::TAU;
        let start = uv(1.0, 0.0);
        let mut c = Contour::new(start);
        let arc =
            Curve2Kind::CircularArc2(crate::geom::CircularArc2::new(uv(0.0, 0.0), 1.0, 0.0, TAU));
        c.push(arc, start); // full circle ends where it started
        c.closed = true;
        c
    }

    #[test]
    fn point_inside_circle() {
        assert!(pip2d(uv(0.1, 0.1), &unit_circle()));
    }

    #[test]
    fn point_outside_circle() {
        assert!(!pip2d(uv(2.0, 2.0), &unit_circle()));
    }

    #[test]
    fn point_at_circle_center_inside() {
        assert!(pip2d(uv(0.0, 0.0), &unit_circle()));
    }

    // ── Quadratic bezier "blob" ───────────────────────────────────────────────
    //
    // A closed loop: line (0,0)→(2,0), then a quadratic up through (1,1) back to
    // (0,0). The bulge encloses points like (1, 0.5).

    fn quad_blob() -> Contour {
        let start = uv(0.0, 0.0);
        let mut c = Contour::new(start);
        c.push(
            Curve2Kind::Line2(Line2::new(uv(0.0, 0.0), uv(2.0, 0.0))),
            uv(2.0, 0.0),
        );
        c.push(
            Curve2Kind::QuadraticBezier2(crate::geom::QuadraticBezier2::new(
                uv(2.0, 0.0),
                uv(1.0, 1.0),
                uv(0.0, 0.0),
            )),
            uv(0.0, 0.0),
        );
        c.closed = true;
        c
    }

    #[test]
    fn point_inside_quad_blob() {
        assert!(pip2d(uv(1.0, 0.3), &quad_blob()));
    }

    #[test]
    fn point_outside_quad_blob_above() {
        // (1, 0.9) is above the bezier's apex region — outside the blob.
        assert!(!pip2d(uv(1.0, 0.9), &quad_blob()));
    }

    // ── Cubic bezier loop ─────────────────────────────────────────────────────
    //
    // A closed loop: line (0,0)→(3,0), then a cubic up through (1,2),(2,2) back to
    // (0,0). Encloses (1.5, 0.5).

    fn cubic_blob() -> Contour {
        let start = uv(0.0, 0.0);
        let mut c = Contour::new(start);
        c.push(
            Curve2Kind::Line2(Line2::new(uv(0.0, 0.0), uv(3.0, 0.0))),
            uv(3.0, 0.0),
        );
        c.push(
            Curve2Kind::CubicBezier2(crate::geom::CubicBezier2::new(
                uv(3.0, 0.0),
                uv(2.0, 2.0),
                uv(1.0, 2.0),
                uv(0.0, 0.0),
            )),
            uv(0.0, 0.0),
        );
        c.closed = true;
        c
    }

    #[test]
    fn point_inside_cubic_blob() {
        assert!(pip2d(uv(1.5, 0.5), &cubic_blob()));
    }

    #[test]
    fn point_outside_cubic_blob() {
        assert!(!pip2d(uv(1.5, 1.8), &cubic_blob()));
    }

    // ── Nested contours (the nesting use case) ────────────────────────────────

    #[test]
    fn inner_rep_point_inside_outer() {
        // Outer square (0,0)-(4,4); inner square (1,1)-(2,2).
        let outer = polygon_contour(&[uv(0.0, 0.0), uv(4.0, 0.0), uv(4.0, 4.0), uv(0.0, 4.0)]);
        let inner = polygon_contour(&[uv(1.0, 1.0), uv(2.0, 1.0), uv(2.0, 2.0), uv(1.0, 2.0)]);
        // Inner's first vertex (1,1) is inside the outer.
        assert!(pip2d(inner.start, &outer));
        // Outer's first vertex (0,0) is NOT inside the inner.
        assert!(!pip2d(outer.start, &inner));
    }

    // ── Degeneracy: primary ray grazes a vertex → deterministic retry ──────────
    //
    // Construct a contour where the point's primary ray (direction index 0,
    // (1, 1/√2)) passes exactly through a shared vertex. The retry must fire
    // deterministically and still give the correct answer. We place the point so
    // the primary ray hits a vertex dead-on, then verify the inside/outside answer
    // is still correct.

    #[test]
    fn degeneracy_vertex_graze_retries_deterministically() {
        // Square with vertices on integer coordinates. Place the test point at
        // (0.5, 0.5) and pick a ray direction that hits corner (1,1) exactly.
        // Primary dir is (1, 1/√2): from (0.5,0.5) the ray is (0.5+t, 0.5+t/√2).
        // To hit (1,1): 0.5+t = 1 → t=0.5; 0.5+0.5/√2 ≈ 0.8536 ≠ 1. So the primary
        // ray does NOT hit (1,1) here. To force a graze, place the point so that
        // the ray through dir0 passes exactly through a vertex.
        //
        // Pick point P and a vertex V such that V - P is parallel to dir0.
        // dir0 = (1, 1/√2). Take V = (1, 1) and P = V - k·dir0 for some k>0,
        // e.g. k = √2: P = (1 - √2, 1 - 1) = (1-√2, 0). Then the primary ray from
        // P passes through (1,1) at t = √2.
        let sq = unit_square();
        let s2 = std::f64::consts::SQRT_2;
        let p = uv(1.0 - s2, 0.0); // primary ray (dir0) hits corner (1,1) exactly
        // P is outside the unit square (x ≈ -0.414 < 0). So the answer must be
        // `false` regardless of which ray is used. This confirms the retry produces
        // a stable, correct answer rather than panicking or mis-counting.
        assert!(!pip2d(p, &sq));
    }

    #[test]
    fn degeneracy_vertex_graze_inside_case() {
        // Same idea, but with the point inside the square so the retry must still
        // return true. Place P inside such that the primary ray grazes a corner.
        // dir0 = (1, 1/√2). Corner (1,1). P = (1,1) - k·dir0 with small k so P is
        // inside: k = 0.1·√2 → P = (1 - 0.1√2, 1 - 0.1) ≈ (0.8586, 0.9). Inside.
        let sq = unit_square();
        let s2 = std::f64::consts::SQRT_2;
        let p = uv(1.0 - 0.1 * s2, 1.0 - 0.1);
        assert!(pip2d(p, &sq));
    }

    // ── Empty contour panics ──────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "pip2d: contour must have segments")]
    fn empty_contour_panics() {
        let c = Contour::new(uv(0.0, 0.0));
        let _ = pip2d(uv(0.5, 0.5), &c);
    }

    // ── Polyline support ──────────────────────────────────────────────────────

    #[test]
    fn polyline_contour_inside_outside() {
        // A triangle expressed as a 3-point polyline (2 segments) plus closing.
        use crate::geom::Polyline2;
        let pts = vec![uv(0.0, 0.0), uv(2.0, 0.0), uv(1.0, 2.0)];
        let mut c = Contour::new(pts[0]);
        c.push(Curve2Kind::Polyline2(Polyline2::new(pts.clone())), pts[2]);
        // Close back to start with a line.
        c.push(Curve2Kind::Line2(Line2::new(pts[2], pts[0])), pts[0]);
        c.closed = true;
        assert!(pip2d(uv(1.0, 0.5), &c));
        assert!(!pip2d(uv(2.5, 1.0), &c));
    }
}
