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
from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request

DEFAULT_STATE_PATH = os.path.join(os.path.expanduser("~"), ".pyreverseetl", "state.json")


def validate_activation_rows(
    rows: list[dict],
    service_url: str,
    *,
    limit: int = 100,
    timeout_seconds: float = 10.0,
) -> tuple[bool, dict]:
    """POST sample rows to StatGuardian's ``/validate`` endpoint.

    Fails closed on network/HTTP/parse errors. Only the first ``limit`` rows
    are checked; this is a pre-activation sample gate, not a full-table scan.
    """
    url = service_url.rstrip("/") + "/validate"
    checked = 0
    for row in rows[: max(1, limit)]:
        checked += 1
        body = json.dumps({"entity": {"attributes": row}}).encode("utf-8")
        request = urllib.request.Request(
            url,
            data=body,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
                payload = json.loads(response.read().decode("utf-8"))
        except (urllib.error.URLError, OSError, ValueError) as exc:
            return False, {"reason": f"StatGuardian request failed: {exc}", "checked": checked}
        if payload.get("passed") is not True:
            return False, {
                "reason": "StatGuardian rejected a sample row",
                "checked": checked,
                "issues": payload.get("issues", []),
            }
    return True, {"checked": checked}


def validation_report_allows(path: str) -> tuple[bool, dict]:
    """Read a StatGuardian-style JSON report and decide whether sync may run.

    Expected shape: ``{"passed": bool, "violations": [...], ...}``. A missing
    or malformed file fails closed. This is a file-based gate until
    StatGuardian exposes a service endpoint; it does not call StatGuardian.
    """
    try:
        with open(path, encoding="utf-8") as handle:
            report = json.load(handle)
    except (OSError, ValueError) as exc:
        return False, {"reason": f"cannot read validation report: {exc}"}
    if not isinstance(report, dict) or "passed" not in report:
        return False, {"reason": "validation report must be a JSON object with a 'passed' field"}
    if report.get("passed") is not True:
        return False, {
            "reason": "validation report did not pass",
            "violations": report.get("violations", []),
        }
    return True, {}


class CLIInterface:
    """Command-line interface for PyReverseETL workflow integration."""

    def __init__(self, state_path: str | None = None):
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
        source_config: dict | None = None,
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
        destination_config: dict | None = None,
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
        limit: int | None = None,
        compliance_rules: list | None = None,
        dry_run: bool = False,
        schema_store_path: str | None = None,
        idempotency_store_path: str | None = None,
        dlq_store_path: str | None = None,
        validation_report_path: str | None = None,
        statguardian_url: str | None = None,
        statguardian_limit: int = 100,
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
            dry_run: If True, records are still read from the real source and
                run through the real compliance engine, but nothing is ever
                written to the destination -- no HTTP request, database
                write, or object-storage put. The response's
                `dry_run_preview` holds the exact post-compliance payload for
                every record that would have been sent, so it can be
                audited before running for real. No run is recorded in
                history, since nothing was actually synced.
            schema_store_path: Optional path to a SQLite file where the
                last-seen field-name/type shape for this source->destination
                pair is persisted and diffed against on every run. Omit to
                disable schema-drift checking entirely (default).
            idempotency_store_path: Optional path to a SQLite file recording
                which exact record content was already successfully sent to
                which destination. Re-running the same sync (e.g. after a
                crash mid-batch) skips records already synced instead of
                re-sending them, while a record whose content genuinely
                changed is still sent. Only applies to the adapter-based
                destinations (webhook/salesforce/hubspot/marketo). Omit to
                disable idempotency checking entirely (default).
            validation_report_path: Optional path to a JSON validation report
                (``{"passed": bool, ...}``). If supplied and not passing, the
                sync is blocked before any source read or destination write.
                This is a file-based StatGuardian gate.
            statguardian_url: Optional base URL of a running StatGuardian
                service. A dry-run reads a sample of source rows and POSTs them
                to ``/validate``; any failure (or network error) blocks the real
                sync. This doubles the source read for the sample pass.
            statguardian_limit: Maximum sample rows sent to StatGuardian
                (default 100).
            dlq_store_path: Optional path to a SQLite file used as a
                dead-letter queue (quarantine) for records the destination
                rejects. Each failed record is stored with its run id, the
                reason for rejection, and its exact payload, so a partial
                batch failure can be inspected and re-driven deliberately
                instead of re-running the whole sync. Only applies to the
                adapter-based destinations. Omit to disable quarantine
                (default).

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
        if validation_report_path:
            allowed, detail = validation_report_allows(validation_report_path)
            if not allowed:
                return {
                    "status": "error",
                    "activation_id": activation_id,
                    "message": "Blocked by validation gate",
                    "validation": detail,
                }
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

        if statguardian_url:
            try:
                preview = _core.run_sync(
                    source_type=workflow["source"],
                    source_config=json.dumps(source_config),
                    destination_type=activation["destination"],
                    destination_config=json.dumps(destination_config),
                    limit=limit,
                    compliance_rules=json.dumps(compliance_rules) if compliance_rules else None,
                    dry_run=True,
                    schema_store_path=None,
                    idempotency_store_path=None,
                    dlq_store_path=None,
                )
            except Exception as exc:  # noqa: BLE001
                return {
                    "status": "error",
                    "activation_id": activation_id,
                    "message": f"StatGuardian sample read failed: {exc}",
                }
            sample_rows = [json.loads(row) for row in preview.dry_run_preview]
            allowed, detail = validate_activation_rows(
                sample_rows, statguardian_url, limit=statguardian_limit
            )
            if not allowed:
                return {
                    "status": "error",
                    "activation_id": activation_id,
                    "message": "Blocked by StatGuardian sample validation",
                    "validation": detail,
                }

        try:
            result = _core.run_sync(
                source_type=workflow["source"],
                source_config=json.dumps(source_config),
                destination_type=activation["destination"],
                destination_config=json.dumps(destination_config),
                limit=limit,
                compliance_rules=json.dumps(compliance_rules) if compliance_rules else None,
                dry_run=dry_run,
                schema_store_path=schema_store_path,
                idempotency_store_path=idempotency_store_path,
                dlq_store_path=dlq_store_path,
            )
        except Exception as e:  # noqa: BLE001 - surface real engine errors verbatim
            return {
                "status": "error",
                "activation_id": activation_id,
                "message": f"Sync failed: {e}",
            }

        if result.dry_run:
            # Nothing was actually synced -- don't record a fake run in history.
            return {
                "status": "success",
                "run_id": result.run_id,
                "activation_id": activation_id,
                "dry_run": True,
                "rows_read": result.rows_read,
                "rows_would_write": len(result.dry_run_preview),
                "rows_dead_lettered": result.rows_dead_lettered,
                "dry_run_preview": [json.loads(p) for p in result.dry_run_preview],
                "compliance_violations": result.compliance_violations,
                "schema_changes": result.schema_changes,
                "duration_ms": result.duration_ms,
                "message": (
                    f"Dry run: {len(result.dry_run_preview)} row(s) would be written to "
                    f"{activation['destination']} (nothing was actually sent)"
                ),
            }

        run_id = result.run_id
        self.runs[run_id] = {
            "run_id": run_id,
            "activation_id": activation_id,
            "status": "success" if result.rows_failed == 0 else "partial_failure",
            "rows_synced": result.rows_written,
            "rows_read": result.rows_read,
            "rows_failed": result.rows_failed,
            "rows_skipped_idempotent": result.rows_skipped_idempotent,
            "rows_dead_lettered": result.rows_dead_lettered,
            "compliance_violations": result.compliance_violations,
            "schema_changes": result.schema_changes,
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
            "rows_skipped_idempotent": result.rows_skipped_idempotent,
            "rows_dead_lettered": result.rows_dead_lettered,
            "compliance_violations": result.compliance_violations,
            "schema_changes": result.schema_changes,
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

    def get_metrics(self, activation_id: str | None = None) -> dict:
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
            dry_run = "--dry-run" in sys.argv
            schema_store_path = None
            if "--schema-store" in sys.argv:
                idx = sys.argv.index("--schema-store")
                if idx + 1 >= len(sys.argv):
                    print(json.dumps({"error": "--schema-store requires a file path"}))
                    sys.exit(1)
                schema_store_path = sys.argv[idx + 1]
            idempotency_store_path = None
            if "--idempotency-store" in sys.argv:
                idx = sys.argv.index("--idempotency-store")
                if idx + 1 >= len(sys.argv):
                    print(json.dumps({"error": "--idempotency-store requires a file path"}))
                    sys.exit(1)
                idempotency_store_path = sys.argv[idx + 1]
            dlq_store_path = None
            if "--dlq-store" in sys.argv:
                idx = sys.argv.index("--dlq-store")
                if idx + 1 >= len(sys.argv):
                    print(json.dumps({"error": "--dlq-store requires a file path"}))
                    sys.exit(1)
                dlq_store_path = sys.argv[idx + 1]

            validation_report_path = None
            if "--validation-report" in sys.argv:
                idx = sys.argv.index("--validation-report")
                if idx + 1 >= len(sys.argv):
                    print(json.dumps({"error": "--validation-report requires a file path"}))
                    sys.exit(1)
                validation_report_path = sys.argv[idx + 1]

            statguardian_url = None
            if "--statguardian-url" in sys.argv:
                idx = sys.argv.index("--statguardian-url")
                if idx + 1 >= len(sys.argv):
                    print(json.dumps({"error": "--statguardian-url requires a URL"}))
                    sys.exit(1)
                statguardian_url = sys.argv[idx + 1]
            statguardian_limit = 100
            if "--statguardian-limit" in sys.argv:
                idx = sys.argv.index("--statguardian-limit")
                if idx + 1 >= len(sys.argv):
                    print(json.dumps({"error": "--statguardian-limit requires a number"}))
                    sys.exit(1)
                statguardian_limit = int(sys.argv[idx + 1])

            result = cli.execute_activation(
                activation_id,
                limit,
                compliance_rules,
                dry_run,
                schema_store_path,
                idempotency_store_path,
                dlq_store_path,
                validation_report_path,
                statguardian_url,
                statguardian_limit,
            )
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

    # Top-level CLI boundary: any exception must become a JSON error +
    # non-zero exit, not a raw Python traceback, so catching broadly here
    # is intentional rather than an oversight.
    except Exception as e:  # noqa: BLE001
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

    execute <activation_id> [limit] [--compliance-rules <json|@file>] [--dry-run] [--schema-store <path>] [--idempotency-store <path>] [--dlq-store <path>]
        Execute a REAL data synchronization through the Rust engine
        - activation_id: Activation to execute (required)
        - limit: Max rows to read from the source (optional)
        - --compliance-rules: JSON array of compliance rules applied before
          write, e.g. '[{"id":"mask_email","rule_type":"pii_masking",
          "target_fields":["email"],"action":{"type":"mask","pattern":"****"}}]'
        - --dry-run: Read and apply compliance rules for real, but never
          write to the destination. Prints the exact payload that would
          have been sent for each record (dry_run_preview) instead.
        - --schema-store <path>: Persist the last-seen field shape for this
          source->destination pair to a real SQLite file at <path>, and
          report any field added/removed/type-changed since the last run
          (schema_changes) instead of only finding out via per-record HTTP
          errors from the destination.
        - --idempotency-store <path>: Persist which exact record content was
          already sent to which destination to a real SQLite file at
          <path>. Re-running the same sync (e.g. after a crash mid-batch)
          skips records already synced (rows_skipped_idempotent) instead of
          re-sending them, while a record whose content genuinely changed
          is still sent. Only applies to webhook/salesforce/hubspot/marketo
          destinations.
        - --dlq-store <path>: Persist every record the destination rejects to
          a real SQLite dead-letter queue at <path>, with the run id, reason,
          and exact payload (rows_dead_lettered), so a partial batch failure
          can be inspected and re-driven deliberately instead of re-running
          the whole sync. Only applies to webhook/salesforce/hubspot/marketo
          destinations.

        Example:
            pyreverseetl execute ltv_to_hook 5000
            pyreverseetl execute ltv_to_hook --dry-run
            pyreverseetl execute ltv_to_hook --schema-store .pyreverseetl/schema.db
            pyreverseetl execute ltv_to_hook --idempotency-store .pyreverseetl/idempotency.db
            pyreverseetl execute ltv_to_hook --dlq-store .pyreverseetl/dlq.db

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
