# Configuration file for the Sphinx documentation builder.
# Build with: sphinx-build -b html docs/ docs/_build/html/

import os
import sys

# Point autodoc at the Python source tree so it can import jefscad.
# The compiled Rust extension must be built first: maturin develop --features extension-module
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'python')))

# ---------------------------------------------------------------------------
# Project metadata
# ---------------------------------------------------------------------------

project = 'jefscad'
copyright = '2025, Jef Wagner'
author = 'Jef Wagner'
release = '0.1.0'

# ---------------------------------------------------------------------------
# Extensions
# ---------------------------------------------------------------------------

extensions = [
    'sphinx.ext.autodoc',    # pull docstrings from the compiled extension
    'sphinx.ext.napoleon',   # parse Google-style Args: / Returns: blocks
]

# autodoc options
autodoc_member_order = 'bysource'   # keep the authoring order from __init__.py
autodoc_typehints = 'signature'     # show types in the function signature line

# napoleon options — we use Google style; NumPy style not needed
napoleon_google_docstring = True
napoleon_numpy_docstring = False
napoleon_use_param = False          # don't emit :param: tags (we use inline Args:)
napoleon_use_rtype = False          # don't emit :rtype: tags

# ---------------------------------------------------------------------------
# HTML output
# ---------------------------------------------------------------------------

html_theme = 'furo'
html_title = 'jefscad'
