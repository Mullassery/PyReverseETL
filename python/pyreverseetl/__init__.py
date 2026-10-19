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
from .connectors import (
    BaseConnector,
    ConnectorError,
    ConnectorSyncResult,
    SQLiteConnector,
    available_connectors,
    get_connector,
    register_connector,
    run_with_connector,
)
from .singer import SingerBatch, SingerSchema, jsonl_sink, parse_messages, run_target

__all__ = [
    "BaseConnector",
    "ConnectorError",
    "ConnectorSyncResult",
    "PyActivation",
    "PyDestination",
    "PyEntity",
    "PySyncResult",
    "PySyncRun",
    "PyWorkflow",
    "SQLiteConnector",
    "SingerBatch",
    "SingerSchema",
    "__version__",
    "available_connectors",
    "get_connector",
    "jsonl_sink",
    "lineage_dot",
    "lineage_json",
    "parse_messages",
    "register_connector",
    "run_sync",
    "run_target",
    "run_with_connector",
]
