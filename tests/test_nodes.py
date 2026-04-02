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


# ---------------------------------------------------------------------------
# Group 7 — geom_id property
# ---------------------------------------------------------------------------

def test_geom_id_is_int():
    assert isinstance(jefscad.sphere(1.0).geom_id, int)


def test_geom_id_same_primitive_same_id():
    assert jefscad.sphere(1.0).geom_id == jefscad.sphere(1.0).geom_id


def test_geom_id_different_params_different_id():
    assert jefscad.sphere(1.0).geom_id != jefscad.sphere(2.0).geom_id


def test_geom_id_differs_from_prov_id():
    # geom_id is geometry-based; prov_id is instance-unique
    n = jefscad.sphere(1.0)
    m = jefscad.sphere(1.0)
    assert n.prov_id != m.prov_id
    assert n.geom_id == m.geom_id


# ---------------------------------------------------------------------------
# Group 8 — Op constructors
# ---------------------------------------------------------------------------

def test_module_exports_ops():
    for name in ("union", "intersection", "difference",
                 "select_largest", "select_closest_to", "select_contains"):
        assert callable(getattr(jefscad, name)), f"jefscad.{name} is not callable"


def test_union_returns_node():
    a, b = jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)
    assert isinstance(jefscad.union(a, b), jefscad.Node)


def test_union_repr_contains_op_name():
    r = repr(jefscad.union(jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)))
    assert "union" in r.lower()


def test_union_geom_id_is_order_independent():
    a, b = jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)
    assert jefscad.union(a, b).geom_id == jefscad.union(b, a).geom_id


def test_union_raises_on_no_args():
    try:
        jefscad.union()
        assert False, "expected an exception"
    except (ValueError, TypeError):
        pass


def test_intersection_returns_node():
    a, b = jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)
    assert isinstance(jefscad.intersection(a, b), jefscad.Node)


def test_intersection_geom_id_is_order_independent():
    a, b = jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)
    assert jefscad.intersection(a, b).geom_id == jefscad.intersection(b, a).geom_id


def test_union_and_intersection_have_different_geom_id():
    a, b = jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)
    assert jefscad.union(a, b).geom_id != jefscad.intersection(a, b).geom_id


def test_difference_returns_node():
    base = jefscad.cuboid(4.0, 4.0, 4.0)
    hole = jefscad.sphere(1.0)
    assert isinstance(jefscad.difference(base, hole), jefscad.Node)


def test_difference_repr_contains_op_name():
    r = repr(jefscad.difference(jefscad.cuboid(4.0, 4.0, 4.0), jefscad.sphere(1.0)))
    assert "difference" in r.lower()


def test_difference_base_order_matters_for_geom_id():
    a, b = jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)
    assert jefscad.difference(a, b).geom_id != jefscad.difference(b, a).geom_id


def test_difference_raises_on_missing_subtract():
    try:
        jefscad.difference(jefscad.sphere(1.0))
        assert False, "expected an exception"
    except (ValueError, TypeError):
        pass


def test_select_largest_returns_node():
    assert isinstance(jefscad.select_largest(jefscad.sphere(1.0)), jefscad.Node)


def test_select_closest_to_returns_node():
    assert isinstance(
        jefscad.select_closest_to(jefscad.sphere(1.0), [0.0, 0.0, 0.0]),
        jefscad.Node,
    )


def test_select_contains_returns_node():
    assert isinstance(
        jefscad.select_contains(jefscad.sphere(1.0), [0.0, 0.0, 0.0]),
        jefscad.Node,
    )


# ---------------------------------------------------------------------------
# Group 9 — __str__ (tree display)
# ---------------------------------------------------------------------------

def test_str_differs_from_repr():
    n = jefscad.sphere(1.0)
    assert str(n) != repr(n)


def test_str_sphere_header():
    assert str(jefscad.sphere(2.5)).splitlines()[0] == "sphere(r=2.5)"


def test_str_primitive_no_transforms_is_single_line():
    assert len(str(jefscad.sphere(1.0)).splitlines()) == 1


def test_str_translate_shows_transforms_branch():
    s = str(jefscad.sphere(1.0).translate(0.0, 0.0, 1.5))
    assert "transforms" in s
    assert "translate" in s


def test_str_four_transforms_collapsed():
    n = (jefscad.sphere(1.0)
         .translate(1.0, 0.0, 0.0)
         .scale(2.0, 2.0, 2.0)
         .rot_z(1.0)
         .translate(0.0, 1.0, 0.0))
    s = str(n)
    assert "transforms[4]" in s
    assert "translate(" not in s


def test_str_union_header_and_children():
    s = str(jefscad.union(jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)))
    lines = s.splitlines()
    assert lines[0] == "union"
    assert any("sphere" in l for l in lines)
    assert any("cuboid" in l for l in lines)


def test_str_difference_has_base_and_subtract():
    s = str(jefscad.difference(jefscad.cuboid(4.0, 4.0, 4.0), jefscad.sphere(1.0)))
    assert "base" in s
    assert "subtract" in s


def test_str_tree_connectors_present():
    s = str(jefscad.union(jefscad.sphere(1.0), jefscad.cuboid(2.0, 2.0, 2.0)))
    assert "├──" in s
    assert "└──" in s
