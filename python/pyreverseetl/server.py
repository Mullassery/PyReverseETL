"""REST API server for PyReverseETL - integrates with workflow tools.

Thin REST wrapper around `CLIInterface` (see `cli.py`), which is what
actually drives the real Rust sync engine. This file used to have its own,
separate in-memory dict simulator with the exact same problem the CLI had --
`execute_activation()` fabricated `rows_synced = limit or 1000` without doing
anything. It now delegates to `CLIInterface` so both entry points share one
real code path instead of two divergent fake ones.
"""

from typing import Any, Dict, Optional

from .cli import CLIInterface


class PyReverseETLServer:
    """REST API server for workflow integration, backed by the real sync engine."""

    def __init__(self, host: str = "0.0.0.0", port: int = 8000):
        """Initialize server."""
        self.host = host
        self.port = port
        self._cli = CLIInterface()

    def create_workflow(self, workflow_id: str, config: Dict[str, Any]) -> Dict[str, Any]:
        """Create a workflow.

        `config` supports: name, source (postgres/mysql/s3 to actually run),
        table, source_config (real connector connection details).
        """
        return self._cli.create_workflow(
            workflow_id,
            config.get("name", workflow_id),
            config.get("source"),
            config.get("table", "data"),
            config.get("source_config"),
        )

    def create_activation(self, activation_id: str, config: Dict[str, Any]) -> Dict[str, Any]:
        """Create an activation.

        `config` supports: workflow_id, destination (postgres/mysql/s3/
        webhook/salesforce/hubspot/marketo to actually run), sync_mode,
        destination_config (real connector connection + auth details).
        """
        workflow_id = config.get("workflow_id")
        return self._cli.create_activation(
            activation_id,
            workflow_id,
            config.get("destination"),
            config.get("sync_mode", "incremental"),
            config.get("destination_config"),
        )

    def execute_activation(
        self,
        activation_id: str,
        limit: Optional[int] = None,
        compliance_rules: Optional[list] = None,
    ) -> Dict[str, Any]:
        """Execute an activation: a real sync through the Rust engine (see
        `CLIInterface.execute_activation`). `rows_synced` is the real number
        of records the destination connector/adapter actually wrote.
        """
        return self._cli.execute_activation(activation_id, limit, compliance_rules)

    def get_run_status(self, run_id: str) -> Dict[str, Any]:
        """Get run status."""
        return self._cli.get_run_status(run_id)

    def list_workflows(self) -> Dict[str, Any]:
        """List workflows."""
        return self._cli.list_workflows()

    def list_activations(self) -> Dict[str, Any]:
        """List activations."""
        return self._cli.list_activations()

    def get_metrics(self, activation_id: Optional[str] = None) -> Dict[str, Any]:
        """Get metrics computed from real recorded runs."""
        return self._cli.get_metrics(activation_id)

    def get_lineage(self, fmt: str = "json") -> Dict[str, Any]:
        """Get the real data-lineage graph accumulated from every sync run."""
        return self._cli.get_lineage(fmt)

    def health_check(self) -> Dict[str, Any]:
        """Health check endpoint."""
        try:
            from . import __version__
        except ImportError:
            __version__ = "unknown"
        return {
            "status": "healthy",
            "service": "pyreverseetl",
            "version": __version__,
            "workflows_count": len(self._cli.workflows),
            "active_activations": len(self._cli.activations),
        }


def create_flask_app(server: Optional[PyReverseETLServer] = None):
    """Create Flask app for REST API."""
    try:
        from flask import Flask, request, jsonify
    except ImportError:
        raise ImportError(
            "Flask is required for REST API. Install with: pip install flask"
        )

    app = Flask(__name__)
    srv = server or PyReverseETLServer()

    @app.route("/health", methods=["GET"])
    def health():
        """Health check."""
        return jsonify(srv.health_check())

    @app.route("/workflows", methods=["GET"])
    def list_workflows():
        """List workflows."""
        return jsonify(srv.list_workflows())

    @app.route("/workflows", methods=["POST"])
    def create_workflow():
        """Create workflow."""
        data = request.get_json()
        workflow_id = data.get("workflow_id")
        config = data.get("config", {})

        if not workflow_id:
            return (
                jsonify({"status": "error", "message": "workflow_id required"}),
                400,
            )

        return jsonify(srv.create_workflow(workflow_id, config))

    @app.route("/activations", methods=["GET"])
    def list_activations():
        """List activations."""
        return jsonify(srv.list_activations())

    @app.route("/activations", methods=["POST"])
    def create_activation():
        """Create activation."""
        data = request.get_json()
        activation_id = data.get("activation_id")
        config = data.get("config", {})

        if not activation_id:
            return (
                jsonify({"status": "error", "message": "activation_id required"}),
                400,
            )

        return jsonify(srv.create_activation(activation_id, config))

    @app.route("/activations/<activation_id>/execute", methods=["POST"])
    def execute_activation(activation_id):
        """Execute activation (real sync via the Rust engine)."""
        data = request.get_json() or {}
        limit = data.get("limit")
        compliance_rules = data.get("compliance_rules")
        return jsonify(srv.execute_activation(activation_id, limit, compliance_rules))

    @app.route("/runs/<run_id>", methods=["GET"])
    def get_status(run_id):
        """Get run status."""
        return jsonify(srv.get_run_status(run_id))

    @app.route("/metrics", methods=["GET"])
    def metrics():
        """Get metrics."""
        activation_id = request.args.get("activation_id")
        return jsonify(srv.get_metrics(activation_id))

    @app.route("/lineage", methods=["GET"])
    def lineage():
        """Get the real data-lineage graph."""
        fmt = request.args.get("format", "json")
        result = srv.get_lineage(fmt)
        if fmt == "dot":
            return result.get("lineage", ""), 200, {"Content-Type": "text/vnd.graphviz"}
        return jsonify(result)

    return app


def run_server(host: str = "0.0.0.0", port: int = 8000):
    """Run the REST API server."""
    app = create_flask_app()
    app.run(host=host, port=port, debug=False)


if __name__ == "__main__":
    run_server()
