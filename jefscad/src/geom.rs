//! Geometric primitives: points, curves, and surfaces.
//!
//! This module contains purely geometric types — coordinates, parameterized curves, and
//! parameterized surfaces. Topological types (edges, faces, shells, etc.) live in
//! `brep_kernel`.
//!
//! # Naming convention
//! Each geometry category has a **trait** (e.g. `Curve3`) that defines the evaluation
//! API, and a **`Kind` enum** (e.g. `Curve3Kind`) that is the concrete stored type in
//! the B-rep arena. Individual structs (`Line3`, `CircularArc3`, …) implement the trait
//! directly; `Curve3Kind` delegates to whichever variant it holds.

// ── Point3 ───────────────────────────────────────────────────────────────────

/// A point (or free vector) in 3-D space.
///
/// `Point3` doubles as a vector type for tangents and normals until a dedicated `Vec3`
/// type is warranted. Fields are public so callers can destructure freely.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    /// Construct a point from its three coordinates.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Euclidean length (magnitude) of the vector.
    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Return a unit vector in the same direction.
    ///
    /// # Panics
    /// Panics if `self` is the zero vector.
    pub fn normalize(self) -> Self {
        let len = self.length();
        assert!(len != 0.0, "cannot normalize the zero vector");
        self * (1.0 / len)
    }

    /// Dot product `self · rhs`.
    pub fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    /// Cross product `self × rhs`.
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }
}

impl std::ops::Add for Point3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Point3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f64> for Point3 {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

// ── Point2 ───────────────────────────────────────────────────────────────────

/// A point (or free vector) in the UV parameter space of a surface.
///
/// Fields are named `u` and `v` to reflect their role as surface parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub u: f64,
    pub v: f64,
}

impl Point2 {
    /// Construct a UV coordinate from its two components.
    pub fn new(u: f64, v: f64) -> Self {
        Self { u, v }
    }
}

impl std::ops::Add for Point2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.u + rhs.u, self.v + rhs.v)
    }
}

impl std::ops::Sub for Point2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.u - rhs.u, self.v - rhs.v)
    }
}

impl std::ops::Mul<f64> for Point2 {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Self::new(self.u * s, self.v * s)
    }
}

// ── Curve3 trait ─────────────────────────────────────────────────────────────

/// Evaluation interface for a parameterized curve in 3-D space.
///
/// Implementations must be consistent: `eval_dt` is the derivative of `eval` with
/// respect to `t`. The parameter domain `[t_min, t_max]` is stored in the concrete type
/// but `eval` and `eval_dt` do **not** clamp — callers are responsible for staying in
/// range (or knowingly extrapolating).
// Phase-0 staging: eval_dt / is_degenerate on the curve traits are not yet called
// by any consumer. Phase 0-b extends the curve API for bezier segments and Phase
// 0-c wires the b-rep compiler that consumes them; remove this allow then.
#[allow(dead_code)]
pub trait Curve3 {
    /// Evaluate the point on the curve at parameter `t`.
    fn eval(&self, t: f64) -> Point3;

    /// Evaluate the un-normalized tangent vector `d/dt eval(t)`.
    fn eval_dt(&self, t: f64) -> Point3;

    /// Returns `true` if the curve degenerates to a single point.
    ///
    /// Used to identify degenerate edges (e.g. at the pole of a sphere or the apex of a
    /// cone) without relying on tolerance — see `brep_notes.md`.
    fn is_degenerate(&self) -> bool;
}

// ── Line3 ─────────────────────────────────────────────────────────────────────

/// A straight line segment in 3-D space.
///
/// The parameterization is linear: `eval(0) = p0`, `eval(1) = p1`, and the formula
/// `p0 + t * (p1 - p0)` holds for all `t`. `t_min` and `t_max` record the intended
/// domain (normally `[0, 1]`) but are not enforced by `eval`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line3 {
    pub p0: Point3,
    pub p1: Point3,
    pub t_min: f64,
    pub t_max: f64,
}

impl Line3 {
    /// Construct a line segment from `p0` to `p1` with the standard domain `[0, 1]`.
    pub fn new(p0: Point3, p1: Point3) -> Self {
        Self {
            p0,
            p1,
            t_min: 0.0,
            t_max: 1.0,
        }
    }
}

impl Curve3 for Line3 {
    fn eval(&self, t: f64) -> Point3 {
        self.p0 + (self.p1 - self.p0) * t
    }

    /// The tangent of a line is constant: `p1 - p0`. The parameter `t` is unused.
    fn eval_dt(&self, _t: f64) -> Point3 {
        self.p1 - self.p0
    }

    fn is_degenerate(&self) -> bool {
        self.p0 == self.p1
    }
}

// ── CircularArc3 ──────────────────────────────────────────────────────────────

/// A circular arc (or full circle) in 3-D space.
///
/// Parameterized by angle `t` in radians. The frame in the circle's plane is:
/// - `ref_dir` at `t = 0`
/// - `normal × ref_dir` at `t = π/2`
///
/// The sweep follows the right-hand rule around `normal`. A full circle has
/// `t1 - t0 = 2π`.
///
/// The caller is responsible for ensuring `normal` and `ref_dir` are unit vectors and
/// mutually perpendicular.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularArc3 {
    pub center: Point3,
    pub normal: Point3,  // unit; defines the circle plane via right-hand rule
    pub ref_dir: Point3, // unit, ⊥ normal; direction at t = 0
    pub radius: f64,
    pub t0: f64, // start angle (radians)
    pub t1: f64, // end angle (radians); t1 > t0; t1 - t0 = 2π for full circle
}

impl CircularArc3 {
    pub fn new(
        center: Point3,
        normal: Point3,
        ref_dir: Point3,
        radius: f64,
        t0: f64,
        t1: f64,
    ) -> Self {
        Self {
            center,
            normal,
            ref_dir,
            radius,
            t0,
            t1,
        }
    }
}

impl Curve3 for CircularArc3 {
    fn eval(&self, t: f64) -> Point3 {
        let e2 = self.normal.cross(self.ref_dir);
        self.center + (self.ref_dir * t.cos() + e2 * t.sin()) * self.radius
    }

    fn eval_dt(&self, t: f64) -> Point3 {
        let e2 = self.normal.cross(self.ref_dir);
        (self.ref_dir * (-t.sin()) + e2 * t.cos()) * self.radius
    }

    fn is_degenerate(&self) -> bool {
        self.radius == 0.0
    }
}

// ── Stub types for remaining Curve3Kind variants ───────────────────────────────

/// A rational B-spline curve in 3-D space. Fields TBD — stub for `Curve3Kind`.
#[derive(Debug, Clone)]
pub struct NurbsCurve3;

/// A surface-surface intersection curve. Fields TBD — Phase 5.
#[derive(Debug, Clone)]
pub struct SsiCurve3;

// ── Polyline3 ─────────────────────────────────────────────────────────────────

/// A piecewise-linear 3-D curve through `N` control points (`N - 1` segments).
///
/// # Parameterization
/// `t ∈ [0, N-1]`: segment `i` covers `t ∈ [i, i+1]`.
/// `eval(t)` for `t` outside `[0, N-1]` extrapolates along the first (t < 0) or
/// last (t > N-1) segment, consistent with the no-clamping contract shared by all
/// [`Curve3`] types.
///
/// # Panics
/// [`Polyline3::new`] panics if fewer than 2 points are provided.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline3 {
    pub points: Vec<Point3>,
}

impl Polyline3 {
    /// Construct a polyline from a sequence of control points.
    ///
    /// Panics if `points.len() < 2`.
    pub fn new(points: Vec<Point3>) -> Self {
        assert!(points.len() >= 2, "Polyline3 requires at least 2 points");
        Self { points }
    }

    /// Number of linear segments (`points.len() - 1`).
    pub fn n_segments(&self) -> usize {
        self.points.len() - 1
    }
}

impl Curve3 for Polyline3 {
    fn eval(&self, t: f64) -> Point3 {
        let n = self.n_segments();
        let i = (t.floor() as i64).clamp(0, n as i64 - 1) as usize;
        let frac = t - i as f64;
        self.points[i] + (self.points[i + 1] - self.points[i]) * frac
    }

    /// Un-normalized tangent: the direction of the segment containing `t`.
    /// Constant within each segment; at segment boundaries returns the direction
    /// of the segment whose index equals `floor(t)` (clamped to valid range).
    fn eval_dt(&self, t: f64) -> Point3 {
        let n = self.n_segments();
        let i = (t.floor() as i64).clamp(0, n as i64 - 1) as usize;
        self.points[i + 1] - self.points[i]
    }

    fn is_degenerate(&self) -> bool {
        self.points.iter().all(|&p| p == self.points[0])
    }
}

// ── Curve3Kind enum ───────────────────────────────────────────────────────────

/// The concrete stored curve type used in the B-rep arena.
///
/// Each variant wraps a concrete curve struct that implements [`Curve3`]. Methods
/// delegate to the inner type; unimplemented variants panic with `todo!`.
#[derive(Debug, Clone)]
pub enum Curve3Kind {
    Line3(Line3),
    CircularArc3(CircularArc3),
    Polyline3(Polyline3),
    Nurbs(NurbsCurve3),
    Ssi(SsiCurve3),
}

impl Curve3 for Curve3Kind {
    fn eval(&self, t: f64) -> Point3 {
        match self {
            Curve3Kind::Line3(l) => l.eval(t),
            Curve3Kind::CircularArc3(a) => a.eval(t),
            Curve3Kind::Polyline3(p) => p.eval(t),
            Curve3Kind::Nurbs(_) => todo!("NurbsCurve3::eval"),
            Curve3Kind::Ssi(_) => todo!("SsiCurve3::eval"),
        }
    }

    fn eval_dt(&self, t: f64) -> Point3 {
        match self {
            Curve3Kind::Line3(l) => l.eval_dt(t),
            Curve3Kind::CircularArc3(a) => a.eval_dt(t),
            Curve3Kind::Polyline3(p) => p.eval_dt(t),
            Curve3Kind::Nurbs(_) => todo!("NurbsCurve3::eval_dt"),
            Curve3Kind::Ssi(_) => todo!("SsiCurve3::eval_dt"),
        }
    }

    fn is_degenerate(&self) -> bool {
        match self {
            Curve3Kind::Line3(l) => l.is_degenerate(),
            Curve3Kind::CircularArc3(a) => a.is_degenerate(),
            Curve3Kind::Polyline3(p) => p.is_degenerate(),
            Curve3Kind::Nurbs(_) => todo!("NurbsCurve3::is_degenerate"),
            Curve3Kind::Ssi(_) => todo!("SsiCurve3::is_degenerate"),
        }
    }
}

// ── Curve2 trait ──────────────────────────────────────────────────────────────

/// Evaluation interface for a parameterized curve in the UV parameter space of a surface.
///
/// The same no-clamping contract as [`Curve3`] applies: `eval` does not enforce the
/// domain stored in the concrete type.
#[allow(dead_code)] // see note on Curve3 above — Phase-0 staging
pub trait Curve2 {
    /// Evaluate the UV point on the curve at parameter `t`.
    fn eval(&self, t: f64) -> Point2;

    /// Evaluate the un-normalized tangent `d/dt eval(t)` in UV space.
    fn eval_dt(&self, t: f64) -> Point2;

    /// Returns `true` if the curve degenerates to a single UV point.
    fn is_degenerate(&self) -> bool;
}

// ── Line2 ─────────────────────────────────────────────────────────────────────

/// A straight line segment in UV parameter space.
///
/// Same parameterization contract as [`Line3`]: `eval(0) = p0`, `eval(1) = p1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line2 {
    pub p0: Point2,
    pub p1: Point2,
    pub t_min: f64,
    pub t_max: f64,
}

impl Line2 {
    /// Construct a line segment from `p0` to `p1` with the standard domain `[0, 1]`.
    pub fn new(p0: Point2, p1: Point2) -> Self {
        Self {
            p0,
            p1,
            t_min: 0.0,
            t_max: 1.0,
        }
    }
}

impl Curve2 for Line2 {
    fn eval(&self, t: f64) -> Point2 {
        self.p0 + (self.p1 - self.p0) * t
    }

    fn eval_dt(&self, _t: f64) -> Point2 {
        self.p1 - self.p0
    }

    fn is_degenerate(&self) -> bool {
        self.p0 == self.p1
    }
}

// ── CircularArc2 ──────────────────────────────────────────────────────────────

/// A circular arc (or full circle) in UV parameter space.
///
/// Parameterized by angle `t` in radians:
/// `eval(t) = center + Point2 { u: t.cos(), v: t.sin() } * radius`
///
/// A full circle has `t1 - t0 = 2π`. Used as the pcurve of circular edges on
/// flat (planar) cap faces where the circle lies in the surface's UV domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularArc2 {
    pub center: Point2,
    pub radius: f64,
    pub t0: f64,
    pub t1: f64,
}

impl CircularArc2 {
    /// Construct a circular arc in UV space.
    pub fn new(center: Point2, radius: f64, t0: f64, t1: f64) -> Self {
        Self {
            center,
            radius,
            t0,
            t1,
        }
    }
}

impl Curve2 for CircularArc2 {
    fn eval(&self, t: f64) -> Point2 {
        self.center + Point2::new(t.cos(), t.sin()) * self.radius
    }

    fn eval_dt(&self, t: f64) -> Point2 {
        Point2::new(-t.sin(), t.cos()) * self.radius
    }

    fn is_degenerate(&self) -> bool {
        self.radius == 0.0
    }
}

// ── Polyline2 ─────────────────────────────────────────────────────────────────

/// A piecewise-linear curve in UV space through `N` control points (`N - 1` segments).
///
/// # Parameterization
/// `t ∈ [0, N-1]`: segment `i` covers `t ∈ [i, i+1]`.
/// `eval(t)` for `t` outside `[0, N-1]` extrapolates along the first (t < 0) or
/// last (t > N-1) segment, consistent with the no-clamping contract shared by all
/// [`Curve2`] types.
///
/// # Panics
/// [`Polyline2::new`] panics if fewer than 2 points are provided.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline2 {
    pub points: Vec<Point2>,
}

impl Polyline2 {
    /// Construct a polyline from a sequence of UV control points.
    ///
    /// Panics if `points.len() < 2`.
    pub fn new(points: Vec<Point2>) -> Self {
        assert!(points.len() >= 2, "Polyline2 requires at least 2 points");
        Self { points }
    }

    /// Number of linear segments (`points.len() - 1`).
    pub fn n_segments(&self) -> usize {
        self.points.len() - 1
    }
}

impl Curve2 for Polyline2 {
    fn eval(&self, t: f64) -> Point2 {
        let n = self.n_segments();
        let i = (t.floor() as i64).clamp(0, n as i64 - 1) as usize;
        let frac = t - i as f64;
        let p0 = self.points[i];
        let p1 = self.points[i + 1];
        Point2::new(p0.u + (p1.u - p0.u) * frac, p0.v + (p1.v - p0.v) * frac)
    }

    /// Un-normalized tangent: the direction of the segment containing `t`.
    fn eval_dt(&self, t: f64) -> Point2 {
        let n = self.n_segments();
        let i = (t.floor() as i64).clamp(0, n as i64 - 1) as usize;
        let p0 = self.points[i];
        let p1 = self.points[i + 1];
        Point2::new(p1.u - p0.u, p1.v - p0.v)
    }

    fn is_degenerate(&self) -> bool {
        self.points.iter().all(|&p| p == self.points[0])
    }
}

// ── Stub type for future Curve2Kind variant ───────────────────────────────────

/// A rational B-spline curve in UV space. Fields TBD — stub for `Curve2Kind`.
#[derive(Debug, Clone)]
pub struct NurbsCurve2;

// ── Curve2Kind enum ───────────────────────────────────────────────────────────

/// The concrete stored pcurve type used in the B-rep arena.
#[derive(Debug, Clone)]
pub enum Curve2Kind {
    Line2(Line2),
    CircularArc2(CircularArc2),
    Polyline2(Polyline2),
    Nurbs(NurbsCurve2),
}

impl Curve2 for Curve2Kind {
    fn eval(&self, t: f64) -> Point2 {
        match self {
            Curve2Kind::Line2(l) => l.eval(t),
            Curve2Kind::CircularArc2(a) => a.eval(t),
            Curve2Kind::Polyline2(p) => p.eval(t),
            Curve2Kind::Nurbs(_) => todo!("NurbsCurve2::eval"),
        }
    }

    fn eval_dt(&self, t: f64) -> Point2 {
        match self {
            Curve2Kind::Line2(l) => l.eval_dt(t),
            Curve2Kind::CircularArc2(a) => a.eval_dt(t),
            Curve2Kind::Polyline2(p) => p.eval_dt(t),
            Curve2Kind::Nurbs(_) => todo!("NurbsCurve2::eval_dt"),
        }
    }

    fn is_degenerate(&self) -> bool {
        match self {
            Curve2Kind::Line2(l) => l.is_degenerate(),
            Curve2Kind::CircularArc2(a) => a.is_degenerate(),
            Curve2Kind::Polyline2(p) => p.is_degenerate(),
            Curve2Kind::Nurbs(_) => todo!("NurbsCurve2::is_degenerate"),
        }
    }
}

impl Curve2Kind {
    /// The end-point of this curve at its upper t-bound (used for segment chaining).
    pub fn end(&self) -> Point2 {
        match self {
            Curve2Kind::Line2(l) => l.p1,
            Curve2Kind::CircularArc2(a) => a.eval(a.t1),
            Curve2Kind::Polyline2(pl) => *pl.points.last().expect("Polyline2 has points"),
            Curve2Kind::Nurbs(_) => todo!("NurbsCurve2::end"),
        }
    }
}

// ── Surface trait ─────────────────────────────────────────────────────────────

/// Evaluation interface for a parameterized surface in 3-D space.
///
/// The surface maps `(u, v)` to a point in R³. Derivative methods return un-normalized
/// tangent vectors; `eval_n` returns the normalized outward normal and is `None` at
/// geometric singularities (e.g. the apex of a cone).
pub trait Surface {
    /// Evaluate the 3-D point at parameter `(u, v)`.
    fn eval(&self, u: f64, v: f64) -> Point3;

    /// Evaluate the un-normalized u-tangent `∂/∂u eval(u, v)`.
    fn eval_du(&self, u: f64, v: f64) -> Point3;

    /// Evaluate the un-normalized v-tangent `∂/∂v eval(u, v)`.
    fn eval_dv(&self, u: f64, v: f64) -> Point3;

    /// Evaluate the normalized surface normal at `(u, v)`.
    ///
    /// Returns `None` at geometric singularities where the normal is undefined (e.g. the
    /// apex of a cone). See `brep_notes.md` for details.
    fn eval_n(&self, u: f64, v: f64) -> Option<Point3>;
}

// ── Plane ─────────────────────────────────────────────────────────────────────

/// An infinite planar surface.
///
/// The parameterization is: `eval(u, v) = p0 + u * u_dir + v * v_dir`. The outward
/// normal is `normalize(u_dir × v_dir)` (right-hand rule, constant everywhere).
///
/// The caller is responsible for ensuring `u_dir` and `v_dir` are unit vectors and
/// mutually perpendicular. The domain is defined by the B-rep trimming loop, not by
/// this struct.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub p0: Point3,
    pub u_dir: Point3,
    pub v_dir: Point3,
}

impl Plane {
    /// Construct a plane from an origin and two (caller-normalised, perpendicular) basis
    /// vectors.
    pub fn new(p0: Point3, u_dir: Point3, v_dir: Point3) -> Self {
        Self { p0, u_dir, v_dir }
    }
}

impl Surface for Plane {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        self.p0 + self.u_dir * u + self.v_dir * v
    }

    /// u-tangent is constant: `u_dir`. Parameters are unused.
    fn eval_du(&self, _u: f64, _v: f64) -> Point3 {
        self.u_dir
    }

    /// v-tangent is constant: `v_dir`. Parameters are unused.
    fn eval_dv(&self, _u: f64, _v: f64) -> Point3 {
        self.v_dir
    }

    /// Normal is constant: `normalize(u_dir × v_dir)`. Always `Some`.
    fn eval_n(&self, _u: f64, _v: f64) -> Option<Point3> {
        Some(self.u_dir.cross(self.v_dir).normalize())
    }
}

// ── CylindricalSurface ────────────────────────────────────────────────────────

/// An infinite cylindrical surface.
///
/// Parameterization: `u = angle ∈ [0, 2π)`, `v = height along axis`.
/// Frame in the cross-section plane: `ref_dir` at `u = 0`, `axis × ref_dir` at `u = π/2`.
/// The outward normal at `(u, v)` is `cos(u)*ref_dir + sin(u)*(axis × ref_dir)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylindricalSurface {
    pub origin: Point3,  // center of the base circle; eval(u, 0) lies on this circle
    pub axis: Point3,    // unit; direction of increasing v
    pub ref_dir: Point3, // unit, ⊥ axis; u = 0 meridian
    pub radius: f64,
}

impl CylindricalSurface {
    pub fn new(origin: Point3, axis: Point3, ref_dir: Point3, radius: f64) -> Self {
        Self {
            origin,
            axis,
            ref_dir,
            radius,
        }
    }
}

impl Surface for CylindricalSurface {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        let r_hat = self.ref_dir * u.cos() + e2 * u.sin();
        self.origin + self.axis * v + r_hat * self.radius
    }

    fn eval_du(&self, u: f64, _v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        (self.ref_dir * (-u.sin()) + e2 * u.cos()) * self.radius
    }

    fn eval_dv(&self, _u: f64, _v: f64) -> Point3 {
        self.axis
    }

    /// The outward radial unit vector `r̂(u)`. Never `None`.
    fn eval_n(&self, u: f64, _v: f64) -> Option<Point3> {
        let e2 = self.axis.cross(self.ref_dir);
        Some(self.ref_dir * u.cos() + e2 * u.sin())
    }
}

// ── ConicalSurface ────────────────────────────────────────────────────────────

/// A conical surface with a singular apex.
///
/// Parameterization: `u = angle ∈ [0, 2π)`, `v = slant distance from apex` (not axial
/// height). `eval_n` returns `None` at `v = 0` (the apex).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConicalSurface {
    pub apex: Point3,    // singular point; eval(u, 0) = apex for all u
    pub axis: Point3,    // unit; direction of increasing v (toward base)
    pub ref_dir: Point3, // unit, ⊥ axis; u = 0 meridian
    pub half_angle: f64, // 0 < half_angle < π/2 (radians)
}

impl ConicalSurface {
    pub fn new(apex: Point3, axis: Point3, ref_dir: Point3, half_angle: f64) -> Self {
        Self {
            apex,
            axis,
            ref_dir,
            half_angle,
        }
    }
}

impl Surface for ConicalSurface {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        let r_hat = self.ref_dir * u.cos() + e2 * u.sin();
        let slant = self.axis * self.half_angle.cos() + r_hat * self.half_angle.sin();
        self.apex + slant * v
    }

    fn eval_du(&self, u: f64, v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        let dr_hat = self.ref_dir * (-u.sin()) + e2 * u.cos();
        dr_hat * (v * self.half_angle.sin())
    }

    fn eval_dv(&self, u: f64, _v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        let r_hat = self.ref_dir * u.cos() + e2 * u.sin();
        self.axis * self.half_angle.cos() + r_hat * self.half_angle.sin()
    }

    /// Returns `None` at `v = 0` (the apex). Otherwise returns the outward unit normal
    /// `cos(ha)*r̂(u) - sin(ha)*axis`, which is perpendicular to the slant direction and
    /// has unit length.
    fn eval_n(&self, u: f64, v: f64) -> Option<Point3> {
        if v == 0.0 {
            return None;
        }
        let e2 = self.axis.cross(self.ref_dir);
        let r_hat = self.ref_dir * u.cos() + e2 * u.sin();
        Some(r_hat * self.half_angle.cos() - self.axis * self.half_angle.sin())
    }
}

// ── SphericalSurface ──────────────────────────────────────────────────────────

/// A spherical surface.
///
/// Parameterization: `u = longitude ∈ [0, 2π)`, `v = latitude ∈ [-π/2, +π/2]`.
/// The outward normal equals the unit radial direction and is well-defined everywhere,
/// including the poles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphericalSurface {
    pub center: Point3,
    pub radius: f64,
    pub ref_dir: Point3, // unit; u = 0, v = 0 reference direction
    pub axis: Point3,    // unit, ⊥ ref_dir; north pole at v = +π/2
}

impl SphericalSurface {
    pub fn new(center: Point3, radius: f64, ref_dir: Point3, axis: Point3) -> Self {
        Self {
            center,
            radius,
            ref_dir,
            axis,
        }
    }
}

impl Surface for SphericalSurface {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        let r_hat = self.ref_dir * u.cos() + e2 * u.sin();
        self.center + (r_hat * v.cos() + self.axis * v.sin()) * self.radius
    }

    fn eval_du(&self, u: f64, v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        let dr_hat = self.ref_dir * (-u.sin()) + e2 * u.cos();
        dr_hat * (v.cos() * self.radius)
    }

    fn eval_dv(&self, u: f64, v: f64) -> Point3 {
        let e2 = self.axis.cross(self.ref_dir);
        let r_hat = self.ref_dir * u.cos() + e2 * u.sin();
        (r_hat * (-v.sin()) + self.axis * v.cos()) * self.radius
    }

    /// The outward unit normal `cos(v)*r̂(u) + sin(v)*axis`. Always `Some`.
    fn eval_n(&self, u: f64, v: f64) -> Option<Point3> {
        let e2 = self.axis.cross(self.ref_dir);
        let r_hat = self.ref_dir * u.cos() + e2 * u.sin();
        Some(r_hat * v.cos() + self.axis * v.sin())
    }
}

// ── LinearExtrusionSurface ────────────────────────────────────────────────────

/// A surface formed by sweeping a profile curve linearly along a direction vector.
///
/// Parameterization: `u` follows the profile, `v` is world-space distance along
/// `direction`.
///
/// `S(u, v) = profile.eval(u) + direction * v`
///
/// This is consistent with `CylindricalSurface`: a cylinder is this type with a
/// `CircularArc3` profile and `direction = +Z`, giving `u = angle`, `v = height`.
#[derive(Debug, Clone)]
pub struct LinearExtrusionSurface {
    pub profile: Curve3Kind, // generatrix curve; lies in the base plane
    pub direction: Point3,   // unit extrusion direction vector
}

impl LinearExtrusionSurface {
    pub fn new(profile: Curve3Kind, direction: Point3) -> Self {
        Self { profile, direction }
    }
}

impl Surface for LinearExtrusionSurface {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        self.profile.eval(u) + self.direction * v
    }

    fn eval_du(&self, u: f64, _v: f64) -> Point3 {
        self.profile.eval_dt(u)
    }

    fn eval_dv(&self, _u: f64, _v: f64) -> Point3 {
        self.direction
    }

    /// Cross product of the profile tangent and the extrusion direction, normalised.
    /// Returns `None` when the profile tangent is zero (degenerate point on profile).
    fn eval_n(&self, u: f64, v: f64) -> Option<Point3> {
        let du = self.eval_du(u, v);
        let dv = self.direction;
        let n = du.cross(dv);
        if n.length() == 0.0 {
            None
        } else {
            Some(n.normalize())
        }
    }
}

// ── RevolutionSurface ─────────────────────────────────────────────────────────

/// A surface formed by rotating a profile curve around an axis.
///
/// Parameterization: `u = angle ∈ [0, 2π)` (rotation), `v` follows the profile.
///
/// `S(u, v)` = `profile.eval(v)` rotated around `axis_dir` through `axis_origin`
/// by angle `u`, via Rodrigues' formula.
///
/// This is consistent with `CylindricalSurface` (`u = angle`, `v = height`),
/// `ConicalSurface` (`u = angle`, `v = slant distance`), and `SphericalSurface`
/// (`u = longitude`, `v = latitude`).
#[derive(Debug, Clone)]
pub struct RevolutionSurface {
    pub profile: Curve3Kind, // generatrix in the meridional half-plane
    pub axis_origin: Point3, // any point on the revolution axis
    pub axis_dir: Point3,    // unit direction of axis
}

impl RevolutionSurface {
    pub fn new(profile: Curve3Kind, axis_origin: Point3, axis_dir: Point3) -> Self {
        Self {
            profile,
            axis_origin,
            axis_dir,
        }
    }

    /// Rotate point `p` around the axis by angle `u` (Rodrigues' formula).
    fn rotate(&self, p: Point3, u: f64) -> Point3 {
        // translate to axis frame, rotate, translate back
        let q = p - self.axis_origin;
        let a = self.axis_dir;
        let q_rot = q * u.cos() + a.cross(q) * u.sin() + a * a.dot(q) * (1.0 - u.cos());
        self.axis_origin + q_rot
    }
}

impl Surface for RevolutionSurface {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        self.rotate(self.profile.eval(v), u)
    }

    /// Tangent along the sweep direction: `cross(axis_dir, radial) * |radial|` rotated.
    /// Zero (and `eval_n` returns `None`) when the profile point lies on the axis.
    fn eval_du(&self, u: f64, v: f64) -> Point3 {
        // d/du of Rodrigues = cross(axis, q)*cos(u) - q*sin(u) + axis*dot(axis,q)*sin(u) … simplifies to:
        // d/du rotate(q, u) = cross(axis, rotate(q, u) - axis_origin)
        let rotated = self.eval(u, v);
        self.axis_dir.cross(rotated - self.axis_origin)
    }

    /// Tangent along the profile, rotated by `u`.
    fn eval_dv(&self, u: f64, v: f64) -> Point3 {
        self.rotate(self.profile.eval_dt(v), u)
    }

    /// Returns `None` when the profile point is on the axis (`eval_du` is zero).
    fn eval_n(&self, u: f64, v: f64) -> Option<Point3> {
        let du = self.eval_du(u, v);
        let dv = self.eval_dv(u, v);
        let n = du.cross(dv);
        if n.length() == 0.0 {
            None
        } else {
            Some(n.normalize())
        }
    }
}

// ── PathError ────────────────────────────────────────────────────────────────

/// Errors from building or finishing a [`Path2D`].
///
/// Builder-time variants are returned by the fallible builder methods
/// ([`Path2D::start_contour`], [`Path2D::close`], [`Path2D::line_to_close`],
/// [`Path2D::finish`]) — the infallible appends (`line_to`/`arc_to`/etc.) panic on
/// programmer error (no open contour) per the pragmatic fallibility split (0-b.Q4).
/// Compile-time variants (`WindingRoleMismatch`, `HoleOutsideOuter`) are produced by
/// the extrusion compiler, not the builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    // ── Builder-time (returned by Result-returning methods) ──────────────────
    /// A builder method that needs an open contour was called with none open
    /// (e.g. `close()` before any `start_contour`).
    NoOpenContour,
    /// `start_contour` was called while the previous contour was still open
    /// (non-empty + not closed), or `finish` was called with a contour still open.
    UnclosedContour,
    /// A contour has no segments (opened with `start_contour` but nothing appended).
    EmptyContour,
    /// A segment degenerates to a single point (e.g. a zero-length `Line2`).
    DegenerateSegment,
    /// A contour has zero signed area (e.g. a back-and-forth line that cancels out).
    ///
    /// This is the build-time winding check (0-b.Q1): each contour must have a
    /// definite, nonzero winding direction. The *role* check (CCW outer / CW hole)
    /// is compile-time, not here.
    ZeroAreaContour,
    /// `close()` was called when `current_pos != start` (bit-exact invariant).
    CloseNotAtStart,

    // ── Compile-time (returned by build_extrusion, not the builder) ───────────
    /// A CW (hole) contour is not inside any CCW (outer) contour.
    HoleOutsideOuter,
    /// A contour's winding direction doesn't match its nesting role
    /// (CCW not a top-level outer, or CW not inside an outer).
    WindingRoleMismatch,
    /// Self-intersection within or between contours (future — needs tolerance).
    SelfIntersection,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::NoOpenContour => write!(f, "no open contour; call start_contour first"),
            PathError::UnclosedContour => write!(
                f,
                "previous contour is still open; call close() before starting a new contour or finishing"
            ),
            PathError::EmptyContour => write!(f, "contour has no segments"),
            PathError::DegenerateSegment => write!(f, "degenerate segment (zero length)"),
            PathError::ZeroAreaContour => {
                write!(f, "contour has zero signed area (degenerate winding)")
            }
            PathError::CloseNotAtStart => write!(
                f,
                "close() called when current_pos != start (bit-exact invariant violated)"
            ),
            PathError::HoleOutsideOuter => {
                write!(f, "CW (hole) contour is not inside any CCW (outer) contour")
            }
            PathError::WindingRoleMismatch => write!(
                f,
                "contour winding direction does not match its nesting role"
            ),
            PathError::SelfIntersection => write!(f, "self-intersection (not yet implemented)"),
        }
    }
}

impl std::error::Error for PathError {}

// ── Contour ───────────────────────────────────────────────────────────────────

/// One closed loop of 2-D segments — the element type of a [`Path2D`] contour set.
///
/// Mirrors `Solid`/`EdgeLoop` in the b-rep: one connected closed piece. A `Contour`
/// with positive signed area is an *outer* (CCW); negative is a *hole* (CW). The
/// role is determined at compile (nesting) time, not here — the builder only enforces
/// that each contour has a *definite, nonzero* winding (see [`Path2D::finish`]).
///
/// Adjacent segments share endpoints implicitly: the start of segment `i+1` is the
/// end of segment `i`, and the start of segment `0` is [`Contour::start`].
#[derive(Debug, Clone)]
pub struct Contour {
    /// The start point (also the expected end point of a closed contour).
    pub start: Point2,
    /// Ordered segments; segment `i` runs from the end of `i-1` to `segments[i].end()`.
    pub segments: Vec<Curve2Kind>,
    /// Whether [`close`](Path2D::close) has marked this contour topologically closed.
    pub closed: bool,
    /// Current end of the last segment (== `start` when empty). Private so callers
    /// can't desynchronize it from `segments`.
    current_pos: Point2,
}

impl Contour {
    /// Create a new empty contour beginning at `start`.
    fn new(start: Point2) -> Self {
        Self {
            start,
            segments: Vec::new(),
            closed: false,
            current_pos: start,
        }
    }

    /// The end-point of the last segment, or `start` if no segments have been added.
    pub fn current_pos(&self) -> Point2 {
        self.current_pos
    }

    /// Push a segment, advancing `current_pos` to the segment's end.
    fn push(&mut self, seg: Curve2Kind, end: Point2) {
        self.segments.push(seg);
        self.current_pos = end;
    }
}

// ── Path2D ────────────────────────────────────────────────────────────────────

/// A 2-D contour set: one or more [`Contour`]s expressing a profile with holes
/// and/or disconnected pieces.
///
/// This is the 2-D analogue of a `SolidSet`: a top-level collection of connected
/// pieces, each of which may carry sub-pieces that represent holes. The element name
/// (`Contour` ↔ `Solid`) mirrors the b-rep; the outer name (`Path2D`, role-named)
/// describes the author's role ("building a 2D path"), not the implementation.
///
/// Built with a canvas-style API: [`Path2D::new`] creates the empty set,
/// [`Path2D::start_contour`] opens a contour (errors if the previous is unclosed),
/// `line_to`/`arc_to`/`quad_to`/`cubic_to` extend the current contour (infallible
/// appends), [`Path2D::close`] / [`Path2D::line_to_close`] close it, and
/// [`Path2D::finish`] consumes the builder and runs validation.
#[derive(Debug, Clone)]
pub struct Path2D {
    contours: Vec<Contour>,
}

impl Path2D {
    /// Create an empty contour set. The first [`start_contour`](Self::start_contour)
    /// opens contour 0.
    pub fn new() -> Self {
        Self {
            contours: Vec::new(),
        }
    }

    /// Number of contours in the set.
    pub fn n_contours(&self) -> usize {
        self.contours.len()
    }

    /// Read-only access to contour `i`.
    pub fn contour(&self, i: usize) -> &Contour {
        &self.contours[i]
    }

    /// Read-only access to all contours.
    pub fn contours(&self) -> &[Contour] {
        &self.contours
    }

    /// The end-point of the last segment of the current (last) contour, or that
    /// contour's `start` if it has no segments. Returns `None` if the set is empty.
    pub fn current_pos(&self) -> Option<Point2> {
        self.contours.last().map(|c| c.current_pos())
    }

    /// A reference to the currently-open contour (the last one), if any.
    fn current_contour(&mut self) -> Option<&mut Contour> {
        self.contours.last_mut()
    }

    /// Open a new contour at `start`. **Strict, not silent:** returns
    /// [`PathError::UnclosedContour`] if the previous contour is still open
    /// (non-empty + not closed), catching the "forgot to close" omission at the
    /// call site where it's made.
    pub fn start_contour(&mut self, start: Point2) -> Result<&mut Self, PathError> {
        if let Some(c) = self.contours.last() {
            if !c.segments.is_empty() && !c.closed {
                return Err(PathError::UnclosedContour);
            }
        }
        self.contours.push(Contour::new(start));
        Ok(self)
    }

    /// Append a straight segment from the current position to `end`. Infallible
    /// (panics with `"no open contour"` if no contour is open — programmer error).
    pub fn line_to(&mut self, end: Point2) -> &mut Self {
        let c = self
            .current_contour()
            .expect("no open contour; call start_contour first");
        let seg = Curve2Kind::Line2(Line2::new(c.current_pos(), end));
        c.push(seg, end);
        self
    }

    /// Append a circular arc from the current position, sweeping `sweep` radians
    /// around `center`. Positive `sweep` is CCW; negative is CW. The end point is
    /// derived geometrically. Infallible (panics if no contour is open).
    pub fn arc_to(&mut self, center: Point2, sweep: f64) -> &mut Self {
        let c = self
            .current_contour()
            .expect("no open contour; call start_contour first");
        let p0 = c.current_pos();
        let r = p0 - center;
        let radius = ((r.u * r.u) + (r.v * r.v)).sqrt();
        let t0 = r.v.atan2(r.u);
        let t1 = t0 + sweep;
        let end = Point2::new(center.u + radius * t1.cos(), center.v + radius * t1.sin());
        c.push(
            Curve2Kind::CircularArc2(CircularArc2::new(center, radius, t0, t1)),
            end,
        );
        self
    }

    /// Mark the current contour as closed. No segment is added; the caller asserts
    /// that `current_pos` is already bit-exactly at the contour's `start`. Returns
    /// [`PathError::CloseNotAtStart`] if not, or [`PathError::NoOpenContour`] if no
    /// contour is open.
    pub fn close(&mut self) -> Result<&mut Self, PathError> {
        let c = self.current_contour().ok_or(PathError::NoOpenContour)?;
        if c.current_pos() != c.start {
            return Err(PathError::CloseNotAtStart);
        }
        c.closed = true;
        Ok(self)
    }

    /// Add a straight closing segment from `current_pos` to the contour's `start`,
    /// then mark the contour closed. Same error variants as [`close`](Self::close).
    pub fn line_to_close(&mut self) -> Result<&mut Self, PathError> {
        // Take start by value first to avoid borrowing conflict across line_to.
        let start = {
            let c = self.current_contour().ok_or(PathError::NoOpenContour)?;
            c.start
        };
        self.line_to(start);
        // line_to just set current_pos == start (bit-exact), so close can't fail on CloseNotAtStart.
        self.close()
    }

    /// Consume the builder, validate, and return the frozen [`Path2D`].
    ///
    /// Build-time validation (tolerance-free, exact):
    /// * every contour non-empty → [`PathError::EmptyContour`],
    /// * every contour closed (no open contour left) → [`PathError::UnclosedContour`],
    /// * every closed contour has `current_pos == start` (bit-exact) → [`PathError::CloseNotAtStart`],
    /// * no degenerate segments → [`PathError::DegenerateSegment`],
    /// * each contour has **nonzero signed area** → [`PathError::ZeroAreaContour`].
    ///
    /// The CCW-outer / CW-hole *role* check is **not** here — it needs nesting
    /// (depends on the other contours) and runs at compile time in `build_extrusion`.
    pub fn finish(mut self) -> Result<Path2D, PathError> {
        for c in &mut self.contours {
            if c.segments.is_empty() {
                return Err(PathError::EmptyContour);
            }
            if !c.closed {
                return Err(PathError::UnclosedContour);
            }
            if c.current_pos() != c.start {
                return Err(PathError::CloseNotAtStart);
            }
            for seg in &c.segments {
                if seg.is_degenerate() {
                    return Err(PathError::DegenerateSegment);
                }
            }
            if contour_signed_area(c).abs() == 0.0 {
                return Err(PathError::ZeroAreaContour);
            }
        }
        Ok(self)
    }
}

impl Default for Path2D {
    fn default() -> Self {
        Self::new()
    }
}

/// Exact signed area of a closed contour (polygon + arc/bezier contributions).
///
/// Positive = CCW (outer), negative = CW (hole). Used by [`Path2D::finish`] to reject
/// zero-area (degenerate-winding) contours. Exact for non-degenerate polygons; for
/// arcs/beziers the contribution is the area swept by the chord (the linear term),
/// which is exact for the signed-area/winding-sign purpose.
fn contour_signed_area(c: &Contour) -> f64 {
    let mut area = 0.0;
    let mut p = c.start;
    for seg in &c.segments {
        let q = seg.end();
        // Shoelace cross term: p × q (the z-component of the 3-D cross).
        area += p.u * q.v - p.v * q.u;
        p = q;
    }
    area * 0.5
}

// ── Display for Path2D ────────────────────────────────────────────────────────

fn fmt_f64_path(v: f64) -> String {
    let rounded = (v * 1e4).round() / 1e4;
    let mut buf = ryu::Buffer::new();
    buf.format(rounded).to_owned()
}

impl std::fmt::Display for Path2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "path(contours={}, closed=all)", self.contours.len())?;
        // Per-contour breakdown
        for (ci, c) in self.contours.iter().enumerate() {
            write!(
                f,
                "\n[contour {}] start=({}, {}) segs={} closed={}",
                ci,
                fmt_f64_path(c.start.u),
                fmt_f64_path(c.start.v),
                c.segments.len(),
                c.closed,
            )?;
            let n = c.segments.len();
            for (i, seg) in c.segments.iter().enumerate() {
                let connector = if i + 1 == n {
                    "\n  └── "
                } else {
                    "\n  ├── "
                };
                let seg_str = match seg {
                    Curve2Kind::Line2(l) => format!(
                        "line_to({}, {})",
                        fmt_f64_path(l.p1.u),
                        fmt_f64_path(l.p1.v)
                    ),
                    Curve2Kind::CircularArc2(a) => {
                        let sweep = a.t1 - a.t0;
                        format!(
                            "arc_to(center=({}, {}), r={}, sweep={})",
                            fmt_f64_path(a.center.u),
                            fmt_f64_path(a.center.v),
                            fmt_f64_path(a.radius),
                            fmt_f64_path(sweep),
                        )
                    }
                    Curve2Kind::Polyline2(pl) => format!("polyline({} pts)", pl.points.len()),
                    Curve2Kind::Nurbs(_) => "nurbs(...)".to_owned(),
                };
                write!(f, "{}{}", connector, seg_str)?;
            }
        }
        Ok(())
    }
}

// ── Stub type for remaining SurfaceKind variant ───────────────────────────────

/// A rational B-spline surface. Fields TBD — stub for `SurfaceKind`.
#[derive(Debug, Clone)]
pub struct NurbsSurf;

// ── SurfaceKind enum ──────────────────────────────────────────────────────────

/// The concrete stored surface type used in the B-rep arena.
pub enum SurfaceKind {
    Plane(Plane),
    Cylinder(CylindricalSurface),
    Cone(ConicalSurface),
    Sphere(SphericalSurface),
    Extrusion(LinearExtrusionSurface),
    Revolution(RevolutionSurface),
    Nurbs(NurbsSurf),
}

impl Surface for SurfaceKind {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval(u, v),
            SurfaceKind::Cylinder(c) => c.eval(u, v),
            SurfaceKind::Cone(c) => c.eval(u, v),
            SurfaceKind::Sphere(s) => s.eval(u, v),
            SurfaceKind::Extrusion(e) => e.eval(u, v),
            SurfaceKind::Revolution(r) => r.eval(u, v),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval"),
        }
    }

    fn eval_du(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval_du(u, v),
            SurfaceKind::Cylinder(c) => c.eval_du(u, v),
            SurfaceKind::Cone(c) => c.eval_du(u, v),
            SurfaceKind::Sphere(s) => s.eval_du(u, v),
            SurfaceKind::Extrusion(e) => e.eval_du(u, v),
            SurfaceKind::Revolution(r) => r.eval_du(u, v),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_du"),
        }
    }

    fn eval_dv(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval_dv(u, v),
            SurfaceKind::Cylinder(c) => c.eval_dv(u, v),
            SurfaceKind::Cone(c) => c.eval_dv(u, v),
            SurfaceKind::Sphere(s) => s.eval_dv(u, v),
            SurfaceKind::Extrusion(e) => e.eval_dv(u, v),
            SurfaceKind::Revolution(r) => r.eval_dv(u, v),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_dv"),
        }
    }

    fn eval_n(&self, u: f64, v: f64) -> Option<Point3> {
        match self {
            SurfaceKind::Plane(p) => p.eval_n(u, v),
            SurfaceKind::Cylinder(c) => c.eval_n(u, v),
            SurfaceKind::Cone(c) => c.eval_n(u, v),
            SurfaceKind::Sphere(s) => s.eval_n(u, v),
            SurfaceKind::Extrusion(e) => e.eval_n(u, v),
            SurfaceKind::Revolution(r) => r.eval_n(u, v),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_n"),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;

    // helpers
    fn p(x: f64, y: f64, z: f64) -> Point3 {
        Point3::new(x, y, z)
    }
    fn uv(u: f64, v: f64) -> Point2 {
        Point2::new(u, v)
    }
    fn approx_eq3(a: Point3, b: Point3) -> bool {
        (a - b).length() < 1e-14
    }

    // ── Point3 ────────────────────────────────────────────────────────────────

    #[test]
    fn point3_new() {
        let pt = p(1.0, 2.0, 3.0);
        assert_eq!(pt.x, 1.0);
        assert_eq!(pt.y, 2.0);
        assert_eq!(pt.z, 3.0);
    }

    #[test]
    fn point3_add() {
        assert_eq!(p(1.0, 2.0, 3.0) + p(4.0, 5.0, 6.0), p(5.0, 7.0, 9.0));
    }

    #[test]
    fn point3_sub() {
        assert_eq!(p(4.0, 5.0, 6.0) - p(1.0, 2.0, 3.0), p(3.0, 3.0, 3.0));
    }

    #[test]
    fn point3_mul() {
        assert_eq!(p(1.0, 2.0, 3.0) * 2.0, p(2.0, 4.0, 6.0));
    }

    #[test]
    fn point3_length() {
        assert_eq!(p(3.0, 4.0, 0.0).length(), 5.0);
        assert_eq!(p(0.0, 0.0, 0.0).length(), 0.0);
    }

    #[test]
    fn point3_normalize_axis_aligned() {
        assert_eq!(p(3.0, 0.0, 0.0).normalize(), p(1.0, 0.0, 0.0));
        assert_eq!(p(0.0, -5.0, 0.0).normalize(), p(0.0, -1.0, 0.0));
    }

    #[test]
    fn point3_normalize_unit_length() {
        let n = p(1.0, 2.0, 3.0).normalize();
        assert!((n.length() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn point3_cross_basis_vectors() {
        assert_eq!(p(1.0, 0.0, 0.0).cross(p(0.0, 1.0, 0.0)), p(0.0, 0.0, 1.0));
        assert_eq!(p(0.0, 1.0, 0.0).cross(p(0.0, 0.0, 1.0)), p(1.0, 0.0, 0.0));
        assert_eq!(p(0.0, 0.0, 1.0).cross(p(1.0, 0.0, 0.0)), p(0.0, 1.0, 0.0));
    }

    #[test]
    fn point3_cross_anticommutative() {
        let a = p(1.0, 2.0, 3.0);
        let b = p(4.0, 5.0, 6.0);
        assert_eq!(a.cross(b), p(-3.0, 6.0, -3.0));
        assert_eq!(b.cross(a), p(3.0, -6.0, 3.0));
    }

    // ── Point2 ────────────────────────────────────────────────────────────────

    #[test]
    fn point2_new() {
        let pt = uv(3.0, 4.0);
        assert_eq!(pt.u, 3.0);
        assert_eq!(pt.v, 4.0);
    }

    #[test]
    fn point2_add() {
        assert_eq!(uv(1.0, 2.0) + uv(3.0, 4.0), uv(4.0, 6.0));
    }

    #[test]
    fn point2_sub() {
        assert_eq!(uv(5.0, 3.0) - uv(1.0, 2.0), uv(4.0, 1.0));
    }

    #[test]
    fn point2_mul() {
        assert_eq!(uv(2.0, 3.0) * 4.0, uv(8.0, 12.0));
    }

    // ── Line3 construction ────────────────────────────────────────────────────

    #[test]
    fn line3_new_stores_endpoints() {
        let l = Line3::new(p(0.0, 0.0, 0.0), p(1.0, 2.0, 3.0));
        assert_eq!(l.p0, p(0.0, 0.0, 0.0));
        assert_eq!(l.p1, p(1.0, 2.0, 3.0));
        assert_eq!(l.t_min, 0.0);
        assert_eq!(l.t_max, 1.0);
    }

    // ── Line3::eval ───────────────────────────────────────────────────────────

    #[test]
    fn line3_eval_at_t0() {
        let l = Line3::new(p(1.0, 2.0, 3.0), p(4.0, 6.0, 8.0));
        assert_eq!(l.eval(0.0), p(1.0, 2.0, 3.0));
    }

    #[test]
    fn line3_eval_at_t1() {
        let l = Line3::new(p(1.0, 2.0, 3.0), p(4.0, 6.0, 8.0));
        assert_eq!(l.eval(1.0), p(4.0, 6.0, 8.0));
    }

    #[test]
    fn line3_eval_midpoint() {
        let l = Line3::new(p(0.0, 0.0, 0.0), p(2.0, 4.0, 6.0));
        assert_eq!(l.eval(0.5), p(1.0, 2.0, 3.0));
    }

    #[test]
    fn line3_eval_extrapolate() {
        let l = Line3::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0));
        assert_eq!(l.eval(2.0), p(2.0, 0.0, 0.0));
        assert_eq!(l.eval(-1.0), p(-1.0, 0.0, 0.0));
    }

    // ── Line3::eval_dt ────────────────────────────────────────────────────────

    #[test]
    fn line3_eval_dt_constant() {
        let l = Line3::new(p(0.0, 0.0, 0.0), p(3.0, 0.0, 4.0));
        let expected = p(3.0, 0.0, 4.0);
        assert_eq!(l.eval_dt(0.0), expected);
        assert_eq!(l.eval_dt(0.5), expected);
        assert_eq!(l.eval_dt(1.0), expected);
    }

    #[test]
    fn line3_eval_dt_zero_length() {
        let l = Line3::new(p(1.0, 1.0, 1.0), p(1.0, 1.0, 1.0));
        assert_eq!(l.eval_dt(0.0), p(0.0, 0.0, 0.0));
    }

    // ── Line3::is_degenerate ──────────────────────────────────────────────────

    #[test]
    fn line3_not_degenerate() {
        assert!(!Line3::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)).is_degenerate());
    }

    #[test]
    fn line3_is_degenerate() {
        assert!(Line3::new(p(1.0, 2.0, 3.0), p(1.0, 2.0, 3.0)).is_degenerate());
    }

    // ── Curve3Kind delegation ─────────────────────────────────────────────────

    #[test]
    fn curve3kind_line3_eval() {
        let l = Line3::new(p(0.0, 0.0, 0.0), p(2.0, 4.0, 6.0));
        let ck = Curve3Kind::Line3(l);
        assert_eq!(ck.eval(0.0), l.eval(0.0));
        assert_eq!(ck.eval(0.5), l.eval(0.5));
        assert_eq!(ck.eval(1.0), l.eval(1.0));
    }

    #[test]
    fn curve3kind_line3_eval_dt() {
        let l = Line3::new(p(0.0, 0.0, 0.0), p(1.0, 2.0, 3.0));
        let ck = Curve3Kind::Line3(l);
        assert_eq!(ck.eval_dt(0.0), l.eval_dt(0.0));
        assert_eq!(ck.eval_dt(0.5), l.eval_dt(0.5));
    }

    #[test]
    fn curve3kind_line3_is_degenerate() {
        let nd = Curve3Kind::Line3(Line3::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)));
        let dg = Curve3Kind::Line3(Line3::new(p(1.0, 1.0, 1.0), p(1.0, 1.0, 1.0)));
        assert!(!nd.is_degenerate());
        assert!(dg.is_degenerate());
    }

    // ── Line2 construction ────────────────────────────────────────────────────

    #[test]
    fn line2_new_stores_endpoints() {
        let l = Line2::new(uv(0.0, 0.0), uv(1.0, 2.0));
        assert_eq!(l.p0, uv(0.0, 0.0));
        assert_eq!(l.p1, uv(1.0, 2.0));
        assert_eq!(l.t_min, 0.0);
        assert_eq!(l.t_max, 1.0);
    }

    // ── Line2::eval ───────────────────────────────────────────────────────────

    #[test]
    fn line2_eval_at_t0() {
        let l = Line2::new(uv(1.0, 2.0), uv(3.0, 5.0));
        assert_eq!(l.eval(0.0), uv(1.0, 2.0));
    }

    #[test]
    fn line2_eval_at_t1() {
        let l = Line2::new(uv(1.0, 2.0), uv(3.0, 5.0));
        assert_eq!(l.eval(1.0), uv(3.0, 5.0));
    }

    #[test]
    fn line2_eval_midpoint() {
        let l = Line2::new(uv(0.0, 0.0), uv(2.0, 4.0));
        assert_eq!(l.eval(0.5), uv(1.0, 2.0));
    }

    #[test]
    fn line2_eval_extrapolate() {
        let l = Line2::new(uv(0.0, 0.0), uv(1.0, 0.0));
        assert_eq!(l.eval(2.0), uv(2.0, 0.0));
        assert_eq!(l.eval(-1.0), uv(-1.0, 0.0));
    }

    // ── Line2::eval_dt ────────────────────────────────────────────────────────

    #[test]
    fn line2_eval_dt_constant() {
        let l = Line2::new(uv(0.0, 0.0), uv(3.0, 4.0));
        let expected = uv(3.0, 4.0);
        assert_eq!(l.eval_dt(0.0), expected);
        assert_eq!(l.eval_dt(0.5), expected);
        assert_eq!(l.eval_dt(1.0), expected);
    }

    #[test]
    fn line2_eval_dt_zero_length() {
        let l = Line2::new(uv(1.0, 1.0), uv(1.0, 1.0));
        assert_eq!(l.eval_dt(0.0), uv(0.0, 0.0));
    }

    // ── Line2::is_degenerate ──────────────────────────────────────────────────

    #[test]
    fn line2_not_degenerate() {
        assert!(!Line2::new(uv(0.0, 0.0), uv(1.0, 0.0)).is_degenerate());
    }

    #[test]
    fn line2_is_degenerate() {
        assert!(Line2::new(uv(1.0, 2.0), uv(1.0, 2.0)).is_degenerate());
    }

    // ── Curve2Kind delegation ─────────────────────────────────────────────────

    #[test]
    fn curve2kind_line2_eval() {
        let l = Line2::new(uv(0.0, 0.0), uv(2.0, 4.0));
        let ck = Curve2Kind::Line2(l);
        assert_eq!(ck.eval(0.0), l.eval(0.0));
        assert_eq!(ck.eval(0.5), l.eval(0.5));
        assert_eq!(ck.eval(1.0), l.eval(1.0));
    }

    #[test]
    fn curve2kind_line2_eval_dt() {
        let l = Line2::new(uv(0.0, 0.0), uv(1.0, 2.0));
        let ck = Curve2Kind::Line2(l);
        assert_eq!(ck.eval_dt(0.0), l.eval_dt(0.0));
        assert_eq!(ck.eval_dt(0.5), l.eval_dt(0.5));
    }

    #[test]
    fn curve2kind_line2_is_degenerate() {
        let nd = Curve2Kind::Line2(Line2::new(uv(0.0, 0.0), uv(1.0, 0.0)));
        let dg = Curve2Kind::Line2(Line2::new(uv(1.0, 1.0), uv(1.0, 1.0)));
        assert!(!nd.is_degenerate());
        assert!(dg.is_degenerate());
    }

    // ── CircularArc2 ──────────────────────────────────────────────────────────

    #[test]
    fn circular_arc2_eval_at_zero() {
        let a = CircularArc2::new(uv(1.0, 2.0), 3.0, 0.0, 2.0 * std::f64::consts::PI);
        let got = a.eval(0.0);
        assert!((got.u - 4.0).abs() < 1e-12);
        assert!((got.v - 2.0).abs() < 1e-12);
    }

    #[test]
    fn circular_arc2_eval_at_half_pi() {
        let a = CircularArc2::new(uv(0.0, 0.0), 1.0, 0.0, 2.0 * std::f64::consts::PI);
        let got = a.eval(std::f64::consts::FRAC_PI_2);
        assert!((got.u - 0.0).abs() < 1e-12);
        assert!((got.v - 1.0).abs() < 1e-12);
    }

    #[test]
    fn circular_arc2_eval_dt_perpendicular() {
        // tangent at t=0 should be (0, r)
        let a = CircularArc2::new(uv(0.0, 0.0), 2.0, 0.0, 2.0 * std::f64::consts::PI);
        let dt = a.eval_dt(0.0);
        assert!((dt.u - 0.0).abs() < 1e-12);
        assert!((dt.v - 2.0).abs() < 1e-12);
    }

    #[test]
    fn circular_arc2_is_degenerate_false() {
        assert!(
            !CircularArc2::new(uv(0.0, 0.0), 1.0, 0.0, 2.0 * std::f64::consts::PI).is_degenerate()
        );
    }

    #[test]
    fn circular_arc2_is_degenerate_true() {
        assert!(
            CircularArc2::new(uv(1.0, 1.0), 0.0, 0.0, 2.0 * std::f64::consts::PI).is_degenerate()
        );
    }

    // ── Curve2Kind CircularArc2 delegation ────────────────────────────────────

    #[test]
    fn curve2kind_arc2_eval() {
        let a = CircularArc2::new(uv(0.0, 0.0), 1.0, 0.0, 2.0 * std::f64::consts::PI);
        let ck = Curve2Kind::CircularArc2(a);
        let got = ck.eval(0.0);
        assert!((got.u - 1.0).abs() < 1e-12);
        assert!((got.v - 0.0).abs() < 1e-12);
    }

    #[test]
    fn curve2kind_arc2_is_degenerate() {
        let nd = Curve2Kind::CircularArc2(CircularArc2::new(
            uv(0.0, 0.0),
            1.0,
            0.0,
            2.0 * std::f64::consts::PI,
        ));
        let dg = Curve2Kind::CircularArc2(CircularArc2::new(
            uv(0.0, 0.0),
            0.0,
            0.0,
            2.0 * std::f64::consts::PI,
        ));
        assert!(!nd.is_degenerate());
        assert!(dg.is_degenerate());
    }

    // ── Polyline3 construction ────────────────────────────────────────────────

    #[test]
    fn polyline3_new_stores_points() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 1.0, 0.0)]);
        assert_eq!(pl.points.len(), 3);
        assert_eq!(pl.n_segments(), 2);
    }

    #[test]
    #[should_panic]
    fn polyline3_new_panics_on_one_point() {
        Polyline3::new(vec![p(0.0, 0.0, 0.0)]);
    }

    // ── Polyline3::eval ───────────────────────────────────────────────────────

    #[test]
    fn polyline3_eval_at_t0() {
        let pl = Polyline3::new(vec![p(1.0, 2.0, 3.0), p(4.0, 5.0, 6.0)]);
        assert_eq!(pl.eval(0.0), p(1.0, 2.0, 3.0));
    }

    #[test]
    fn polyline3_eval_at_t_n() {
        let pl = Polyline3::new(vec![p(1.0, 2.0, 3.0), p(4.0, 5.0, 6.0)]);
        assert_eq!(pl.eval(1.0), p(4.0, 5.0, 6.0));
    }

    #[test]
    fn polyline3_eval_midpoint_first_seg() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(2.0, 0.0, 0.0), p(2.0, 2.0, 0.0)]);
        assert_eq!(pl.eval(0.5), p(1.0, 0.0, 0.0));
    }

    #[test]
    fn polyline3_eval_midpoint_second_seg() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(2.0, 0.0, 0.0), p(2.0, 2.0, 0.0)]);
        assert_eq!(pl.eval(1.5), p(2.0, 1.0, 0.0));
    }

    #[test]
    fn polyline3_eval_at_knot() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 1.0, 0.0)]);
        assert_eq!(pl.eval(1.0), p(1.0, 0.0, 0.0));
    }

    #[test]
    fn polyline3_eval_extrapolate_negative() {
        // t < 0: extrapolates along segment 0
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 1.0, 0.0)]);
        assert!(approx_eq3(pl.eval(-1.0), p(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn polyline3_eval_extrapolate_past_end() {
        // t > n_segments: extrapolates along last segment
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 1.0, 0.0)]);
        assert!(approx_eq3(pl.eval(3.0), p(1.0, 2.0, 0.0)));
    }

    // ── Polyline3::eval_dt ────────────────────────────────────────────────────

    #[test]
    fn polyline3_eval_dt_first_seg() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(3.0, 0.0, 0.0), p(3.0, 4.0, 0.0)]);
        assert_eq!(pl.eval_dt(0.5), p(3.0, 0.0, 0.0));
    }

    #[test]
    fn polyline3_eval_dt_second_seg() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(3.0, 0.0, 0.0), p(3.0, 4.0, 0.0)]);
        assert_eq!(pl.eval_dt(1.5), p(0.0, 4.0, 0.0));
    }

    #[test]
    fn polyline3_eval_dt_at_knot() {
        // t=1.0: floor(1.0)==1, clamped to [0, n-1=1] → segment 1; not segment 0.
        // (This is the "knot picks incoming segment" contract — see notes.md.)
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(3.0, 0.0, 0.0), p(3.0, 4.0, 0.0)]);
        assert_eq!(pl.eval_dt(1.0), p(0.0, 4.0, 0.0));
    }

    // ── Polyline3::is_degenerate ──────────────────────────────────────────────

    #[test]
    fn polyline3_not_degenerate() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)]);
        assert!(!pl.is_degenerate());
    }

    #[test]
    fn polyline3_is_degenerate() {
        let pl = Polyline3::new(vec![p(1.0, 2.0, 3.0), p(1.0, 2.0, 3.0), p(1.0, 2.0, 3.0)]);
        assert!(pl.is_degenerate());
    }

    // ── Curve3Kind::Polyline3 delegation ─────────────────────────────────────

    #[test]
    fn curve3kind_polyline3_eval() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(2.0, 0.0, 0.0), p(2.0, 2.0, 0.0)]);
        let ck = Curve3Kind::Polyline3(pl.clone());
        assert_eq!(ck.eval(0.0), pl.eval(0.0));
        assert_eq!(ck.eval(0.5), pl.eval(0.5));
        assert_eq!(ck.eval(1.5), pl.eval(1.5));
    }

    #[test]
    fn curve3kind_polyline3_eval_dt() {
        let pl = Polyline3::new(vec![p(0.0, 0.0, 0.0), p(2.0, 0.0, 0.0), p(2.0, 2.0, 0.0)]);
        let ck = Curve3Kind::Polyline3(pl.clone());
        assert_eq!(ck.eval_dt(0.5), pl.eval_dt(0.5));
        assert_eq!(ck.eval_dt(1.5), pl.eval_dt(1.5));
    }

    #[test]
    fn curve3kind_polyline3_is_degenerate() {
        let nd = Curve3Kind::Polyline3(Polyline3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)]));
        let dg = Curve3Kind::Polyline3(Polyline3::new(vec![p(1.0, 1.0, 1.0), p(1.0, 1.0, 1.0)]));
        assert!(!nd.is_degenerate());
        assert!(dg.is_degenerate());
    }

    // ── Polyline2 construction ────────────────────────────────────────────────

    #[test]
    fn polyline2_new_stores_points() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(1.0, 0.0), uv(1.0, 1.0)]);
        assert_eq!(pl.points.len(), 3);
        assert_eq!(pl.n_segments(), 2);
    }

    #[test]
    #[should_panic]
    fn polyline2_new_panics_on_one_point() {
        Polyline2::new(vec![uv(0.0, 0.0)]);
    }

    // ── Polyline2::eval ───────────────────────────────────────────────────────

    #[test]
    fn polyline2_eval_at_t0() {
        let pl = Polyline2::new(vec![uv(1.0, 2.0), uv(3.0, 4.0)]);
        assert_eq!(pl.eval(0.0), uv(1.0, 2.0));
    }

    #[test]
    fn polyline2_eval_at_t_n() {
        let pl = Polyline2::new(vec![uv(1.0, 2.0), uv(3.0, 4.0)]);
        assert_eq!(pl.eval(1.0), uv(3.0, 4.0));
    }

    #[test]
    fn polyline2_eval_midpoint_first_seg() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(2.0, 0.0), uv(2.0, 4.0)]);
        assert_eq!(pl.eval(0.5), uv(1.0, 0.0));
    }

    #[test]
    fn polyline2_eval_midpoint_second_seg() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(2.0, 0.0), uv(2.0, 4.0)]);
        assert_eq!(pl.eval(1.5), uv(2.0, 2.0));
    }

    #[test]
    fn polyline2_eval_extrapolate_negative() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(1.0, 0.0), uv(1.0, 1.0)]);
        assert_eq!(pl.eval(-1.0), uv(-1.0, 0.0));
    }

    #[test]
    fn polyline2_eval_extrapolate_past_end() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(1.0, 0.0), uv(1.0, 1.0)]);
        assert_eq!(pl.eval(3.0), uv(1.0, 2.0));
    }

    // ── Polyline2::eval_dt ────────────────────────────────────────────────────

    #[test]
    fn polyline2_eval_dt_first_seg() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(3.0, 0.0), uv(3.0, 4.0)]);
        assert_eq!(pl.eval_dt(0.5), uv(3.0, 0.0));
    }

    #[test]
    fn polyline2_eval_dt_second_seg() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(3.0, 0.0), uv(3.0, 4.0)]);
        assert_eq!(pl.eval_dt(1.5), uv(0.0, 4.0));
    }

    // ── Polyline2::is_degenerate ──────────────────────────────────────────────

    #[test]
    fn polyline2_not_degenerate() {
        assert!(!Polyline2::new(vec![uv(0.0, 0.0), uv(1.0, 0.0)]).is_degenerate());
    }

    #[test]
    fn polyline2_is_degenerate() {
        assert!(Polyline2::new(vec![uv(1.0, 2.0), uv(1.0, 2.0), uv(1.0, 2.0)]).is_degenerate());
    }

    // ── Curve2Kind::Polyline2 delegation ─────────────────────────────────────

    #[test]
    fn curve2kind_polyline2_eval() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(2.0, 0.0), uv(2.0, 2.0)]);
        let ck = Curve2Kind::Polyline2(pl.clone());
        assert_eq!(ck.eval(0.0), pl.eval(0.0));
        assert_eq!(ck.eval(0.5), pl.eval(0.5));
        assert_eq!(ck.eval(1.5), pl.eval(1.5));
    }

    #[test]
    fn curve2kind_polyline2_eval_dt() {
        let pl = Polyline2::new(vec![uv(0.0, 0.0), uv(2.0, 0.0), uv(2.0, 2.0)]);
        let ck = Curve2Kind::Polyline2(pl.clone());
        assert_eq!(ck.eval_dt(0.5), pl.eval_dt(0.5));
        assert_eq!(ck.eval_dt(1.5), pl.eval_dt(1.5));
    }

    #[test]
    fn curve2kind_polyline2_is_degenerate() {
        let nd = Curve2Kind::Polyline2(Polyline2::new(vec![uv(0.0, 0.0), uv(1.0, 0.0)]));
        let dg = Curve2Kind::Polyline2(Polyline2::new(vec![uv(1.0, 1.0), uv(1.0, 1.0)]));
        assert!(!nd.is_degenerate());
        assert!(dg.is_degenerate());
    }

    // ── Plane ─────────────────────────────────────────────────────────────────

    #[test]
    fn plane_eval_origin() {
        let pl = Plane::new(p(1.0, 2.0, 3.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval(0.0, 0.0), p(1.0, 2.0, 3.0));
    }

    #[test]
    fn plane_eval_along_u() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval(3.0, 0.0), p(3.0, 0.0, 0.0));
    }

    #[test]
    fn plane_eval_along_v() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval(0.0, 5.0), p(0.0, 5.0, 0.0));
    }

    #[test]
    fn plane_eval_du_constant() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval_du(0.0, 0.0), p(1.0, 0.0, 0.0));
        assert_eq!(pl.eval_du(3.0, 7.0), p(1.0, 0.0, 0.0));
    }

    #[test]
    fn plane_eval_dv_constant() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval_dv(0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval_dv(3.0, 7.0), p(0.0, 1.0, 0.0));
    }

    #[test]
    fn plane_eval_n_xy_plane() {
        // xy-plane: u_dir=x, v_dir=y → normal should be +z
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval_n(0.0, 0.0), Some(p(0.0, 0.0, 1.0)));
    }

    #[test]
    fn plane_eval_n_always_some() {
        let pl = Plane::new(p(1.0, 2.0, 3.0), p(1.0, 0.0, 0.0), p(0.0, 0.0, 1.0));
        assert!(pl.eval_n(0.0, 0.0).is_some());
        assert!(pl.eval_n(5.0, 9.0).is_some());
    }

    #[test]
    fn plane_eval_n_is_unit_length() {
        // tilted plane: u_dir and v_dir are unit, perpendicular but not axis-aligned
        let u = p(1.0, 1.0, 0.0).normalize();
        let v = p(0.0, 0.0, 1.0);
        let pl = Plane::new(p(0.0, 0.0, 0.0), u, v);
        let n = pl.eval_n(0.0, 0.0).unwrap();
        assert!((n.length() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn plane_eval_n_constant() {
        // normal does not vary with (u, v)
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        assert_eq!(pl.eval_n(0.0, 0.0), pl.eval_n(3.0, 7.0));
    }

    // ── SurfaceKind delegation ────────────────────────────────────────────────

    #[test]
    fn surfacekind_plane_eval() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        let sk = SurfaceKind::Plane(pl);
        assert!(approx_eq3(sk.eval(1.0, 2.0), pl.eval(1.0, 2.0)));
    }

    #[test]
    fn surfacekind_plane_eval_du() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        let sk = SurfaceKind::Plane(pl);
        assert_eq!(sk.eval_du(1.0, 2.0), pl.eval_du(1.0, 2.0));
    }

    #[test]
    fn surfacekind_plane_eval_dv() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        let sk = SurfaceKind::Plane(pl);
        assert_eq!(sk.eval_dv(1.0, 2.0), pl.eval_dv(1.0, 2.0));
    }

    #[test]
    fn surfacekind_plane_eval_n() {
        let pl = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0));
        let sk = SurfaceKind::Plane(pl);
        assert_eq!(sk.eval_n(1.0, 2.0), pl.eval_n(1.0, 2.0));
    }

    // ── CircularArc3 ──────────────────────────────────────────────────────────
    //
    // Canonical frame: center=(0,0,0), normal=(0,0,1), ref_dir=(1,0,0) → ê₂=(0,1,0)

    fn std_arc(radius: f64) -> CircularArc3 {
        CircularArc3::new(
            p(0.0, 0.0, 0.0),
            p(0.0, 0.0, 1.0),
            p(1.0, 0.0, 0.0),
            radius,
            0.0,
            2.0 * std::f64::consts::PI,
        )
    }

    #[test]
    fn arc3_eval_at_zero() {
        let a = std_arc(3.0);
        assert!(approx_eq3(a.eval(0.0), p(3.0, 0.0, 0.0)));
    }

    #[test]
    fn arc3_eval_at_half_pi() {
        let a = std_arc(3.0);
        assert!(approx_eq3(
            a.eval(std::f64::consts::FRAC_PI_2),
            p(0.0, 3.0, 0.0)
        ));
    }

    #[test]
    fn arc3_eval_at_pi() {
        let a = std_arc(2.0);
        assert!(approx_eq3(a.eval(std::f64::consts::PI), p(-2.0, 0.0, 0.0)));
    }

    #[test]
    fn arc3_eval_full_circle_closure() {
        let a = std_arc(1.0);
        assert!(approx_eq3(a.eval(2.0 * std::f64::consts::PI), a.eval(0.0)));
    }

    #[test]
    fn arc3_eval_dt_at_zero() {
        // tangent at t=0 points in +ê₂ direction, scaled by radius
        let a = std_arc(3.0);
        assert!(approx_eq3(a.eval_dt(0.0), p(0.0, 3.0, 0.0)));
    }

    #[test]
    fn arc3_eval_dt_at_half_pi() {
        let a = std_arc(2.0);
        assert!(approx_eq3(
            a.eval_dt(std::f64::consts::FRAC_PI_2),
            p(-2.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn arc3_not_degenerate() {
        assert!(!std_arc(1.0).is_degenerate());
    }

    #[test]
    fn arc3_degenerate_zero_radius() {
        assert!(std_arc(0.0).is_degenerate());
    }

    #[test]
    fn curve3kind_arc3_eval_delegates() {
        let a = std_arc(2.0);
        let ck = Curve3Kind::CircularArc3(a);
        assert!(approx_eq3(ck.eval(0.0), a.eval(0.0)));
        assert!(approx_eq3(
            ck.eval(std::f64::consts::FRAC_PI_2),
            a.eval(std::f64::consts::FRAC_PI_2)
        ));
    }

    // ── CylindricalSurface ────────────────────────────────────────────────────
    //
    // Canonical: origin=(0,0,0), axis=(0,0,1), ref_dir=(1,0,0), radius=2

    fn std_cyl() -> CylindricalSurface {
        CylindricalSurface::new(p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0), p(1.0, 0.0, 0.0), 2.0)
    }

    #[test]
    fn cyl_eval_at_u0_v0() {
        assert_eq!(std_cyl().eval(0.0, 0.0), p(2.0, 0.0, 0.0));
    }

    #[test]
    fn cyl_eval_at_half_pi_v0() {
        assert!(approx_eq3(
            std_cyl().eval(std::f64::consts::FRAC_PI_2, 0.0),
            p(0.0, 2.0, 0.0)
        ));
    }

    #[test]
    fn cyl_eval_v_moves_along_axis() {
        assert!(approx_eq3(std_cyl().eval(0.0, 5.0), p(2.0, 0.0, 5.0)));
    }

    #[test]
    fn cyl_eval_du_tangent() {
        // du-tangent at u=0 points in +ê₂ = (0,1,0), scaled by radius
        assert!(approx_eq3(std_cyl().eval_du(0.0, 0.0), p(0.0, 2.0, 0.0)));
    }

    #[test]
    fn cyl_eval_dv_is_axis() {
        assert_eq!(std_cyl().eval_dv(0.0, 0.0), p(0.0, 0.0, 1.0));
        assert_eq!(std_cyl().eval_dv(1.0, 5.0), p(0.0, 0.0, 1.0));
    }

    #[test]
    fn cyl_eval_n_radial() {
        assert!(approx_eq3(
            std_cyl().eval_n(0.0, 0.0).unwrap(),
            p(1.0, 0.0, 0.0)
        ));
        assert!(approx_eq3(
            std_cyl().eval_n(std::f64::consts::FRAC_PI_2, 3.0).unwrap(),
            p(0.0, 1.0, 0.0),
        ));
    }

    #[test]
    fn cyl_eval_n_unit_length() {
        let n = std_cyl().eval_n(1.0, 7.0).unwrap();
        assert!((n.length() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn surfacekind_cylinder_delegates() {
        let c = std_cyl();
        let sk = SurfaceKind::Cylinder(c);
        assert!(approx_eq3(sk.eval(0.0, 1.0), c.eval(0.0, 1.0)));
        assert!(approx_eq3(sk.eval_du(0.0, 1.0), c.eval_du(0.0, 1.0)));
        assert!(approx_eq3(sk.eval_dv(0.0, 1.0), c.eval_dv(0.0, 1.0)));
        assert_eq!(sk.eval_n(0.0, 1.0), c.eval_n(0.0, 1.0));
    }

    // ── ConicalSurface ────────────────────────────────────────────────────────
    //
    // Canonical: apex=(0,0,0), axis=(0,0,1), ref_dir=(1,0,0), half_angle=π/4

    fn std_cone() -> ConicalSurface {
        ConicalSurface::new(
            p(0.0, 0.0, 0.0),
            p(0.0, 0.0, 1.0),
            p(1.0, 0.0, 0.0),
            std::f64::consts::FRAC_PI_4,
        )
    }

    #[test]
    fn cone_eval_at_apex() {
        // v=0 always gives the apex regardless of u
        let c = std_cone();
        assert_eq!(c.eval(0.0, 0.0), p(0.0, 0.0, 0.0));
        assert_eq!(c.eval(1.234, 0.0), p(0.0, 0.0, 0.0));
    }

    #[test]
    fn cone_eval_at_u0_v1() {
        // slant dir = cos(π/4)*(0,0,1) + sin(π/4)*(1,0,0) = (1/√2, 0, 1/√2)
        let s = std::f64::consts::FRAC_1_SQRT_2;
        assert!(approx_eq3(std_cone().eval(0.0, 1.0), p(s, 0.0, s)));
    }

    #[test]
    fn cone_eval_n_none_at_apex() {
        assert_eq!(std_cone().eval_n(0.0, 0.0), None);
        assert_eq!(std_cone().eval_n(1.5, 0.0), None);
    }

    #[test]
    fn cone_eval_n_at_u0_v1() {
        // outward normal = cos(π/4)*(1,0,0) - sin(π/4)*(0,0,1) = (1/√2, 0, -1/√2)
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let n = std_cone().eval_n(0.0, 1.0).unwrap();
        assert!(approx_eq3(n, p(s, 0.0, -s)));
    }

    #[test]
    fn cone_eval_n_unit_length() {
        let n = std_cone().eval_n(1.0, 2.0).unwrap();
        assert!((n.length() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn surfacekind_cone_delegates() {
        let c = std_cone();
        let sk = SurfaceKind::Cone(c);
        assert!(approx_eq3(sk.eval(0.0, 1.0), c.eval(0.0, 1.0)));
        assert_eq!(sk.eval_n(0.0, 0.0), None);
        assert_eq!(sk.eval_n(0.0, 1.0), c.eval_n(0.0, 1.0));
    }

    // ── SphericalSurface ──────────────────────────────────────────────────────
    //
    // Canonical: center=(0,0,0), radius=3, ref_dir=(1,0,0), axis=(0,0,1)

    fn std_sphere() -> SphericalSurface {
        SphericalSurface::new(p(0.0, 0.0, 0.0), 3.0, p(1.0, 0.0, 0.0), p(0.0, 0.0, 1.0))
    }

    #[test]
    fn sphere_eval_equatorial_ref() {
        assert!(approx_eq3(std_sphere().eval(0.0, 0.0), p(3.0, 0.0, 0.0)));
    }

    #[test]
    fn sphere_eval_north_pole() {
        assert!(approx_eq3(
            std_sphere().eval(0.0, std::f64::consts::FRAC_PI_2),
            p(0.0, 0.0, 3.0)
        ));
    }

    #[test]
    fn sphere_eval_south_pole() {
        assert!(approx_eq3(
            std_sphere().eval(0.0, -std::f64::consts::FRAC_PI_2),
            p(0.0, 0.0, -3.0)
        ));
    }

    #[test]
    fn sphere_eval_equatorial_quarter() {
        assert!(approx_eq3(
            std_sphere().eval(std::f64::consts::FRAC_PI_2, 0.0),
            p(0.0, 3.0, 0.0),
        ));
    }

    #[test]
    fn sphere_eval_n_equatorial_ref() {
        assert!(approx_eq3(
            std_sphere().eval_n(0.0, 0.0).unwrap(),
            p(1.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn sphere_eval_n_north_pole() {
        // normal at north pole = axis = (0,0,1), independent of u
        let n0 = std_sphere()
            .eval_n(0.0, std::f64::consts::FRAC_PI_2)
            .unwrap();
        let n1 = std_sphere()
            .eval_n(1.234, std::f64::consts::FRAC_PI_2)
            .unwrap();
        assert!(approx_eq3(n0, p(0.0, 0.0, 1.0)));
        assert!(approx_eq3(n1, p(0.0, 0.0, 1.0)));
    }

    #[test]
    fn sphere_eval_n_south_pole() {
        let n = std_sphere()
            .eval_n(0.5, -std::f64::consts::FRAC_PI_2)
            .unwrap();
        assert!(approx_eq3(n, p(0.0, 0.0, -1.0)));
    }

    #[test]
    fn sphere_eval_n_always_some() {
        assert!(std_sphere().eval_n(0.0, 0.0).is_some());
        assert!(
            std_sphere()
                .eval_n(0.0, std::f64::consts::FRAC_PI_2)
                .is_some()
        );
        assert!(
            std_sphere()
                .eval_n(0.0, -std::f64::consts::FRAC_PI_2)
                .is_some()
        );
    }

    #[test]
    fn sphere_eval_n_unit_length() {
        let n = std_sphere().eval_n(1.0, 0.5).unwrap();
        assert!((n.length() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn surfacekind_sphere_delegates() {
        let s = std_sphere();
        let sk = SurfaceKind::Sphere(s);
        assert!(approx_eq3(sk.eval(0.0, 0.0), s.eval(0.0, 0.0)));
        assert_eq!(sk.eval_n(0.0, 0.0), s.eval_n(0.0, 0.0));
    }

    // ── LinearExtrusionSurface helpers ────────────────────────────────────────

    /// Profile: Line3 from (0,0,0) to (1,0,0); direction: +Z.
    fn std_les() -> LinearExtrusionSurface {
        let profile = Curve3Kind::Line3(Line3::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)));
        LinearExtrusionSurface::new(profile, p(0.0, 0.0, 1.0))
    }

    // ── LinearExtrusionSurface construction ───────────────────────────────────

    #[test]
    fn les_new_stores_fields() {
        let s = std_les();
        assert_eq!(s.direction, p(0.0, 0.0, 1.0));
        // profile stored: eval at u=0 gives origin
        assert_eq!(s.profile.eval(0.0), p(0.0, 0.0, 0.0));
    }

    // ── LinearExtrusionSurface::eval ──────────────────────────────────────────

    #[test]
    fn les_eval_at_v0() {
        let s = std_les();
        // At v=0 the direction term vanishes; eval should equal profile.eval(u)
        assert!(approx_eq3(s.eval(0.0, 0.0), s.profile.eval(0.0)));
        assert!(approx_eq3(s.eval(0.5, 0.0), s.profile.eval(0.5)));
        assert!(approx_eq3(s.eval(1.0, 0.0), s.profile.eval(1.0)));
    }

    #[test]
    fn les_eval_along_direction() {
        let s = std_les();
        // Fixed u=0 (profile origin); varying v moves along +Z
        assert!(approx_eq3(s.eval(0.0, 3.0), p(0.0, 0.0, 3.0)));
        assert!(approx_eq3(s.eval(0.0, -1.0), p(0.0, 0.0, -1.0)));
    }

    #[test]
    fn les_eval_midpoint() {
        let s = std_les();
        // u=0.5 → profile midpoint (0.5, 0, 0); v=2 → +2 along Z
        assert!(approx_eq3(s.eval(0.5, 2.0), p(0.5, 0.0, 2.0)));
    }

    // ── LinearExtrusionSurface::eval_du ───────────────────────────────────────

    #[test]
    fn les_eval_du_matches_profile_tangent() {
        let s = std_les();
        assert!(approx_eq3(s.eval_du(0.0, 0.0), s.profile.eval_dt(0.0)));
        assert!(approx_eq3(s.eval_du(0.5, 0.0), s.profile.eval_dt(0.5)));
    }

    #[test]
    fn les_eval_du_v_independent() {
        let s = std_les();
        let at_v0 = s.eval_du(0.5, 0.0);
        let at_v5 = s.eval_du(0.5, 5.0);
        assert!(approx_eq3(at_v0, at_v5));
    }

    // ── LinearExtrusionSurface::eval_dv ───────────────────────────────────────

    #[test]
    fn les_eval_dv_is_direction() {
        let s = std_les();
        assert!(approx_eq3(s.eval_dv(0.0, 0.0), s.direction));
        assert!(approx_eq3(s.eval_dv(0.7, 3.0), s.direction));
    }

    // ── LinearExtrusionSurface::eval_n ────────────────────────────────────────

    #[test]
    fn les_eval_n_unit_length() {
        let s = std_les();
        let n = s.eval_n(0.5, 1.0).unwrap();
        assert!((n.length() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn les_eval_n_perpendicular_to_du_and_dv() {
        let s = std_les();
        let n = s.eval_n(0.5, 1.0).unwrap();
        let du = s.eval_du(0.5, 1.0);
        let dv = s.eval_dv(0.5, 1.0);
        assert!(du.dot(n).abs() < 1e-14);
        assert!(dv.dot(n).abs() < 1e-14);
    }

    #[test]
    fn les_eval_n_none_on_degenerate_profile() {
        // Degenerate profile: both endpoints the same → zero tangent everywhere
        let profile = Curve3Kind::Line3(Line3::new(p(1.0, 1.0, 0.0), p(1.0, 1.0, 0.0)));
        let s = LinearExtrusionSurface::new(profile, p(0.0, 0.0, 1.0));
        assert!(s.eval_n(0.0, 0.0).is_none());
    }

    // ── LinearExtrusionSurface SurfaceKind delegation ─────────────────────────

    #[test]
    fn surfacekind_les_eval() {
        let s = std_les();
        let sk = SurfaceKind::Extrusion(s.clone());
        assert!(approx_eq3(sk.eval(0.0, 0.0), s.eval(0.0, 0.0)));
        assert!(approx_eq3(sk.eval(0.5, 2.0), s.eval(0.5, 2.0)));
    }

    #[test]
    fn surfacekind_les_eval_n() {
        let s = std_les();
        let sk = SurfaceKind::Extrusion(s.clone());
        assert_eq!(sk.eval_n(0.5, 1.0).is_some(), s.eval_n(0.5, 1.0).is_some());
        let n_sk = sk.eval_n(0.5, 1.0).unwrap();
        let n_s = s.eval_n(0.5, 1.0).unwrap();
        assert!(approx_eq3(n_sk, n_s));
    }

    // ── RevolutionSurface helpers ─────────────────────────────────────────────

    /// Profile: Line3 from (1,0,0) to (2,0,0) (radial, no Z component).
    /// Axis: Z through origin. This sweeps a flat annular strip.
    fn std_rs() -> RevolutionSurface {
        let profile = Curve3Kind::Line3(Line3::new(p(1.0, 0.0, 0.0), p(2.0, 0.0, 0.0)));
        RevolutionSurface::new(profile, p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0))
    }

    // ── RevolutionSurface construction ────────────────────────────────────────

    #[test]
    fn rs_new_stores_fields() {
        let s = std_rs();
        assert_eq!(s.axis_origin, p(0.0, 0.0, 0.0));
        assert_eq!(s.axis_dir, p(0.0, 0.0, 1.0));
        assert_eq!(s.profile.eval(0.0), p(1.0, 0.0, 0.0));
    }

    // ── RevolutionSurface::eval ───────────────────────────────────────────────

    #[test]
    fn rs_eval_at_u0() {
        let s = std_rs();
        // No rotation; eval equals profile.eval(v)
        assert!(approx_eq3(s.eval(0.0, 0.0), p(1.0, 0.0, 0.0)));
        assert!(approx_eq3(s.eval(0.0, 1.0), p(2.0, 0.0, 0.0)));
    }

    #[test]
    fn rs_eval_at_u_half_pi() {
        let s = std_rs();
        // 90° rotation of (1,0,0) around Z → (0,1,0)
        let got = s.eval(std::f64::consts::FRAC_PI_2, 0.0);
        assert!((got.x - 0.0).abs() < 1e-14);
        assert!((got.y - 1.0).abs() < 1e-14);
        assert!((got.z - 0.0).abs() < 1e-14);
    }

    #[test]
    fn rs_eval_at_u_pi() {
        let s = std_rs();
        // 180° rotation of (1,0,0) around Z → (-1,0,0)
        let got = s.eval(std::f64::consts::PI, 0.0);
        assert!((got.x - -1.0).abs() < 1e-14);
        assert!((got.y - 0.0).abs() < 1e-12);
        assert!((got.z - 0.0).abs() < 1e-14);
    }

    #[test]
    fn rs_eval_full_rotation() {
        let s = std_rs();
        // eval(2π, v) ≈ eval(0, v)
        let a = s.eval(0.0, 0.5);
        let b = s.eval(2.0 * std::f64::consts::PI, 0.5);
        assert!(approx_eq3(a, b));
    }

    #[test]
    fn rs_eval_on_axis() {
        // Profile point on the Z axis: rotation should leave it unchanged
        let profile = Curve3Kind::Line3(Line3::new(p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0)));
        let s = RevolutionSurface::new(profile, p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0));
        let pt = s.eval(1.23, 0.0);
        assert!(approx_eq3(pt, p(0.0, 0.0, 0.0)));
    }

    // ── RevolutionSurface::eval_du ────────────────────────────────────────────

    #[test]
    fn rs_eval_du_perpendicular_to_axis() {
        let s = std_rs();
        let du = s.eval_du(0.0, 0.0);
        assert!(s.axis_dir.dot(du).abs() < 1e-14);
    }

    #[test]
    fn rs_eval_du_zero_on_axis() {
        // When the profile point is on the axis, eval_du should be zero
        let profile = Curve3Kind::Line3(Line3::new(p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0)));
        let s = RevolutionSurface::new(profile, p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0));
        let du = s.eval_du(0.7, 0.0); // profile.eval(0) = (0,0,0) = on axis
        assert!(approx_eq3(du, p(0.0, 0.0, 0.0)));
    }

    // ── RevolutionSurface::eval_dv ────────────────────────────────────────────

    #[test]
    fn rs_eval_dv_at_u0() {
        let s = std_rs();
        // No rotation: dv should equal profile tangent
        let dv = s.eval_dv(0.0, 0.5);
        let tangent = s.profile.eval_dt(0.5);
        assert!(approx_eq3(dv, tangent));
    }

    #[test]
    fn rs_eval_dv_rotated() {
        let s = std_rs();
        // At u=π/2, profile tangent (1,0,0) rotated 90° around Z → (0,1,0)
        let dv = s.eval_dv(std::f64::consts::FRAC_PI_2, 0.5);
        assert!((dv.x - 0.0).abs() < 1e-14);
        assert!((dv.y - 1.0).abs() < 1e-14);
        assert!((dv.z - 0.0).abs() < 1e-14);
    }

    // ── RevolutionSurface::eval_n ─────────────────────────────────────────────

    #[test]
    fn rs_eval_n_unit_length() {
        let s = std_rs();
        let n = s.eval_n(0.5, 0.5).unwrap();
        assert!((n.length() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn rs_eval_n_perpendicular_to_du_and_dv() {
        let s = std_rs();
        let n = s.eval_n(0.5, 0.5).unwrap();
        let du = s.eval_du(0.5, 0.5);
        let dv = s.eval_dv(0.5, 0.5);
        assert!(du.dot(n).abs() < 1e-14);
        assert!(dv.dot(n).abs() < 1e-14);
    }

    #[test]
    fn rs_eval_n_none_on_axis() {
        // Profile point on the axis → eval_du is zero → normal is None
        let profile = Curve3Kind::Line3(Line3::new(p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0)));
        let s = RevolutionSurface::new(profile, p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0));
        assert!(s.eval_n(0.7, 0.0).is_none()); // profile.eval(0) = (0,0,0) = on axis
    }

    // ── RevolutionSurface SurfaceKind delegation ──────────────────────────────

    #[test]
    fn surfacekind_rs_eval() {
        let s = std_rs();
        let sk = SurfaceKind::Revolution(s.clone());
        assert!(approx_eq3(sk.eval(0.0, 0.0), s.eval(0.0, 0.0)));
        assert!(approx_eq3(
            sk.eval(std::f64::consts::FRAC_PI_2, 0.5),
            s.eval(std::f64::consts::FRAC_PI_2, 0.5)
        ));
    }

    #[test]
    fn surfacekind_rs_eval_n() {
        let s = std_rs();
        let sk = SurfaceKind::Revolution(s.clone());
        let n_sk = sk.eval_n(0.5, 0.5).unwrap();
        let n_s = s.eval_n(0.5, 0.5).unwrap();
        assert!(approx_eq3(n_sk, n_s));
    }

    // ── Path2D construction ───────────────────────────────────────────────────

    #[test]
    fn path2d_new_empty() {
        let path = Path2D::new();
        assert_eq!(path.n_contours(), 0);
    }

    #[test]
    fn path2d_new_no_open_contour() {
        // finish() on an empty set should be OK (zero contours is a valid empty path,
        // though compilers may reject it; the builder itself doesn't).
        let path = Path2D::new().finish().expect("empty set finishes clean");
        assert_eq!(path.n_contours(), 0);
    }

    // ── Path2D::start_contour ─────────────────────────────────────────────────

    #[test]
    fn start_contour_opens_first_contour() {
        let mut p = Path2D::new();
        p.start_contour(uv(1.0, 2.0)).unwrap();
        assert_eq!(p.n_contours(), 1);
        assert_eq!(p.current_pos().unwrap(), uv(1.0, 2.0));
    }

    #[test]
    fn start_contour_opens_second_after_close() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to_close()
            .unwrap();
        p.start_contour(uv(5.0, 5.0)).unwrap();
        assert_eq!(p.n_contours(), 2);
        assert_eq!(p.current_pos().unwrap(), uv(5.0, 5.0));
    }

    #[test]
    fn start_contour_errors_if_previous_unclosed() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0)).unwrap().line_to(uv(1.0, 0.0));
        // previous contour is open (non-empty + not closed)
        let err = p.start_contour(uv(5.0, 5.0)).unwrap_err();
        assert_eq!(err, PathError::UnclosedContour);
        // original contour is untouched
        assert_eq!(p.n_contours(), 1);
    }

    #[test]
    fn start_contour_errors_no_open_contour_on_close() {
        let mut p = Path2D::new();
        // no contour open yet
        let err = p.close().unwrap_err();
        assert_eq!(err, PathError::NoOpenContour);
    }

    // ── Path2D::line_to (infallible append) ────────────────────────────────────

    #[test]
    fn line_to_adds_segment_to_current_contour() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0)).unwrap();
        p.line_to(uv(1.0, 0.0));
        let contour = p.contour(0);
        assert_eq!(contour.segments.len(), 1);
        assert_eq!(contour.segments[0].eval(0.0), uv(0.0, 0.0));
        assert_eq!(contour.segments[0].eval(1.0), uv(1.0, 0.0));
    }

    #[test]
    fn line_to_advances_current_pos() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0)).unwrap();
        p.line_to(uv(2.0, 3.0));
        assert_eq!(p.current_pos().unwrap(), uv(2.0, 3.0));
    }

    #[test]
    fn line_to_chained() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(1.0, 1.0));
        let contour = p.contour(0);
        assert_eq!(contour.segments.len(), 2);
        assert_eq!(contour.segments[1].eval(0.0), uv(1.0, 0.0));
        assert_eq!(p.current_pos().unwrap(), uv(1.0, 1.0));
    }

    #[test]
    #[should_panic(expected = "no open contour")]
    fn line_to_panics_with_no_open_contour() {
        let mut p = Path2D::new();
        p.line_to(uv(1.0, 0.0));
    }

    // ── Path2D::arc_to ────────────────────────────────────────────────────────

    #[test]
    fn arc_to_adds_arc_segment() {
        let mut p = Path2D::new();
        p.start_contour(uv(1.0, 0.0)).unwrap();
        p.arc_to(uv(0.0, 0.0), std::f64::consts::FRAC_PI_2);
        let contour = p.contour(0);
        assert_eq!(contour.segments.len(), 1);
        matches!(contour.segments[0], Curve2Kind::CircularArc2(_));
    }

    #[test]
    fn arc_to_ccw_advances_current_pos() {
        let mut p = Path2D::new();
        p.start_contour(uv(1.0, 0.0)).unwrap();
        p.arc_to(uv(0.0, 0.0), std::f64::consts::FRAC_PI_2);
        let pos = p.current_pos().unwrap();
        assert!((pos.u - 0.0).abs() < 1e-14);
        assert!((pos.v - 1.0).abs() < 1e-14);
    }

    // ── Path2D::close ─────────────────────────────────────────────────────────

    #[test]
    fn close_sets_closed_flag() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(0.0, 1.0))
            .line_to(uv(0.0, 0.0)); // back to start exactly
        p.close().unwrap();
        assert!(p.contour(0).closed);
    }

    #[test]
    fn close_does_not_add_segment() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(0.0, 1.0))
            .line_to(uv(0.0, 0.0));
        let count_before = p.contour(0).segments.len();
        p.close().unwrap();
        assert_eq!(p.contour(0).segments.len(), count_before);
    }

    #[test]
    fn close_errors_when_not_at_start() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(1.0, 1.0)); // current_pos != start
        let err = p.close().unwrap_err();
        assert_eq!(err, PathError::CloseNotAtStart);
        // contour not closed
        assert!(!p.contour(0).closed);
    }

    // ── Path2D::line_to_close ─────────────────────────────────────────────────

    #[test]
    fn line_to_close_adds_segment() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(1.0, 1.0));
        p.line_to_close().unwrap();
        let contour = p.contour(0);
        assert_eq!(contour.segments.len(), 3);
        assert_eq!(contour.segments[2].eval(1.0), uv(0.0, 0.0));
    }

    #[test]
    fn line_to_close_sets_closed_flag() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0)).unwrap().line_to(uv(1.0, 0.0));
        p.line_to_close().unwrap();
        assert!(p.contour(0).closed);
    }

    #[test]
    fn line_to_close_current_pos_is_start() {
        let mut p = Path2D::new();
        p.start_contour(uv(2.0, 3.0))
            .unwrap()
            .line_to(uv(5.0, 3.0))
            .line_to(uv(5.0, 6.0));
        p.line_to_close().unwrap();
        assert_eq!(p.current_pos().unwrap(), uv(2.0, 3.0));
    }

    // ── Path2D::finish ────────────────────────────────────────────────────────

    #[test]
    fn finish_returns_validated_path() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(1.0, 1.0))
            .line_to_close()
            .unwrap();
        let path = p.finish().expect("valid triangle finishes");
        assert_eq!(path.n_contours(), 1);
        assert!(path.contour(0).closed);
    }

    #[test]
    fn finish_errors_on_unclosed_contour() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0)).unwrap().line_to(uv(1.0, 0.0));
        // forgot to close
        let err = p.finish().unwrap_err();
        assert_eq!(err, PathError::UnclosedContour);
    }

    #[test]
    fn finish_errors_on_empty_contour() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0)).unwrap();
        // opened but added no segments
        let err = p.finish().unwrap_err();
        assert_eq!(err, PathError::EmptyContour);
    }

    #[test]
    fn finish_errors_on_zero_area_contour() {
        // a back-and-forth line: out and back along the same segment → zero signed area
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(0.0, 0.0));
        p.close().unwrap();
        let err = p.finish().unwrap_err();
        assert_eq!(err, PathError::ZeroAreaContour);
    }

    #[test]
    fn finish_errors_on_degenerate_segment() {
        // a zero-length line segment is degenerate
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(0.0, 0.0)) // degenerate
            .line_to(uv(1.0, 0.0))
            .line_to_close()
            .unwrap();
        let err = p.finish().unwrap_err();
        assert_eq!(err, PathError::DegenerateSegment);
    }

    #[test]
    fn finish_two_contours_ok() {
        // like an "o with umlaut" skeleton: outer triangle + a small inner triangle
        let mut p = Path2D::new();
        // outer (CCW)
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(4.0, 0.0))
            .line_to(uv(2.0, 4.0))
            .line_to_close()
            .unwrap();
        // inner (CW) — a small triangle inside, wound clockwise
        p.start_contour(uv(1.5, 1.0))
            .unwrap()
            .line_to(uv(1.5, 2.0))
            .line_to(uv(2.5, 1.5))
            .line_to_close()
            .unwrap();
        let path = p.finish().expect("two-contour path finishes");
        assert_eq!(path.n_contours(), 2);
        assert!(path.contour(0).closed);
        assert!(path.contour(1).closed);
    }

    // ── Path2D::Display ───────────────────────────────────────────────────────

    #[test]
    fn path2d_display_empty() {
        let path = Path2D::new().finish().unwrap();
        let s = format!("{path}");
        assert!(s.starts_with("path("), "header missing: {s}");
        assert!(s.contains("contours=0"), "body wrong: {s}");
    }

    #[test]
    fn path2d_display_triangle() {
        let mut p = Path2D::new();
        p.start_contour(uv(0.0, 0.0))
            .unwrap()
            .line_to(uv(1.0, 0.0))
            .line_to(uv(0.5, 1.0))
            .line_to_close()
            .unwrap();
        let path = p.finish().unwrap();
        let s = format!("{path}");
        assert!(s.starts_with("path("), "header missing: {s}");
        assert!(s.contains("contours=1"), "body wrong: {s}");
        assert!(s.contains("├── line_to("), "missing ├── entries: {s}");
        assert!(s.contains("└── line_to("), "missing └── entry: {s}");
    }
}
