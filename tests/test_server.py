"""Tests for the REST wrapper in `python/pyreverseetl/server.py`.

`PyReverseETLServer` is a thin delegation layer over `CLIInterface` (see
`cli.py`); these tests cover the delegation itself, the workflow/activation
bookkeeping, and the Flask route wiring (request validation and response
shapes). They deliberately never trigger a real sync: `execute_activation`
for a *known* activation calls the Rust engine and needs a live
source/destination, so only its "activation not found" fast-path is
exercised here (the engine itself is covered by the Rust test suite in
`core/`).
"""

from __future__ import annotations

import pytest

pyreverseetl = pytest.importorskip("pyreverseetl")

from pyreverseetl.server import PyReverseETLServer, create_flask_app


@pytest.fixture()
def server(tmp_path, monkeypatch):
    """A server isolated from the real `~/.pyreverseetl/state.json`."""
    monkeypatch.setenv("PYREVERSEETL_STATE_PATH", str(tmp_path / "state.json"))
    return PyReverseETLServer()


def test_server_defaults_to_loopback(server):
    # The REST API has no authentication, so the default bind address must be
    # loopback rather than every interface (RECHECK-002).
    assert server.host == "127.0.0.1"
    assert PyReverseETLServer().host == "127.0.0.1"


def test_health_check(server):
    result = server.health_check()
    assert result["status"] == "healthy"
    assert result["service"] == "pyreverseetl"
    assert "version" in result
    assert result["workflows_count"] == 0
    assert result["active_activations"] == 0


def test_create_and_list_workflows(server):
    created = server.create_workflow("wf1", {"name": "My WF", "source": "postgres"})
    assert created["status"] == "success"
    assert created["workflow_id"] == "wf1"

    listed = server.list_workflows()
    assert listed["status"] == "success"
    assert listed["count"] == 1
    assert any(w["id"] == "wf1" for w in listed["workflows"])

    health = server.health_check()
    assert health["workflows_count"] == 1


def test_create_activation_requires_existing_workflow(server):
    result = server.create_activation("act1", {"workflow_id": "missing"})
    assert result["status"] == "error"
    assert "not found" in result["message"]


def test_create_and_list_activations(server):
    server.create_workflow("wf1", {"name": "WF", "source": "postgres"})
    created = server.create_activation(
        "act1", {"workflow_id": "wf1", "destination": "s3"}
    )
    assert created["status"] == "success"
    assert created["activation_id"] == "act1"

    listed = server.list_activations()
    assert listed["count"] == 1
    assert any(a["id"] == "act1" for a in listed["activations"])


def test_execute_unknown_activation_returns_error(server):
    result = server.execute_activation("does-not-exist")
    assert result["status"] == "error"
    assert "not found" in result["message"]


def test_get_run_status_unknown(server):
    result = server.get_run_status("no-such-run")
    assert result["status"] == "error"
    assert "not found" in result["message"]


def test_get_metrics_empty(server):
    result = server.get_metrics()
    assert result["status"] == "success"
    assert result["total_runs"] == 0
    assert result["success_rate"] == 0


def test_get_lineage_json_and_dot(server):
    pytest.importorskip("pyreverseetl._core")
    json_result = server.get_lineage("json")
    assert json_result["status"] == "success"
    assert json_result["format"] == "json"

    dot_result = server.get_lineage("dot")
    assert dot_result["status"] == "success"
    assert dot_result["format"] == "dot"
    assert isinstance(dot_result["lineage"], str)


flask = pytest.importorskip("flask")


@pytest.fixture()
def client(tmp_path, monkeypatch):
    monkeypatch.setenv("PYREVERSEETL_STATE_PATH", str(tmp_path / "state.json"))
    app = create_flask_app(PyReverseETLServer())
    app.config.update(TESTING=True)
    return app.test_client()


def test_route_health(client):
    resp = client.get("/health")
    assert resp.status_code == 200
    assert resp.get_json()["status"] == "healthy"


def test_route_list_workflows_empty(client):
    resp = client.get("/workflows")
    assert resp.status_code == 200
    assert resp.get_json()["count"] == 0


def test_route_create_workflow_requires_id(client):
    resp = client.post("/workflows", json={"config": {}})
    assert resp.status_code == 400
    assert resp.get_json()["message"] == "workflow_id required"


def test_route_create_and_fetch_workflow(client):
    resp = client.post(
        "/workflows",
        json={"workflow_id": "wf1", "config": {"name": "WF", "source": "postgres"}},
    )
    assert resp.status_code == 200
    assert resp.get_json()["status"] == "success"

    listed = client.get("/workflows").get_json()
    assert listed["count"] == 1


def test_route_create_activation_requires_id(client):
    resp = client.post("/activations", json={"config": {}})
    assert resp.status_code == 400
    assert resp.get_json()["message"] == "activation_id required"


def test_route_execute_unknown_activation_is_error_payload(client):
    resp = client.post("/activations/missing/execute", json={})
    assert resp.status_code == 200
    assert resp.get_json()["status"] == "error"


def test_route_metrics(client):
    resp = client.get("/metrics")
    assert resp.status_code == 200
    assert resp.get_json()["status"] == "success"


def test_route_lineage_json(client):
    pytest.importorskip("pyreverseetl._core")
    resp = client.get("/lineage?format=json")
    assert resp.status_code == 200
    assert resp.get_json()["status"] == "success"


def test_route_lineage_dot_sets_graphviz_content_type(client):
    pytest.importorskip("pyreverseetl._core")
    resp = client.get("/lineage?format=dot")
    assert resp.status_code == 200
    assert resp.headers["Content-Type"].startswith("text/vnd.graphviz")
