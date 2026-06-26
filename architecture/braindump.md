# jefscad

## Main idea - 

I am working on a project for creating a code-based solid modeling language similar to 
OpenSCAD that I want to address 3 pain points I have with OpenSCAD

1. OpenSCAD meshes objects when they are created, which can lead to severe faceting when
   you apply non-uniform scaling. I would prefer a circle stays a circle (or ellipse if
   stretched) until its time to mesh the object.

2. OpenSCAD does not easily interface with other solid modeling programs as it doesn't
   support the STEP format. That makes using CNC services or working with others more
   difficult.

3. OpenSCAD will often display super thin left-overs, for example when doing a
   difference of a cylinder from a block where both are the same thickness, a super thin
   layer can be left behind due to floating point error. This can be overcome with
   adding/subtracting small epsilons, but it would be nicer if there we some idea of
   tolerance when doing binary operations for solids.

So I want to create my own solid modeling language based on constructive solid geometry
for creating solids, but instead of creating a mesh for each primative and then
combining the meshes, I want to instead create a boundary representation (b-rep) for the
solids with analytic surfaces. I would like to be able to export as STEP files directly
from the b-rep, and be able to specify a smoothness (size constraint on triangles or
angle constrains for non-crease dihedral angles) at meshing time instead of at solid 
creation time.

Finally, I want to leverage a full-fledged coding language - python - for CSG solid
authoring to allow things like functions or loops, instead of relying on a custom
domain-specific-language for those actions.

The project will be mostly written in Rust, using maturin and PyO3 to create a python
interface.

## Constructive Solid Geometry

Constructive solid geometry consist of taking 

* solid primitives: cuboids, cylinders, sphere, and cones, 

manipulating them with 

* transformations: translation, rotation, scaling, 

and then combining them with 

* boolean operations: union, difference, and intersection

So when creating a solid, you are essentially creating a tree structure, where each
internal node is a boolean operation, and each leaf node is primitive, and each node can
have an arbitrary number of transformations applied to it.

### 2D to 3D non-primitive leaf-nodes

In addition to the primitives, there are often two more types of leaf-node solids where
you take a shape in 2D and bring it into the 3D.

Extrusions: taking a closed path in a x-y 2D plane and extruding it linearly along the
z-axis

Solids of Rotation - open path: Take an open path in x-z 2D half-plane (positive x), and 
rotate that around path round the z-axis, extending the endpoints to the z-axis as 
circular caps.

Solids of Rotation - closed path: Take a closed path in the x-z 2D half-plane (positive
x), and rotate that path around the z-axis creating a 'donut' like shape.

Note: This program is a solid modeling program and does NOT support the idea of
infintely thin sheets.

#### Path

Now we have to construct paths in 2D to support the extrusion and solid-of-rotation
nodes. A path is an ordered list of segments, where the start-point of the next segment
is always the end-point of the previous. The usual segment types are: line, circular
arc, quadratic bezier, and cubic bezier.

### Selection operator

It is possible for the outcome of a binary operation to contain two or more disconnected
solids. A simple example is a long cylinder that is cut in two by a thin block leaving
the two disconnected ends of the cylinder as the result of the difference op. For
convience a 'selection' operator that allows you to choose a single solid from multiple
should also be supported. I _think_ a good rule for chosing a solid will be 'contains
point' because the outcome of boolean operations should never have overlapping solids.

## Boundary Representation

A boundary representation is a representation of a solid by modeling its boundary as 2D
surfaces in 3D space. In detail the representation has two parts: topological (which
parts are connect/contain others etc) and geometric (surfaces, curves, pcurves).

The b-rep structure that I have defined has:

Topological Structs

* SolidSet
  - topo: 
    - down: A collection of solids
* Solid 
  - topo: 
    - down: An outer shell and a (possibly zero-length) list of inner shells around voids
    - up: A solid set
* Shell 
  - topo: 
    - down: A collection of (face+face-sense)s, edges, and vertices
    - up: A solid
* Face 
  - topo: 
    - down: An outer edge-loop, and a list of inner edge-loops around holes
    - up: A shell
  - geom: A surface
* EdgeLoop
  - topo: 
    - down: An ordered list of co-edges
    - up: A face
* Coedge:
  - topo: 
    - down: An edge, an orientation
    - up: A face
  - geom: A pcurve
* Edge:
  - topo: 
    - down: A start vertex, end vertex
    - up: A set of co-edges, a set of faces
  - geom: A curve, and start and end t-values for start and end vertex
* Vertex:
  - topo:
    - up: A shell, optional faces, optional edges
  - geom: A point

Geometric Structs

* Surface - a uv->R^3 mapping
* PCurve - a t->uv mapping for given surface
* Curve - a t->R^3 mapping
* Point - a point in R^3

A single CSG node will be compiled into a SolidSet object, which will have reference to
all the other required structure for the node.

### Context

Each struct in the brep will be part of a context object with unique IDs for each
struct, and all connections between structs (downward, such as a SolidSet having
acollecitn of solids, or Shell having a collection of faces, edges, and vertices, or
upward such as A co-edge belonging to a face) will be through ID's. 

### Boolean Ops

Boolean ops with the b-reps are complicated, and will require the most care and tunine

Basics for intersection two solids A and B with ONLY outer shells
* find faces that intersect as pairs (f_a, f_b) one from each solid
* find the line of intersecton as new edges (curves) and coedges (pcurves)
* figure out how to split faces using new edges
  - option 1 - one edge-loop goes into two as face is split completely
  - option 2 - a new edge-loop is created making a hole in existing face
* figure how to to combine new faces with existing non-split faces to create new shells

#### Coincident faces

With appropriate tolerances, we need to deal with coincident faces during boolean
operations. I _think_ the natural behavior is:
* For difference operation with coincident faces with parallel normals: The overlapping
  section of the solids is completely removed
* For difference operation with coincident faces with perpendicular normals: The face
  from the base is left unchanged as if there is no overlap from the subtracted solid 
  for that face.
* For union operation with conincident faces with perpendicular normals: The solids are
  joined together and the overlapping face is removed since it is interior to the
  combined solid.

I _think_ this is the natural behavior to avoid super thin left-over slivers and allow
for unions of solids that touch but dont' necessarily inter-penitrate.

### Geometric functions (surfaces, curves, pcurves)

The geometric functions are structs that implement the traits for a surface, curve, or
pcurve.

* Surface: A surface is a function from a 2D uv space to R^3, and along with the
function should also carry a maximum domain as a rectangular region in uv space. The
surface should implement evaluatation and first derivatives `eval`, `du`, `dv`, `norm`

* Curve: A curve is a function from 1D t space to R^3, and should carry the maximum
domain as a finite range in t. The curve should implement evaluation and first
derivative `eval` and `dt`.

* PCurve: A pcurve is a function from 1D t space to the uv space for a given surface,
and should carry the maximum domain as a finite range in t. The pcurve should implement
evaluation and first derivative `eval` and `dt`.

Note: For an edge or face/coedge the curve and pcurve should correspond to the same
curve such that curve t-> point should be the same (to within tolerance) to pcurve t->
uv, surface(uv) -> point.

#### Surface types

There are various analytic surfaces supported

* Planar surface
* Clinderical surface
* Conical surface
* Spherical surface
* Ruled surface (extrusion along a quadratic/cubic bezier)
* Rotate surface (solid of rotation along a quadraic/cubic bezier)
* NURBS surface (spherical, cylindircal, conical surface with non-uniform scaling)

Although I would like to support NURBS surfaces to allow for non-uniform scaling of
analytic surfaces with circular components, I don't think I need to support the full
range of NURBS manipulations such as knot insertion or control-point or weight
manipulation. I just want NURBS surfaces as fallback for elliptical surfaces.

## Mesh

While a brep is an 'exact' description of a solid to some extent, it is less directly
useful for rendering or 3D printing, where a triangular mesh is prefered. To do generate
such a mesh, we will use the doubly-connected-edge-list data structure to allow us to
locally refine the mesh after initial meshing.

The data structure will contain
* HalfEdgeMesh
  - list of mesh-faces, half-edges, vertices
* MeshFace
  - a single half-edge
* HalfEdge
  - A vertex
  - A next half-edge
  - A twin half-edge
  - A mesh-face
  - A boolean if the edge is a constraint
* MeshVertex
  - A point
  - A uv coord
  - A normal
  - A ref to a b-rep owner (vertex or edge or face)

The idea is that the data-structure can be locally refined by adding new vertices along
and edge or in a face and recreating the local connections.

## Current progress

I have a partially completed project that allows authoring a CSG AST in python with
primitives, transformations, and boolean ops. I have extrusions and solids of rotation
implemented for paths that include lines and circular arcs only (no bezier curves yet).
I have most the b-rep data-structure implemented, and I have b-reps for all the
primitives already. I do NOT have boolean ops working for b-reps yet. I have the mesh
data structure implemented, and have initial meshes worked for the primitives. I do NOT
have any mesh manipulation methods or any mesh refinement.

## First goal

I want, as the first application, to be able to make a python program that will allow
you to make custom D&D style dice with custom fonts and symbols.

As a first step, I want to simply be able to make a render (and maybe 3D print) 
font-glyph that has been inset (with a boolean subtraction) from one surface of a 
cuboid.


