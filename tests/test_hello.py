"""
Tests for the jefscad HelloWorld demo and the add function.
Run with: pytest   (from the repo root, with the venv active)
"""

import jefscad
from jefscad import HelloWorld, add


class TestHelloWorld:
    def test_greet_basic(self):
        hw = HelloWorld("Alice")
        assert hw.greet() == "Hello, Alice!"

    def test_greet_different_name(self):
        hw = HelloWorld("World")
        assert hw.greet() == "Hello, World!"

    def test_greet_empty_name(self):
        hw = HelloWorld("")
        assert hw.greet() == "Hello, !"

    def test_multiple_instances_independent(self):
        a = HelloWorld("Alice")
        b = HelloWorld("Bob")
        assert a.greet() != b.greet()


class TestAdd:
    def test_add_positive(self):
        assert add(2, 3) == 5

    def test_add_zero(self):
        assert add(0, 0) == 0

    def test_add_negative(self):
        assert add(-1, 1) == 0

    def test_add_large(self):
        assert add(10**9, 10**9) == 2 * 10**9


class TestModuleStructure:
    def test_module_has_hello_world(self):
        assert hasattr(jefscad, "HelloWorld")

    def test_module_has_add(self):
        assert hasattr(jefscad, "add")
