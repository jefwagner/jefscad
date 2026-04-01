"""
Regression tests for the jefscad Python bindings.

These tests confirm that the binding layer (pyo3 / py_bindings.rs) is wired
correctly. They deliberately avoid repeating the matrix-math assertions that
are already covered by the Rust unit tests in csg_lang.rs.
"""

import math
import jefscad


# ---------------------------------------------------------------------------
# Group 1 — Module surface
# ---------------------------------------------------------------------------

def test_module_exports_node_class():
    assert hasattr(jefscad, "Node")


def test_module_exports_constructors():
    for name in ("sphere", "cuboid", "cylinder", "cone"):
        assert callable(getattr(jefscad, name)), f"jefscad.{name} is not callable"


# ---------------------------------------------------------------------------
# Group 2 — Constructors return Node
# ---------------------------------------------------------------------------

def test_sphere_returns_node():
    assert isinstance(jefscad.sphere(1.0), jefscad.Node)


def test_cuboid_returns_node():
    assert isinstance(jefscad.cuboid(1.0, 2.0, 3.0), jefscad.Node)


def test_cylinder_returns_node():
    assert isinstance(jefscad.cylinder(1.0, 4.0), jefscad.Node)


def test_cone_returns_node():
    assert isinstance(jefscad.cone(0.5, 2.0), jefscad.Node)


# ---------------------------------------------------------------------------
# Group 3 — prov_id
# ---------------------------------------------------------------------------

def test_prov_id_is_int():
    assert isinstance(jefscad.sphere(1.0).prov_id, int)


def test_prov_ids_are_distinct():
    assert jefscad.sphere(1.0).prov_id != jefscad.sphere(1.0).prov_id


# ---------------------------------------------------------------------------
# Group 4 — Transforms return a new Node object
# ---------------------------------------------------------------------------

def test_translate_returns_node():
    assert isinstance(jefscad.sphere(1.0).translate(1.0, 0.0, 0.0), jefscad.Node)


def test_transform_returns_different_object():
    n = jefscad.sphere(1.0)
    assert n.translate(1.0, 0.0, 0.0) is not n


def test_original_prov_id_unchanged_after_transform():
    n = jefscad.sphere(1.0)
    pid = n.prov_id
    _ = n.translate(1.0, 0.0, 0.0)
    assert n.prov_id == pid


# ---------------------------------------------------------------------------
# Group 5 — Chaining
# ---------------------------------------------------------------------------

def test_chain_returns_node():
    result = (
        jefscad.cuboid(1.0, 1.0, 1.0)
        .translate(1.0, 0.0, 0.0)
        .rot_x(math.pi / 4)
    )
    assert isinstance(result, jefscad.Node)


def test_all_transform_methods_callable_and_return_node():
    n = jefscad.sphere(1.0)
    assert isinstance(n.translate(1.0, 0.0, 0.0), jefscad.Node)
    assert isinstance(n.scale(2.0, 2.0, 2.0), jefscad.Node)
    assert isinstance(n.rot_x(1.0), jefscad.Node)
    assert isinstance(n.rot_y(1.0), jefscad.Node)
    assert isinstance(n.rot_z(1.0), jefscad.Node)
    assert isinstance(n.rot_aa([0.0, 0.0, 1.0], 1.0), jefscad.Node)


# ---------------------------------------------------------------------------
# Group 6 — __repr__
# ---------------------------------------------------------------------------

def test_repr_sphere_contains_primitive_name_and_param():
    r = repr(jefscad.sphere(2.5))
    assert "sphere" in r.lower()
    assert "2.5" in r


def test_repr_cuboid_contains_primitive_name_and_dims():
    r = repr(jefscad.cuboid(1.0, 2.0, 3.0))
    assert "cuboid" in r.lower()
    assert "1.0" in r and "2.0" in r and "3.0" in r


def test_repr_fresh_node_shows_empty_transforms():
    r = repr(jefscad.sphere(1.0))
    assert "transforms" in r.lower()
    assert "[]" in r


def test_repr_after_translate_shows_transform():
    r = repr(jefscad.sphere(1.0).translate(0.0, 0.0, 1.5))
    assert "translat" in r.lower()  # matches both "translate" and "Translation"
    assert "1.5" in r


def test_repr_original_unchanged_after_transform():
    n = jefscad.sphere(1.0)
    _ = n.translate(0.0, 0.0, 1.5)
    assert "[]" in repr(n)  # original still has an empty transform stack
