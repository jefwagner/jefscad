# Notes and ToDo for jefscad

jefscad should eventual be a constructive solid geometry language similar to openscad
that is exposed as a python package.

## Hello-world style first implementation

The first step is to scaffold the project and build/testing tools to create a rust
backed local python package that can be loaded into a python environment (or jupyter
notebook) in editable mode with pip install -e (or the UV equivalent).

[x] Document the environmental setup for making a rust backed python package
[x] Create simple HelloWorld rust struct with new that takes a string
[x] Create a method for the HelloWorld struct that prints a greeting with the string
[x] Use pyo3 macros to expose HelloWorld struct and method to python
[x] Scaffold the setup to create the rust backed python library
[x] Create a simple python unit test (using pytest package)
[x] Create a jupyter notebook the demos the use of the python package

See [DEVELOPMENT.md](../DEVELOPMENT.md) for environment setup, build commands,
running tests, and the Jupyter workflow.

## Construct Solid Geometry Layer

[_] Design/Architect the constructive solid geometry (csg) layer in rust

Notes for the above todo item:
  * A CsgNode is a struct with { inner: CsgNodeBase, transforms: Vec<AffineTransform> ,
  meta: CsgMetadata}
  * A CsgNodeBase is an enum with:
    - Op(CsgOp) where CsgOp is an enum with:
      - Union
      - Difference
      - Intersection
      - Selection -> select a single solid from multiple disjoint solids using a ranking
        function/method.
    - Node(CsgNode) allows for reference of nodes with only some transformations applied
    - Prim(CsgPrimative) where CsgPrimative is an enum with:
      - Cuboid
      - Cylinder
      - Sphere
      - Cone
      - Extrusion(ClosedPath2d, length)
      - SolidOfRotation(Path2d)
  * AffineTransform is an enum with:
    - Generic(matrix) a generic affine transformation given by a 3x4 matrix
    - Translation(delta) or Move(delta) -> A translation by the vector delta
    - RotationAA(axis, angle) -> A rotation around axis by angle
    - Scale(sx, sy, sz) axis-alligned non-uniform scaling
    - Shear(to-be-determinied) A shear transformation
  * CsgMetadata - additional data for a CsgNode, right now the only things that
    come to mind are a color for rendering, or optional material specifier. Maybe
    some texture info?

A CSG tree is traditionally made with the operators (union, difference, intersection) as
internal nodes, and leaf nodes being the solid primatives. But there are several
questions I have about this:

1. Can this system work without needing a context or global workspace? Ergonomically
I would prefer not needing to keep global variables in my head while designing, but
it somehow feels like this will be a necessary evil... we'll see. If so, I we night have
to add a CsgContext or CsgWorkspace struct to the system.

2. In rust this raises some questions about ownership of nodes - does a
internal node own it's children? If not, who does own the children, and if so, how would
we have a separate inner node that uses the same child node for a different operation
(say we want to subract a the same sphere from multiple different cubes)? Right now I'm
leaning towards No - an operator just gets an Arc ref to the child nodes.

3. Will the operators be strictly binary operators, or do we allow them to
have multiple inputs? I'm leaning towards allowing multiple inputs - at least for order
independent unions and intersections -  because it's logically equivalent, and it's nice
when the data structure reflects reality.

4. In a perfect world, a given node in a CSG tree would correspond to a
single connected solid, but in reality that is often not the case. I would like to be
able to have some method of choosing a single solid from set of disconnected solids
from a single node in a simple to understand and easy to use manor. How might one create
some selection operation like this?

