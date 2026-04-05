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
    pub center:  Point3,
    pub normal:  Point3,   // unit; defines the circle plane via right-hand rule
    pub ref_dir: Point3,   // unit, ⊥ normal; direction at t = 0
    pub radius:  f64,
    pub t0:      f64,      // start angle (radians)
    pub t1:      f64,      // end angle (radians); t1 > t0; t1 - t0 = 2π for full circle
}

impl CircularArc3 {
    pub fn new(
        center: Point3, normal: Point3, ref_dir: Point3,
        radius: f64, t0: f64, t1: f64,
    ) -> Self {
        Self { center, normal, ref_dir, radius, t0, t1 }
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
            Curve3Kind::CircularArc3(a) => a.eval(t),
            Curve3Kind::Nurbs(_) => todo!("NurbsCurve3::eval"),
            Curve3Kind::Ssi(_) => todo!("SsiCurve3::eval"),
        }
    }

    fn eval_dt(&self, t: f64) -> Point3 {
        match self {
            Curve3Kind::Line3(l) => l.eval_dt(t),
            Curve3Kind::CircularArc3(a) => a.eval_dt(t),
            Curve3Kind::Nurbs(_) => todo!("NurbsCurve3::eval_dt"),
            Curve3Kind::Ssi(_) => todo!("SsiCurve3::eval_dt"),
        }
    }

    fn is_degenerate(&self) -> bool {
        match self {
            Curve3Kind::Line3(l) => l.is_degenerate(),
            Curve3Kind::CircularArc3(a) => a.is_degenerate(),
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

// ── CylindricalSurface ────────────────────────────────────────────────────────

/// An infinite cylindrical surface.
///
/// Parameterization: `u = angle ∈ [0, 2π)`, `v = height along axis`.
/// Frame in the cross-section plane: `ref_dir` at `u = 0`, `axis × ref_dir` at `u = π/2`.
/// The outward normal at `(u, v)` is `cos(u)*ref_dir + sin(u)*(axis × ref_dir)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylindricalSurface {
    pub origin:  Point3,   // center of the base circle; eval(u, 0) lies on this circle
    pub axis:    Point3,   // unit; direction of increasing v
    pub ref_dir: Point3,   // unit, ⊥ axis; u = 0 meridian
    pub radius:  f64,
}

impl CylindricalSurface {
    pub fn new(origin: Point3, axis: Point3, ref_dir: Point3, radius: f64) -> Self {
        Self { origin, axis, ref_dir, radius }
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
    pub apex:       Point3,  // singular point; eval(u, 0) = apex for all u
    pub axis:       Point3,  // unit; direction of increasing v (toward base)
    pub ref_dir:    Point3,  // unit, ⊥ axis; u = 0 meridian
    pub half_angle: f64,     // 0 < half_angle < π/2 (radians)
}

impl ConicalSurface {
    pub fn new(apex: Point3, axis: Point3, ref_dir: Point3, half_angle: f64) -> Self {
        Self { apex, axis, ref_dir, half_angle }
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
    pub center:  Point3,
    pub radius:  f64,
    pub ref_dir: Point3,   // unit; u = 0, v = 0 reference direction
    pub axis:    Point3,   // unit, ⊥ ref_dir; north pole at v = +π/2
}

impl SphericalSurface {
    pub fn new(center: Point3, radius: f64, ref_dir: Point3, axis: Point3) -> Self {
        Self { center, radius, ref_dir, axis }
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

// ── Stub type for remaining SurfaceKind variant ───────────────────────────────

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
            SurfaceKind::Cylinder(c) => c.eval(u, v),
            SurfaceKind::Cone(c) => c.eval(u, v),
            SurfaceKind::Sphere(s) => s.eval(u, v),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval"),
        }
    }

    fn eval_du(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval_du(u, v),
            SurfaceKind::Cylinder(c) => c.eval_du(u, v),
            SurfaceKind::Cone(c) => c.eval_du(u, v),
            SurfaceKind::Sphere(s) => s.eval_du(u, v),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_du"),
        }
    }

    fn eval_dv(&self, u: f64, v: f64) -> Point3 {
        match self {
            SurfaceKind::Plane(p) => p.eval_dv(u, v),
            SurfaceKind::Cylinder(c) => c.eval_dv(u, v),
            SurfaceKind::Cone(c) => c.eval_dv(u, v),
            SurfaceKind::Sphere(s) => s.eval_dv(u, v),
            SurfaceKind::Nurbs(_) => todo!("NurbsSurf::eval_dv"),
        }
    }

    fn eval_n(&self, u: f64, v: f64) -> Option<Point3> {
        match self {
            SurfaceKind::Plane(p) => p.eval_n(u, v),
            SurfaceKind::Cylinder(c) => c.eval_n(u, v),
            SurfaceKind::Cone(c) => c.eval_n(u, v),
            SurfaceKind::Sphere(s) => s.eval_n(u, v),
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

    // ── CircularArc3 ──────────────────────────────────────────────────────────
    //
    // Canonical frame: center=(0,0,0), normal=(0,0,1), ref_dir=(1,0,0) → ê₂=(0,1,0)

    fn std_arc(radius: f64) -> CircularArc3 {
        CircularArc3::new(p(0.0,0.0,0.0), p(0.0,0.0,1.0), p(1.0,0.0,0.0), radius, 0.0, 2.0*std::f64::consts::PI)
    }

    #[test]
    fn arc3_eval_at_zero() {
        let a = std_arc(3.0);
        assert!(approx_eq3(a.eval(0.0), p(3.0, 0.0, 0.0)));
    }

    #[test]
    fn arc3_eval_at_half_pi() {
        let a = std_arc(3.0);
        assert!(approx_eq3(a.eval(std::f64::consts::FRAC_PI_2), p(0.0, 3.0, 0.0)));
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
        assert!(approx_eq3(a.eval_dt(std::f64::consts::FRAC_PI_2), p(-2.0, 0.0, 0.0)));
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
        assert!(approx_eq3(ck.eval(std::f64::consts::FRAC_PI_2), a.eval(std::f64::consts::FRAC_PI_2)));
    }

    // ── CylindricalSurface ────────────────────────────────────────────────────
    //
    // Canonical: origin=(0,0,0), axis=(0,0,1), ref_dir=(1,0,0), radius=2

    fn std_cyl() -> CylindricalSurface {
        CylindricalSurface::new(p(0.0,0.0,0.0), p(0.0,0.0,1.0), p(1.0,0.0,0.0), 2.0)
    }

    #[test]
    fn cyl_eval_at_u0_v0() {
        assert_eq!(std_cyl().eval(0.0, 0.0), p(2.0, 0.0, 0.0));
    }

    #[test]
    fn cyl_eval_at_half_pi_v0() {
        assert!(approx_eq3(std_cyl().eval(std::f64::consts::FRAC_PI_2, 0.0), p(0.0, 2.0, 0.0)));
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
        assert!(approx_eq3(std_cyl().eval_n(0.0, 0.0).unwrap(), p(1.0, 0.0, 0.0)));
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
        ConicalSurface::new(p(0.0,0.0,0.0), p(0.0,0.0,1.0), p(1.0,0.0,0.0), std::f64::consts::FRAC_PI_4)
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
        SphericalSurface::new(p(0.0,0.0,0.0), 3.0, p(1.0,0.0,0.0), p(0.0,0.0,1.0))
    }

    #[test]
    fn sphere_eval_equatorial_ref() {
        assert!(approx_eq3(std_sphere().eval(0.0, 0.0), p(3.0, 0.0, 0.0)));
    }

    #[test]
    fn sphere_eval_north_pole() {
        assert!(approx_eq3(std_sphere().eval(0.0, std::f64::consts::FRAC_PI_2), p(0.0, 0.0, 3.0)));
    }

    #[test]
    fn sphere_eval_south_pole() {
        assert!(approx_eq3(std_sphere().eval(0.0, -std::f64::consts::FRAC_PI_2), p(0.0, 0.0, -3.0)));
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
        assert!(approx_eq3(std_sphere().eval_n(0.0, 0.0).unwrap(), p(1.0, 0.0, 0.0)));
    }

    #[test]
    fn sphere_eval_n_north_pole() {
        // normal at north pole = axis = (0,0,1), independent of u
        let n0 = std_sphere().eval_n(0.0, std::f64::consts::FRAC_PI_2).unwrap();
        let n1 = std_sphere().eval_n(1.234, std::f64::consts::FRAC_PI_2).unwrap();
        assert!(approx_eq3(n0, p(0.0, 0.0, 1.0)));
        assert!(approx_eq3(n1, p(0.0, 0.0, 1.0)));
    }

    #[test]
    fn sphere_eval_n_south_pole() {
        let n = std_sphere().eval_n(0.5, -std::f64::consts::FRAC_PI_2).unwrap();
        assert!(approx_eq3(n, p(0.0, 0.0, -1.0)));
    }

    #[test]
    fn sphere_eval_n_always_some() {
        assert!(std_sphere().eval_n(0.0, 0.0).is_some());
        assert!(std_sphere().eval_n(0.0, std::f64::consts::FRAC_PI_2).is_some());
        assert!(std_sphere().eval_n(0.0, -std::f64::consts::FRAC_PI_2).is_some());
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
}
