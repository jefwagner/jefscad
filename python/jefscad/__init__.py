"""
jefscad — solid modeling language.

The core implementation is in Rust (via pyo3). This module re-exports
everything from the compiled Rust extension (jefscad._jefscad) and is
the place to add pure-Python helpers, submodules, or documentation
augmentations on top of the Rust layer.
"""

from ._jefscad import HelloWorld, add  # noqa: F401

__all__ = ["HelloWorld", "add"]
