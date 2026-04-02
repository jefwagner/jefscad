Getting started
===============

Installation
------------

Prerequisites: Python ≥ 3.9, Rust nightly, and `uv`.

.. code-block:: bash

   # Clone the repo, then from the repo root:
   uv venv .venv
   uv pip install --python .venv/bin/python maturin pytest jupyterlab ipykernel
   source .venv/bin/activate
   maturin develop --features extension-module

After this, ``import jefscad`` works in any Python process that uses the virtualenv.
See ``DEVELOPMENT.md`` for the full environment reference.

First shapes
------------

Import the package and create a few primitives:

.. code-block:: python

   import jefscad

   # A sphere of radius 1, centered at the origin
   ball = jefscad.sphere(1.0)

   # A flat cuboid: corner at origin, opposite corner at (4, 4, 0.5)
   base = jefscad.cuboid(4.0, 4.0, 0.5)

Transforms
----------

Transforms return a **new** node — the original is unchanged:

.. code-block:: python

   # Lift the ball so it sits on top of the base
   ball_raised = ball.translate(0, 0, 1.5)

   # Rotate 45° around the Z axis
   import math
   ball_rotated = ball.rot_z(math.pi / 4)

   # Chain transforms
   piece = jefscad.cone(0.5, 1.0).rot_x(math.pi).translate(0, 0, 2.5)

Boolean operations
------------------

Combine shapes with ``union``, ``intersection``, and ``difference``:

.. code-block:: python

   # Ball sitting on a pedestal
   model = jefscad.union(base, ball_raised)

   # Hollow out the base with a smaller box
   cutout = jefscad.cuboid(3.0, 3.0, 0.4).translate(0.5, 0.5, 0.05)
   hollow_base = jefscad.difference(base, cutout)

   # Only the overlapping volume
   overlap = jefscad.intersection(ball, jefscad.cuboid(1.5, 1.5, 1.5))

Inspecting the tree
-------------------

Every node has a ``__str__`` that prints the CSG tree:

.. code-block:: python

   print(jefscad.union(base, ball_raised))
   # union
   # ├── cuboid(dx=4, dy=4, dz=0.5)
   # └── sphere(r=1)
   #     └── transforms
   #         └── translate(dx=0, dy=0, dz=1.5)

Two identity properties are available on every node:

.. code-block:: python

   a = jefscad.sphere(1.0).translate(0, 0, 1.0)
   b = jefscad.sphere(1.0).translate(0, 0, 1.0)

   a.prov_id   # unique per construction call — a != b
   a.geom_id   # structural hash — a.geom_id == b.geom_id
