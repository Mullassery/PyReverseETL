"""PyReverseETL: The Open Operational Data Activation Runtime.

Re-exports the compiled Rust core's public classes so they're reachable as
`pyreverseetl.X` instead of requiring `pyreverseetl._core.X`.
"""

from ._core import (
    __version__,
    PyActivation,
    PyDestination,
    PyEntity,
    PySyncRun,
    PyWorkflow,
)

__all__ = [
    "__version__",
    "PyActivation",
    "PyDestination",
    "PyEntity",
    "PySyncRun",
    "PyWorkflow",
]
