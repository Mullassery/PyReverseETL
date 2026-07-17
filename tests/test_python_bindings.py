"""Test Python bindings for PyReverseETL Rust core."""

import pytest


def test_pyreverseetl_import():
    """Verify Python bindings are accessible."""
    try:
        import pyreverseetl
        assert pyreverseetl is not None
    except ImportError:
        pytest.skip("pyreverseetl bindings not built yet (run maturin develop)")


def test_pyreverseetl_version():
    """Verify version is set."""
    try:
        import pyreverseetl
        assert hasattr(pyreverseetl, "__version__")
    except ImportError:
        pytest.skip("pyreverseetl bindings not built yet")
