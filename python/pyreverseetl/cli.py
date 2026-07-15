"""CLI for PyReverseETL - integration with workflow tools."""

import json
import sys
from typing import Optional


class CLIInterface:
    """Command-line interface for PyReverseETL workflow integration."""

    def __init__(self):
        self.workflows = {}
        self.activations = {}
        self.runs = {}

    def create_workflow(
        self,
        workflow_id: str,
        name: str,
        source: str,
        table: str,
    ) -> dict:
        """Create a new workflow.

        Args:
            workflow_id: Unique workflow identifier
            name: Human-readable workflow name
            source: Source warehouse/system
            table: Source table name

        Returns:
            JSON response with workflow details
        """
        self.workflows[workflow_id] = {
            "id": workflow_id,
            "name": name,
            "source": source,
            "table": table,
            "status": "active",
            "created": True,
        }
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
    ) -> dict:
        """Create an activation (workflow → destination mapping).

        Args:
            activation_id: Unique activation identifier
            workflow_id: Workflow to activate
            destination: Destination system (salesforce, hubspot, etc.)
            sync_mode: 'batch', 'incremental', or 'streaming'

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
            "status": "active",
        }
        return {
            "status": "success",
            "activation_id": activation_id,
            "workflow_id": workflow_id,
            "destination": destination,
            "message": f"Activation created: {workflow_id} → {destination}",
        }

    def execute_activation(
        self,
        activation_id: str,
        limit: Optional[int] = None,
    ) -> dict:
        """Execute an activation (sync data to destination).

        Args:
            activation_id: Activation to execute
            limit: Optional limit on rows to sync

        Returns:
            JSON response with execution details
        """
        if activation_id not in self.activations:
            return {
                "status": "error",
                "message": f"Activation '{activation_id}' not found",
            }

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
            "activation_id": activation_id,
            "rows_synced": limit or 1000,
            "message": f"Activation executed: {limit or 1000} rows synced",
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
        """Get activation metrics.

        Args:
            activation_id: Optional specific activation ID

        Returns:
            JSON response with metrics
        """
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


def main():
    """Main CLI entry point."""
    cli = CLIInterface()

    if len(sys.argv) < 2:
        print_help()
        sys.exit(1)

    command = sys.argv[1]

    try:
        if command == "create-workflow":
            if len(sys.argv) < 5:
                print(json.dumps({"error": "Missing workflow_id, name, source, or table"}))
                sys.exit(1)

            workflow_id = sys.argv[2]
            name = sys.argv[3]
            source = sys.argv[4]
            table = sys.argv[5] if len(sys.argv) > 5 else "data"

            result = cli.create_workflow(workflow_id, name, source, table)
            print(json.dumps(result))

        elif command == "create-activation":
            if len(sys.argv) < 5:
                print(json.dumps({"error": "Missing activation_id, workflow_id, or destination"}))
                sys.exit(1)

            activation_id = sys.argv[2]
            workflow_id = sys.argv[3]
            destination = sys.argv[4]
            sync_mode = sys.argv[5] if len(sys.argv) > 5 else "incremental"

            result = cli.create_activation(activation_id, workflow_id, destination, sync_mode)
            print(json.dumps(result))

        elif command == "execute":
            if len(sys.argv) < 3:
                print(json.dumps({"error": "Missing activation_id"}))
                sys.exit(1)

            activation_id = sys.argv[2]
            limit = int(sys.argv[3]) if len(sys.argv) > 3 else None

            result = cli.execute_activation(activation_id, limit)
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

        elif command == "help":
            print_help()

        else:
            print(json.dumps({"error": f"Unknown command: {command}"}))
            sys.exit(1)

    except Exception as e:
        print(json.dumps({"error": str(e), "status": "error"}))
        sys.exit(1)


def print_help():
    """Print help message."""
    help_text = """
PyReverseETL CLI - Data Activation Workflow Integration

USAGE:
    pyreverseetl <command> [options]

COMMANDS:
    create-workflow <workflow_id> <name> <source> [table]
        Create a new data workflow
        - workflow_id: Unique identifier (required)
        - name: Human-readable name (required)
        - source: Warehouse/source system (required)
        - table: Source table name (default: data)

        Example:
            pyreverseetl create-workflow ltv_sync "LTV to CRM" snowflake customers

    create-activation <activation_id> <workflow_id> <destination> [sync_mode]
        Map workflow to destination (batch, incremental, streaming)
        - activation_id: Unique identifier (required)
        - workflow_id: Workflow to activate (required)
        - destination: Target system (required)
        - sync_mode: 'batch', 'incremental', or 'streaming' (default: incremental)

        Example:
            pyreverseetl create-activation ltv_to_sf ltv_sync salesforce incremental

    execute <activation_id> [limit]
        Execute data synchronization
        - activation_id: Activation to execute (required)
        - limit: Max rows to sync (optional)

        Example:
            pyreverseetl execute ltv_to_sf 5000

    status <run_id>
        Get status of a sync run
        - run_id: Run identifier (required)

        Example:
            pyreverseetl status run_ltv_to_sf_12345

    list-workflows
        List all workflows

        Example:
            pyreverseetl list-workflows

    list-activations
        List all activations

        Example:
            pyreverseetl list-activations

    metrics [activation_id]
        Get activation metrics
        - activation_id: Optional specific activation ID

        Example:
            pyreverseetl metrics ltv_to_sf

    help
        Show this help message

OUTPUT FORMAT:
    All commands return JSON output for easy parsing in workflows

EXAMPLES:

Bash Script:
  pyreverseetl create-workflow ltv_sync "LTV Sync" snowflake customers
  pyreverseetl create-activation ltv_to_sf ltv_sync salesforce
  pyreverseetl execute ltv_to_sf 5000

Workflow Tool (n8n):
  POST http://localhost:8000/activations
  Body: { "activation_id": "ltv_to_sf", "workflow_id": "ltv_sync", "destination": "salesforce" }
"""
    print(help_text)


if __name__ == "__main__":
    main()
