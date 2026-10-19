"""PyReverseETL: The Open Operational Data Activation Runtime.

Re-exports the compiled Rust core's public classes so they're reachable as
`pyreverseetl.X` instead of requiring `pyreverseetl._core.X`.
"""

from ._core import (
    PyActivation,
    PyDestination,
    PyEntity,
    PySyncResult,
    PySyncRun,
    PyWorkflow,
    __version__,
    lineage_dot,
    lineage_json,
    run_sync,
)
from .singer import SingerBatch, SingerSchema, jsonl_sink, parse_messages, run_target

__all__ = [
    "PyActivation",
    "PyDestination",
    "PyEntity",
    "PySyncResult",
    "PySyncRun",
    "PyWorkflow",
    "SingerBatch",
    "SingerSchema",
    "__version__",
    "jsonl_sink",
    "lineage_dot",
    "lineage_json",
    "parse_messages",
    "run_sync",
    "run_target",
]
