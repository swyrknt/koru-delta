"""Sphinx configuration for KoruDelta Python documentation."""

import os
import sys

# Add the parent directory to the path so Sphinx can find the module
sys.path.insert(0, os.path.abspath('..'))

# Project information
project = 'KoruDelta'
copyright = '2026, KoruDelta Contributors'
author = 'KoruDelta Contributors'
version = '3.0.0'
release = '3.0.0'

# General configuration
extensions = [
    'sphinx.ext.autodoc',
    'sphinx.ext.viewcode',
    'sphinx.ext.napoleon',
    'sphinx.ext.intersphinx',
    'sphinx_rtd_theme',
]

templates_path = ['_templates']
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']

# HTML output options
html_theme = 'sphinx_rtd_theme'
html_static_path = ['_static']

# Autodoc settings
autodoc_member_order = 'bysource'
autodoc_typehints = 'description'

# Intersphinx mapping
intersphinx_mapping = {
    'python': ('https://docs.python.org/3', None),
}

# Napoleon settings for Google/NumPy style docstrings
napoleon_google_docstring = True
napoleon_numpy_docstring = True
