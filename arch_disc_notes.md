[question]

1. In openscad, I notice the preview and then pre-emptively do what you suggestd -
   define the cylinder as 10+2eps and translate it so that it subtracts properly.
2. In the future kernel, I plan on having the solid modeling step (after the CSG
   authoring step) as being represented by b-rep solids (set of faces -(a 2-D surface
   surrounded by a loop of 1-D edges) that enclose a volume)

To ask further questions - cause (at this point in time I want to focus on a discussion
so I can fully understand the situation so the design of the new solid modeling language
all makes sense to me). In the situation where we have coincident faces, can we not have
assumed meaning for given operations - difference means we should always remove the
faces, union means we combine and remove the new 'internal faces'? What are the other
situations with topological ambiguity?

[answer]

Yes—you can (and should) define intended semantics like “difference removes the cutter
volume” and “union removes internal faces”. The catch is that those high-level semantics
still don’t uniquely determine what to do in degenerate boundary configurations, because
the kernel has to answer lower-level questions like “is this point inside or outside?”
and “which side of this coincident face is ‘material’?” in order to build a valid
manifold boundary.

Below are the main ambiguity classes you’ll run into, why “just remove faces” isn’t
always sufficient, and what robust kernels typically do.

1) Coincident faces: why “difference removes faces” isn’t fully defined

The simple case (ideal intuition) Plate thickness 10, cutter cylinder height 10,
perfectly aligned so its caps coincide with the plate’s top/bottom faces. Intuitively:

Difference(plate, cylinder) should remove the hole cleanly. Any faces that end up
“internal” should be dropped.

Where ambiguity arises to know which faces are internal, the kernel must classify
boundary pieces. With coincident faces, many points lie on both boundaries
simultaneously, so the classification becomes “on boundary” rather than inside/outside.

Typical low-level steps in a B-rep boolean are:

* compute intersection curves between faces (to split faces into pieces)
* classify each face piece as inside/outside/on the other solid keep or discard pieces
  based on op (union/diff/intersection)
* sew the kept pieces into a closed shell

When faces are coincident:

the intersection between those faces is not a curve; it’s a 2D region (overlap area).
“split by intersection curves” doesn’t directly handle region-overlap without
special-case logic. classification of pieces gives “ON” over an area, not just along a
curve. So you need explicit rules like:

in UNION, if two faces are coincident and oppositely oriented, drop both (internal) if
coincident and same orientation, keep one (but which one? and with which
metadata/provenance?) in DIFFERENCE, drop the part of the minuend face that lies in the
overlap region, and possibly create trimming boundaries Even in that “keep/drop” logic,
you must be careful to keep the boundary manifold and watertight.

Bottom line: “difference removes faces” is not enough unless you also define how to
handle ON classifications and face-overlap regions.

2) Other common topological ambiguity situations (beyond coincident faces)

A) Tangential contact (touching without crossing)

Examples:

* a sphere just touches a plane (one point contact)
* a cylinder is tangent to a box face (line contact)
* two solids “kiss” at an edge

In these cases, the intersection is measure-zero (point/line), and the boolean result
depends on conventions:

does UNION create a single solid or leave them as two components touching? does
DIFFERENCE create a hole? (Usually no—because there’s no volume overlap.) do you keep
the touching point as part of the boundary? Many kernels regularize results to avoid
zero-thickness features, but you need to define it.

B) Nearly coincident / within tolerance (the practical version of coincidence)

Even if in math the faces are distinct, numerically they may be closer than your
tolerance. Then you must choose:

merge/snap them (treat as coincident) or keep them separate (risk sliver faces and
non-manifold edges) Your interval predicates help detect “uncertain” but won’t choose
the policy.

C) Edge-on-face / vertex-on-face degeneracy

An edge lies exactly on a face of the other solid, or a vertex lies on the other’s
edge/vertex.

This creates ambiguous intersection graphs:

Should that edge be considered part of the intersection set? does it split the face? how
do you build loops consistently when the intersection network contains “T-junctions”?

These cases often create non-manifold topology unless handled carefully.

D) Non-manifold intermediate results

Even if your inputs are valid solids, the intermediate splitting/sewing process can
create:

* edges with >2 incident faces
* dangling edges
* zero-area faces
* self-intersecting loops

The kernel must decide whether to:

* reject as invalid
* heal/regularize
* adjust within tolerance

E) Self-intersections and trimming loop ambiguity (esp. with NURBS)

Intersection curves on parametric surfaces can:

* loop back
* cross themselves in UV
* have multiple branches meeting at singularities

Building consistent face trims from these curves is a combinatorial/topological problem,
not just numeric.

3) So can we define a consistent “assumed meaning”? Yes: regularized boolean operations

What you’re describing is close to the standard concept of regularized set operations on
solids: Treat solids as regular closed sets. After applying set
union/difference/intersection, take the regularization (remove lower-dimensional
artifacts: isolated faces/edges/points with no volume).

In practice this means your boolean engine should:

* explicitly eliminate internal faces
* eliminate “dangling” zero-thickness sheets produced by coincidence/tangency
* merge near-coincident entities per tolerance

This gives users the behavior they expect (“no paper-thin leftovers”) but it requires:

* a tolerance model
* explicit ON-case policies
* robust “is this boundary piece internal/external?” classification

4) Concrete policy knobs to consider in your language/kernel (to make behavior 
   predictable)

Overshoot policy for cutters (authoring convenience)

If user asks for a through-hole, automatically extend the cutter beyond the target
bounds. This avoids exact coincidence in the common case. ON-classification rule When
classifying a face fragment against the other solid, if result is “ON”:

* decide based on operation + orientation (keep one side, drop both, etc.)
* decide which metadata/provenance wins if you keep one

Regularization / cleanup pass

* delete zero-area faces, zero-length edges
* merge vertices within tolerance
* collapse sliver faces below area threshold
* ensure 2-manifold boundary for a solid result (or report a specific error)

These are the real “meaning” choices that determine user experience.

5) Answering your direct question: “what are the other situations with topological 
   ambiguity?”

If I had to give the shortest list:

* Coincident face overlap regions (2D intersection, not curve)
* Tangency (intersection of lower dimension: point/line)
* Edge/vertex lying on the other boundary (T-junction style degeneracy)
* Near-coincidence within tolerance (policy decision)
* Intersection curve networks with branch points / self-crossings (trim construction ambiguity)

[question]

I appreciate this explanation - next question: You advise using f64 for faster
calculations (which is VERY reasonable - part is part of why I've tried to write the
rounded floating point interval crate to support SIMD ops), how would that work. If I
want to use rounded floating point intervals to determine cases of indeterminacy, wont I
have to use those same intervals for all mathematical operations (creation ->
transforms(translations,rotations,etc) -> NURBS calculations etc..) in order to properly
ask the "could these faces be coincident"? 

[answer]

You don’t have to use interval arithmetic for all operations to get the “indeterminacy
detection” benefit. The usual robust-kernel pattern is:

do most computations in f64 (fast, SIMD-friendly), then use interval arithmetic
selectively as a verification/filter step for predicates and “are we near-degenerate?”
questions, and if the filter says “uncertain”, trigger refinement (subdivide, increase
precision, tighten bounds, rerun a local solve). This is often called a filtered or
certified computation strategy.

The key is that the question “could these faces be coincident?” is not a single numeric
computation—it’s a predicate (or a set of predicates), and predicates can be verified
without redoing every upstream calculation in intervals.

Below are concrete ways to make that work.

1) What you actually need intervals for: predicates, not full constructions

Most degeneracy decisions boil down to a small set of predicate types:

* sign of a value (distance to plane, point-side-of-surface, dot product sign)
* “is this within tolerance?” comparisons
* bounding overlap tests (AABB / OBB / patch bounds)
* “do these two parametric patches definitely not intersect?”

Intervals shine here because they can return:

* definitely true
* definitely false
* unknown

You do not need interval arithmetic for every vector/matrix/NURBS evaluation to gain
this; you need interval arithmetic for the predicate inputs with guaranteed bounds.

2) The practical trick: attach bounds to geometry, don’t interval-evaluate everything

A) Carry conservative bounds for entities

For each geometric entity (curve segment, surface patch, face), maintain an AABB that is
guaranteed to enclose it.

Compute bounds quickly in f64 then inflate them conservatively by a tolerance and/or an
interval-derived bound

Example:

A surface patch bounding box computed via control points (NURBS has a convex hull
property in many representations) can be made conservative without interval evaluation
of the surface at all.

Then your first degeneracy/coincedence test becomes:

* if AABBs are disjoint → definitely not coincident/intersecting
* if they overlap → maybe; proceed

This handles a lot of cases cheaply.

B) Use interval arithmetic at the predicate point

When you need to decide if two faces are (near-)coplanar/coincident, you typically end
up evaluating something like: for planar faces: distance of vertices of face A to plane
of face B for NURBS: distance between two surfaces over a region, or at sample points,
or via intersection solver residuals

You can evaluate those residuals with intervals, not everything.

3) Example: “are these planar faces coincident?”

You can do:

In f64:

* compute plane equation for face B: n·x + d
* compute distances for a few key points from face A (or all vertices)
  - if all distances are clearly within tolerance → likely coplanar
  - if clearly outside → not coplanar
* If close to tolerance:
  - re-evaluate n·x + d using interval arithmetic just for that expression
    - n, x, d can be represented as small intervals: n ± εn, x ± εx, d ± εd
    - or use your “grow by 1 ulp” interval ops to bound rounding
  - if interval result is strictly outside tolerance → definitely not coplanar
  - if interval result is strictly inside tolerance → definitely coplanar (within your
    model tolerance)
  - else → uncertain → handle with a robust policy (treat as coincident, subdivide, or 
    escalate)

Notice: you didn’t need interval transforms everywhere—just for the distance predicate
when it matters.

4) For NURBS: you usually need bounds + adaptive refinement anyway

NURBS intersection is typically iterative/subdivision-based. Intervals can help a lot if
you use them where they’re strongest:

A) Patch bounding via control net

A classic approach:

* represent a surface region as a patch (subset of knot spans)
* compute a conservative bounding box from its control points (often good enough)
  - if two patch bounds don’t overlap → no intersection; stop
  - if overlap and patch is “too big” → subdivide patches and repeat
* when small enough, run local solver (Newton) in f64

Intervals can refine the bounds or certify separation, but the core strategy is
subdivision + bounding. This avoids having intervals explode inside Newton iterations.

B) Interval arithmetic inside Newton often blows up

This is exactly the “intervals grow fast” issue:

Newton updates depend on derivatives and division by Jacobians; uncertainty magnifies. 
So the common strategy is:

* run Newton in f64 to propose a solution
* use interval/predicate checks to validate the solution (residual bounds, containment
  in parameter domain, etc.)
  - if validation fails/uncertain, subdivide or increase precision

5) Answering your core worry directly

“If I want intervals to detect indeterminacy, don’t I need intervals for creation →
transforms → NURBS calculations to properly ask ‘could these faces be coincident’?”

No, not generally. You need guaranteed bounds on the quantities involved in the
coincidence test. There are multiple ways to obtain those bounds:

* cheap conservative geometry bounds (control points / AABBs / inflated tolerances)
* interval evaluation of a local predicate (distance/residual at a point)
* certified separation tests (interval dot products, etc.)
* refinement when uncertain (subdivide patches, sample more points, tighten bounds)

You only need “interval everywhere” if your only way to get a bound is to
interval-evaluate the entire pipeline, which is usually unnecessary and often
impractical.

6) A good architecture for your numeric types

Use two scalars in the kernel:

* Real = f64 for geometry storage + fast evaluation
* Bounded = Interval<f64> (or your rounded-interval type) for filters/predicates

Then design APIs like:

* fn eval_surface(&self, u: f64, v: f64) -> Vec3 (fast)
* fn eval_surface_bounds(&self, u: Interval, v: Interval) -> AabbInterval (only when 
  needed)
* fn point_on_side_filter(...) -> Certainty returning True/False/Unknown
* This gives you performance and robustness without committing to interval computations 
  everywhere.

7) Practical note: your “1 ulp growth per op” is still useful

Even if you use intervals only in filters, having intervals that rigorously bound
roundoff makes your “Unknown” detection meaningful and trustworthy.

The limitation is: if you try to use that same interval type for long chains of
nonlinear computations, you’ll get “Unknown” too often. Used as a filter, that’s fine:
“Unknown” just triggers refinement, not failure.


