"""
PyReverseETL: Python package

Rust-powered extension via PyO3/maturin.
"""

try:
    from ._core import *  # noqa: F401, F403
except ImportError:
    # Fallback for development/import errors
    pass

__version__ = "2.1.1"
__all__ = ['Reverse', 'ETL', 'activate']
