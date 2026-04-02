Concepts
========

.. note::
   This section is a work in progress and will be expanded as the project matures.

CSG model
---------

*To be written: what a CSG tree is, how nodes are shared via Arc, why immutability
matters for caching, and the difference between the authoring AST and the canonical
form used for hashing.*

Primitive coordinate conventions
---------------------------------

All primitives are defined at the origin with a consistent convention:

- **Sphere** — centered at the origin.
- **Cuboid** — one corner at the origin, the opposite corner at ``(dx, dy, dz)``.
- **Cylinder** — base circle at z = 0, centered at the origin; top at z = h.
- **Cone** — base circle at z = 0, centered at the origin; apex at z = h.

Use ``translate`` to position a primitive anywhere in space after construction.

Transform convention
--------------------

*To be written: right-hand rule for rotations, row-major 4×4 matrix, column-vector
convention, how transforms compose (right-multiply), and the flat_transform field.*

Geometry identity and caching
------------------------------

*To be written: prov_id vs geom_id, how geom_id is computed (canonical form +
structural hash), and how it can be used to avoid redundant evaluation.*

Boolean operations and selection
---------------------------------

*To be written: union/intersection/difference semantics, n-ary ops, Difference base
vs subtract ordering, and the Select op for picking connected components.*
