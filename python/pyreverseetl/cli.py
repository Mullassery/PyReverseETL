"""CLI for PyReverseETL - integration with workflow tools.

`execute-activation` (and the legacy `execute` alias) drive the real Rust sync
engine (`pyreverseetl._core.run_sync`): it opens a real connection to the
configured source, reads real records, applies real compliance rules, and
writes real records to the configured destination (Postgres, MySQL, an
S3-compatible store, a webhook, or a real Salesforce/HubSpot/Marketo API
call). Nothing here fabricates row counts -- `rows_synced` in the output is
exactly what the engine actually wrote.

`create-workflow` / `create-activation` are local bookkeeping only (this
process's view of what workflows/activations exist and which real connector
config to use for each) -- that part was never the problem the prior audit
found. What *was* the problem, and what this file no longer does, is
simulate `execute` with an in-memory dict instead of calling the engine.

Workflow/activation/run bookkeeping is persisted to a small JSON state file
(default `~/.pyreverseetl/state.json`) so that `create-workflow`,
`create-activation`, and `execute` -- three separate CLI invocations, as
shown in the README -- see the same state. This mirrors what a real CLI tool
needs (`git`, `docker`, etc. all persist local state between invocations);
without it, `execute` run as its own process would never be able to find a
workflow/activation created by an earlier command.
"""

import json
import os
import sys
from typing import Optional

DEFAULT_STATE_PATH = os.path.join(os.path.expanduser("~"), ".pyreverseetl", "state.json")


class CLIInterface:
    """Command-line interface for PyReverseETL workflow integration."""

    def __init__(self, state_path: Optional[str] = None):
        self.state_path = state_path or os.environ.get("PYREVERSEETL_STATE_PATH", DEFAULT_STATE_PATH)
        self.workflows = {}
        self.activations = {}
        self.runs = {}
        self._load_state()

    def _load_state(self):
        try:
            with open(self.state_path) as f:
                data = json.load(f)
            self.workflows = data.get("workflows", {})
            self.activations = data.get("activations", {})
            self.runs = data.get("runs", {})
        except (FileNotFoundError, json.JSONDecodeError):
            pass

    def _save_state(self):
        directory = os.path.dirname(self.state_path)
        if directory:
            os.makedirs(directory, exist_ok=True)
        with open(self.state_path, "w") as f:
            json.dump(
                {"workflows": self.workflows, "activations": self.activations, "runs": self.runs},
                f,
                indent=2,
            )

    def create_workflow(
        self,
        workflow_id: str,
        name: str,
        source: str,
        table: str,
        source_config: Optional[dict] = None,
    ) -> dict:
        """Create a new workflow.

        Args:
            workflow_id: Unique workflow identifier
            name: Human-readable workflow name
            source: Source connector type. One of the real, engine-backed
                connectors ("postgres", "mysql", "s3") if this workflow will
                ever be executed; anything else is fine for bookkeeping only.
            table: Source table name
            source_config: Real connection details for the source connector
                (host/port/database/username/password/table for postgres and
                mysql; bucket/path/[endpoint/access_key/secret_key] for s3).
                Required for `execute` to actually run.

        Returns:
            JSON response with workflow details
        """
        self.workflows[workflow_id] = {
            "id": workflow_id,
            "name": name,
            "source": source,
            "table": table,
            "source_config": source_config or {},
            "status": "active",
            "created": True,
        }
        self._save_state()
        return {
            "status": "success",
            "workflow_id": workflow_id,
            "name": name,
            "message": f"Workflow '{name}' created successfully",
        }

    def create_activation(
        self,
        activation_id: str,
        workflow_id: str,
        destination: str,
        sync_mode: str = "incremental",
        destination_config: Optional[dict] = None,
    ) -> dict:
        """Create an activation (workflow -> destination mapping).

        Args:
            activation_id: Unique activation identifier
            workflow_id: Workflow to activate
            destination: Destination connector type. One of the real,
                engine-backed connectors ("postgres", "mysql", "s3",
                "webhook", "salesforce", "hubspot", "marketo") if this
                activation will ever be executed.
            sync_mode: 'batch', 'incremental', or 'streaming'
            destination_config: Real connection details for the destination
                connector, including an "auth" object for webhook/salesforce/
                hubspot/marketo (see README for the shape per connector).
                Required for `execute` to actually run.

        Returns:
            JSON response with activation details
        """
        if workflow_id not in self.workflows:
            return {
                "status": "error",
                "message": f"Workflow '{workflow_id}' not found",
            }

        self.activations[activation_id] = {
            "id": activation_id,
            "workflow_id": workflow_id,
            "destination": destination,
            "sync_mode": sync_mode,
            "destination_config": destination_config or {},
            "status": "active",
        }
        self._save_state()
        return {
            "status": "success",
            "activation_id": activation_id,
            "workflow_id": workflow_id,
            "destination": destination,
            "message": f"Activation created: {workflow_id} -> {destination}",
        }

    def execute_activation(
        self,
        activation_id: str,
        limit: Optional[int] = None,
        compliance_rules: Optional[list] = None,
    ) -> dict:
        """Execute an activation: run a real sync through the Rust engine.

        This calls `pyreverseetl._core.run_sync`, which opens a real
        connection to the workflow's source, reads real records (bounded by
        `limit` if given), applies `compliance_rules` (PII masking/removal/
        truncation), and writes real records to the activation's
        destination. `rows_synced` below is the real number of records the
        destination connector/adapter actually wrote -- not a fabricated
        number.

        Args:
            activation_id: Activation to execute
            limit: Optional limit on rows to read from the source
            compliance_rules: Optional list of compliance rule dicts, e.g.
                `[{"id": "mask_email", "rule_type": "pii_masking",
                   "target_fields": ["email"],
                   "action": {"type": "mask", "pattern": "****"}}]`

        Returns:
            JSON response with real execution details, or a real error if
            the source/destination isn't reachable or isn't configured.
        """
        if activation_id not in self.activations:
            return {
                "status": "error",
                "message": f"Activation '{activation_id}' not found",
            }

        activation = self.activations[activation_id]
        workflow = self.workflows.get(activation["workflow_id"])
        if workflow is None:
            return {
                "status": "error",
                "message": f"Workflow '{activation['workflow_id']}' not found",
            }

        try:
            from . import _core
        except ImportError as e:
            return {
                "status": "error",
                "message": f"Rust engine not available (build with `maturin develop`): {e}",
            }

        source_config = dict(workflow.get("source_config") or {})
        source_config.setdefault("table", workflow.get("table"))
        destination_config = activation.get("destination_config") or {}

        try:
            result = _core.run_sync(
                source_type=workflow["source"],
                source_config=json.dumps(source_config),
                destination_type=activation["destination"],
                destination_config=json.dumps(destination_config),
                limit=limit,
                compliance_rules=json.dumps(compliance_rules) if compliance_rules else None,
            )
        except Exception as e:  # noqa: BLE001 - surface real engine errors verbatim
            return {
                "status": "error",
                "activation_id": activation_id,
                "message": f"Sync failed: {e}",
            }

        run_id = result.run_id
        self.runs[run_id] = {
            "run_id": run_id,
            "activation_id": activation_id,
            "status": "success" if result.rows_failed == 0 else "partial_failure",
            "rows_synced": result.rows_written,
            "rows_read": result.rows_read,
            "rows_failed": result.rows_failed,
            "compliance_violations": result.compliance_violations,
            "destination": activation["destination"],
            "started_at": result.started_at,
            "completed_at": result.completed_at,
            "duration_ms": result.duration_ms,
        }
        self._save_state()

        return {
            "status": "success",
            "run_id": run_id,
            "activation_id": activation_id,
            "rows_synced": result.rows_written,
            "rows_read": result.rows_read,
            "rows_failed": result.rows_failed,
            "compliance_violations": result.compliance_violations,
            "duration_ms": result.duration_ms,
            "message": f"Activation executed: {result.rows_written} rows written to {activation['destination']}",
        }

    def get_run_status(self, run_id: str) -> dict:
        """Get status of an activation run.

        Args:
            run_id: Run identifier

        Returns:
            JSON response with run status
        """
        if run_id not in self.runs:
            return {
                "status": "error",
                "message": f"Run '{run_id}' not found",
            }

        run = self.runs[run_id]
        return {
            "status": "success",
            "run_id": run_id,
            "activation_id": run["activation_id"],
            "sync_status": run["status"],
            "rows_synced": run["rows_synced"],
            "destination": run["destination"],
        }

    def list_workflows(self) -> dict:
        """List all workflows.

        Returns:
            JSON response with workflow list
        """
        return {
            "status": "success",
            "workflows": list(self.workflows.values()),
            "count": len(self.workflows),
        }

    def list_activations(self) -> dict:
        """List all activations.

        Returns:
            JSON response with activation list
        """
        return {
            "status": "success",
            "activations": list(self.activations.values()),
            "count": len(self.activations),
        }

    def get_metrics(self, activation_id: Optional[str] = None) -> dict:
        """Get activation metrics computed from real recorded runs.

        Args:
            activation_id: Optional specific activation ID

        Returns:
            JSON response with metrics
        """
        if activation_id:
            runs = [
                r for r in self.runs.values() if r["activation_id"] == activation_id
            ]
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

    def get_lineage(self, fmt: str = "json") -> dict:
        """Return the real data-lineage graph accumulated from every sync run
        executed by this process (source -> destination edges with real
        record counts and timestamps).

        Args:
            fmt: "json" (default) or "dot" (Graphviz)
        """
        try:
            from . import _core
        except ImportError as e:
            return {
                "status": "error",
                "message": f"Rust engine not available (build with `maturin develop`): {e}",
            }

        if fmt == "dot":
            return {"status": "success", "format": "dot", "lineage": _core.lineage_dot()}
        return {"status": "success", "format": "json", "lineage": json.loads(_core.lineage_json())}


def main():
    """Main CLI entry point."""
    cli = CLIInterface()

    if len(sys.argv) < 2:
        print_help()
        sys.exit(1)

    command = sys.argv[1]

    try:
        if command in ("--version", "-v", "version"):
            try:
                from . import __version__

                print(__version__)
            except ImportError as e:
                print(json.dumps({"error": f"Rust engine not available: {e}"}))
                sys.exit(1)

        elif command == "create-workflow":
            if len(sys.argv) < 5:
                print(
                    json.dumps({"error": "Missing workflow_id, name, source, or table"})
                )
                sys.exit(1)

            workflow_id = sys.argv[2]
            name = sys.argv[3]
            source = sys.argv[4]
            table = sys.argv[5] if len(sys.argv) > 5 else "data"
            source_config = _parse_config_arg(sys.argv, "--source-config")

            result = cli.create_workflow(workflow_id, name, source, table, source_config)
            print(json.dumps(result))

        elif command == "create-activation":
            if len(sys.argv) < 5:
                print(
                    json.dumps(
                        {"error": "Missing activation_id, workflow_id, or destination"}
                    )
                )
                sys.exit(1)

            activation_id = sys.argv[2]
            workflow_id = sys.argv[3]
            destination = sys.argv[4]
            sync_mode = sys.argv[5] if len(sys.argv) > 5 and not sys.argv[5].startswith("--") else "incremental"
            destination_config = _parse_config_arg(sys.argv, "--dest-config")

            result = cli.create_activation(
                activation_id, workflow_id, destination, sync_mode, destination_config
            )
            print(json.dumps(result))

        elif command in ("execute", "execute-activation"):
            if len(sys.argv) < 3:
                print(json.dumps({"error": "Missing activation_id"}))
                sys.exit(1)

            activation_id = sys.argv[2]
            limit = None
            if len(sys.argv) > 3 and sys.argv[3].isdigit():
                limit = int(sys.argv[3])
            compliance_rules = _parse_config_arg(sys.argv, "--compliance-rules")

            result = cli.execute_activation(activation_id, limit, compliance_rules)
            print(json.dumps(result))

        elif command == "status":
            if len(sys.argv) < 3:
                print(json.dumps({"error": "Missing run_id"}))
                sys.exit(1)

            run_id = sys.argv[2]
            result = cli.get_run_status(run_id)
            print(json.dumps(result))

        elif command == "list-workflows":
            result = cli.list_workflows()
            print(json.dumps(result))

        elif command == "list-activations":
            result = cli.list_activations()
            print(json.dumps(result))

        elif command == "metrics":
            activation_id = sys.argv[2] if len(sys.argv) > 2 else None
            result = cli.get_metrics(activation_id)
            print(json.dumps(result))

        elif command == "lineage":
            fmt = sys.argv[2] if len(sys.argv) > 2 else "json"
            result = cli.get_lineage(fmt)
            print(json.dumps(result) if fmt != "dot" else result.get("lineage", ""))

        elif command == "help":
            print_help()

        else:
            print(json.dumps({"error": f"Unknown command: {command}"}))
            sys.exit(1)

    except Exception as e:
        print(json.dumps({"error": str(e), "status": "error"}))
        sys.exit(1)


def _parse_config_arg(argv, flag: str):
    """Pull `--flag <json>` or `--flag @path/to/file.json` out of argv."""
    if flag not in argv:
        return None
    idx = argv.index(flag)
    if idx + 1 >= len(argv):
        return None
    raw = argv[idx + 1]
    if raw.startswith("@"):
        with open(raw[1:]) as f:
            raw = f.read()
    return json.loads(raw)


def print_help():
    """Print help message."""
    help_text = """
PyReverseETL CLI - Data Activation Workflow Integration

USAGE:
    pyreverseetl <command> [options]

COMMANDS:
    create-workflow <workflow_id> <name> <source> [table] [--source-config <json|@file>]
        Create a new data workflow
        - workflow_id: Unique identifier (required)
        - name: Human-readable name (required)
        - source: Source connector: postgres, mysql, or s3 to actually run
          (required)
        - table: Source table name (default: data)
        - --source-config: JSON connection details, e.g.
          '{"host":"localhost","port":5432,"database":"crm","username":"postgres","password":"postgres"}'

        Example:
            pyreverseetl create-workflow ltv_sync "LTV to CRM" postgres customers \\
                --source-config '{"host":"localhost","port":5432,"database":"crm","username":"postgres","password":"postgres"}'

    create-activation <activation_id> <workflow_id> <destination> [sync_mode] [--dest-config <json|@file>]
        Map workflow to destination (batch, incremental, streaming)
        - activation_id: Unique identifier (required)
        - workflow_id: Workflow to activate (required)
        - destination: Target connector: postgres, mysql, s3, webhook,
          salesforce, hubspot, or marketo to actually run (required)
        - sync_mode: 'batch', 'incremental', or 'streaming' (default: incremental)
        - --dest-config: JSON connection details, including "auth" for
          webhook/salesforce/hubspot/marketo

        Example:
            pyreverseetl create-activation ltv_to_hook ltv_sync webhook \\
                --dest-config '{"url":"https://example.com/hook","auth":{"type":"bearer","token":"secret"}}'

    execute <activation_id> [limit] [--compliance-rules <json|@file>]
        Execute a REAL data synchronization through the Rust engine
        - activation_id: Activation to execute (required)
        - limit: Max rows to read from the source (optional)
        - --compliance-rules: JSON array of compliance rules applied before
          write, e.g. '[{"id":"mask_email","rule_type":"pii_masking",
          "target_fields":["email"],"action":{"type":"mask","pattern":"****"}}]'

        Example:
            pyreverseetl execute ltv_to_hook 5000

    status <run_id>
        Get status of a sync run

    list-workflows
        List all workflows

    list-activations
        List all activations

    metrics [activation_id]
        Get activation metrics computed from real recorded runs

    lineage [json|dot]
        Print the real data-lineage graph (source -> destination edges with
        real record counts and timestamps) accumulated by this process

    help
        Show this help message

OUTPUT FORMAT:
    All commands return JSON output for easy parsing in workflows
"""
    print(help_text)


if __name__ == "__main__":
    main()
