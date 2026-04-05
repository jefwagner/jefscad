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
        Self { p0, p1, t_min: 0.0, t_max: 1.0 }
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

// ── Stub types for future Curve3Kind variants ─────────────────────────────────

/// A circular arc in 3-D space. Fields TBD — stub for `Curve3Kind`.
pub struct CircularArc3;

/// A rational B-spline curve in 3-D space. Fields TBD — stub for `Curve3Kind`.
pub struct NurbsCurve3;

/// A surface-surface intersection curve. Fields TBD — Phase 5.
pub struct SsiCurve3;

// ── Curve3Kind enum ───────────────────────────────────────────────────────────

/// The concrete stored curve type used in the B-rep arena.
///
/// Each variant wraps a concrete curve struct that implements [`Curve3`]. Methods
/// delegate to the inner type; unimplemented variants panic with `todo!`.
pub enum Curve3Kind {
    Line3(Line3),
    CircularArc3(CircularArc3),
    Nurbs(NurbsCurve3),
    Ssi(SsiCurve3),
}

impl Curve3 for Curve3Kind {
    fn eval(&self, t: f64) -> Point3 {
        match self {
            Curve3Kind::Line3(l) => l.eval(t),
            Curve3Kind::CircularArc3(_) => todo!("CircularArc3::eval"),
            Curve3Kind::Nurbs(_) => todo!("NurbsCurve3::eval"),
            Curve3Kind::Ssi(_) => todo!("SsiCurve3::eval"),
        }
    }

    fn eval_dt(&self, t: f64) -> Point3 {
        match self {
            Curve3Kind::Line3(l) => l.eval_dt(t),
            Curve3Kind::CircularArc3(_) => todo!("CircularArc3::eval_dt"),
            Curve3Kind::Nurbs(_) => todo!("NurbsCurve3::eval_dt"),
            Curve3Kind::Ssi(_) => todo!("SsiCurve3::eval_dt"),
        }
    }

    fn is_degenerate(&self) -> bool {
        match self {
            Curve3Kind::Line3(l) => l.is_degenerate(),
            Curve3Kind::CircularArc3(_) => todo!("CircularArc3::is_degenerate"),
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
        Self { p0, p1, t_min: 0.0, t_max: 1.0 }
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

// ── Stub type for future Curve2Kind variant ───────────────────────────────────

/// A rational B-spline curve in UV space. Fields TBD — stub for `Curve2Kind`.
pub struct NurbsCurve2;

// ── Curve2Kind enum ───────────────────────────────────────────────────────────

/// The concrete stored pcurve type used in the B-rep arena.
pub enum Curve2Kind {
    Line2(Line2),
    Nurbs(NurbsCurve2),
}

impl Curve2 for Curve2Kind {
    fn eval(&self, t: f64) -> Point2 {
        match self {
            Curve2Kind::Line2(l) => l.eval(t),
            Curve2Kind::Nurbs(_) => todo!("NurbsCurve2::eval"),
        }
    }

    fn eval_dt(&self, t: f64) -> Point2 {
        match self {
            Curve2Kind::Line2(l) => l.eval_dt(t),
            Curve2Kind::Nurbs(_) => todo!("NurbsCurve2::eval_dt"),
        }
    }

    fn is_degenerate(&self) -> bool {
        match self {
            Curve2Kind::Line2(l) => l.is_degenerate(),
            Curve2Kind::Nurbs(_) => todo!("NurbsCurve2::is_degenerate"),
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

// ── Stub types for future SurfaceKind variants ────────────────────────────────

/// A cylindrical surface. Fields TBD — stub for `SurfaceKind`.
pub struct CylindricalSurface;

/// A conical surface. Fields TBD — stub for `SurfaceKind`.
pub struct ConicalSurface;

/// A spherical surface. Fields TBD — stub for `SurfaceKind`.
pub struct SphericalSurface;

/// A rational B-spline surface. Fields TBD — stub for `SurfaceKind`.
pub struct NurbsSurf;

// ── SurfaceKind enum ──────────────────────────────────────────────────────────

/// The concrete stored surface type used in the B-rep arena.
pub enum SurfaceKind {
    Plane(Plane),
    Cylinder(CylindricalSurface),
    Cone(ConicalSurface),
    Sphere(SphericalSurface),
    Nurbs(NurbsSurf),
}

impl Surface for SurfaceKind {
    fn eval(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval(u, v),
            SurfaceKind::Cylinder(_) => todo!("CylindricalSurface::eval"),
            SurfaceKind::Cone(_) => todo!("ConicalSurface::eval"),
            SurfaceKind::Sphere(_) => todo!("SphericalSurface::eval"),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval"),
        }
    }

    fn eval_du(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval_du(u, v),
            SurfaceKind::Cylinder(_) => todo!("CylindricalSurface::eval_du"),
            SurfaceKind::Cone(_) => todo!("ConicalSurface::eval_du"),
            SurfaceKind::Sphere(_) => todo!("SphericalSurface::eval_du"),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_du"),
        }
    }

    fn eval_dv(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval_dv(u, v),
            SurfaceKind::Cylinder(_) => todo!("CylindricalSurface::eval_dv"),
            SurfaceKind::Cone(_) => todo!("ConicalSurface::eval_dv"),
            SurfaceKind::Sphere(_) => todo!("SphericalSurface::eval_dv"),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_dv"),
        }
    }

    fn eval_n(&self, u: f64, v: f64) -> Option<Point3> {
        match self {
            SurfaceKind::Plane(p) => p.eval_n(u, v),
            SurfaceKind::Cylinder(_) => todo!("CylindricalSurface::eval_n"),
            SurfaceKind::Cone(_) => todo!("ConicalSurface::eval_n"),
            SurfaceKind::Sphere(_) => todo!("SphericalSurface::eval_n"),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_n"),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;

    // helpers
    fn p(x: f64, y: f64, z: f64) -> Point3 { Point3::new(x, y, z) }
    fn uv(u: f64, v: f64) -> Point2 { Point2::new(u, v) }
    fn approx_eq3(a: Point3, b: Point3) -> bool { (a - b).length() < 1e-14 }

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
}
