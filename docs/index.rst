jefscad
=======

A constructive solid geometry (CSG) modeling language exposed as a Python package,
with a high-performance Rust core.

Shapes are built by combining **solid primitives** (spheres, cuboids, cylinders, cones)
with **boolean operations** (union, intersection, difference) and **affine transforms**
(translate, rotate, scale). All objects are immutable — every operation returns a new
node, so you can freely branch and reuse shapes without copying.

.. code-block:: python

   import jefscad

   pedestal = jefscad.cuboid(2.0, 2.0, 0.5)
   ball     = jefscad.sphere(0.8).translate(0, 0, 1.3)
   model    = jefscad.union(pedestal, ball)
   print(model)

.. toctree::
   :maxdepth: 2
   :caption: Contents

   getting_started
   concepts
   api
