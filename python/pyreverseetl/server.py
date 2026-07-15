"""REST API server for PyReverseETL - integrates with workflow tools."""

from typing import Dict, Any, Optional, List


class PyReverseETLServer:
    """REST API server for workflow integration."""

    def __init__(self, host: str = "0.0.0.0", port: int = 8000):
        """Initialize server."""
        self.host = host
        self.port = port
        self.workflows: Dict[str, Dict[str, Any]] = {}
        self.activations: Dict[str, Dict[str, Any]] = {}
        self.runs: Dict[str, Dict[str, Any]] = {}

    def create_workflow(self, workflow_id: str, config: Dict[str, Any]) -> Dict[str, Any]:
        """Create a workflow."""
        self.workflows[workflow_id] = {
            "id": workflow_id,
            "name": config.get("name", workflow_id),
            "source": config.get("source"),
            "table": config.get("table", "data"),
            "status": "active",
        }
        return {
            "status": "success",
            "workflow_id": workflow_id,
            "message": f"Workflow '{config.get('name')}' created",
        }

    def create_activation(
        self, activation_id: str, config: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Create an activation."""
        workflow_id = config.get("workflow_id")
        if workflow_id not in self.workflows:
            return {"status": "error", "message": f"Workflow '{workflow_id}' not found"}

        self.activations[activation_id] = {
            "id": activation_id,
            "workflow_id": workflow_id,
            "destination": config.get("destination"),
            "sync_mode": config.get("sync_mode", "incremental"),
            "status": "active",
        }
        return {
            "status": "success",
            "activation_id": activation_id,
            "message": "Activation created",
        }

    def execute_activation(self, activation_id: str, limit: Optional[int] = None) -> Dict[str, Any]:
        """Execute an activation."""
        if activation_id not in self.activations:
            return {"status": "error", "message": f"Activation '{activation_id}' not found"}

        activation = self.activations[activation_id]
        run_id = f"run_{activation_id}_{id(limit or 0)}"

        self.runs[run_id] = {
            "run_id": run_id,
            "activation_id": activation_id,
            "status": "running",
            "rows_synced": limit or 1000,
            "destination": activation["destination"],
        }

        return {
            "status": "success",
            "run_id": run_id,
            "rows_synced": limit or 1000,
            "message": "Activation executed",
        }

    def get_run_status(self, run_id: str) -> Dict[str, Any]:
        """Get run status."""
        if run_id not in self.runs:
            return {"status": "error", "message": f"Run '{run_id}' not found"}

        run = self.runs[run_id]
        return {
            "status": "success",
            "run_id": run_id,
            "activation_id": run["activation_id"],
            "sync_status": run["status"],
            "rows_synced": run["rows_synced"],
            "destination": run["destination"],
        }

    def list_workflows(self) -> Dict[str, Any]:
        """List workflows."""
        return {
            "status": "success",
            "workflows": list(self.workflows.values()),
            "count": len(self.workflows),
        }

    def list_activations(self) -> Dict[str, Any]:
        """List activations."""
        return {
            "status": "success",
            "activations": list(self.activations.values()),
            "count": len(self.activations),
        }

    def get_metrics(self, activation_id: Optional[str] = None) -> Dict[str, Any]:
        """Get metrics."""
        if activation_id:
            runs = [r for r in self.runs.values() if r["activation_id"] == activation_id]
        else:
            runs = list(self.runs.values())

        total_runs = len(runs)
        total_rows = sum(r.get("rows_synced", 0) for r in runs)
        successful = sum(1 for r in runs if r.get("status") == "success")

        return {
            "status": "success",
            "activation_id": activation_id,
            "total_runs": total_runs,
            "successful_runs": successful,
            "total_rows_synced": total_rows,
            "success_rate": ((successful / total_runs * 100) if total_runs > 0 else 0),
        }

    def health_check(self) -> Dict[str, Any]:
        """Health check endpoint."""
        return {
            "status": "healthy",
            "service": "pyreverseetl",
            "version": "0.1.0",
            "workflows_count": len(self.workflows),
            "active_activations": len(self.activations),
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
        """Execute activation."""
        data = request.get_json() or {}
        limit = data.get("limit")
        return jsonify(srv.execute_activation(activation_id, limit))

    @app.route("/runs/<run_id>", methods=["GET"])
    def get_status(run_id):
        """Get run status."""
        return jsonify(srv.get_run_status(run_id))

    @app.route("/metrics", methods=["GET"])
    def metrics():
        """Get metrics."""
        activation_id = request.args.get("activation_id")
        return jsonify(srv.get_metrics(activation_id))

    return app


def run_server(host: str = "0.0.0.0", port: int = 8000):
    """Run the REST API server."""
    app = create_flask_app()
    app.run(host=host, port=port, debug=False)


if __name__ == "__main__":
    run_server()
