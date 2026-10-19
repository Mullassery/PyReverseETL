"""Singer protocol target tests."""

from __future__ import annotations

import json

from pyreverseetl.singer import parse_messages, run_target


def _lines():
    yield json.dumps(
        {
            "type": "SCHEMA",
            "stream": "users",
            "schema": {"properties": {}},
            "key_properties": ["id"],
        }
    )
    yield json.dumps({"type": "RECORD", "stream": "users", "record": {"id": 1}})
    yield json.dumps({"type": "RECORD", "stream": "users", "record": {"id": 2}})
    yield json.dumps({"type": "STATE", "value": {"bookmark": 2}})


def test_parse_messages():
    assert len(list(parse_messages(_lines()))) == 4


def test_run_target_batches_records():
    received = []
    count = run_target(
        _lines(), lambda schema, records: received.append((schema.stream, records))
    )
    assert count == 2
    assert received[0][0] == "users"
    assert received[0][1] == [{"id": 1}, {"id": 2}]


def test_jsonl_sink(tmp_path):
    from pyreverseetl.singer import jsonl_sink

    out = tmp_path / "out.jsonl"
    run_target(_lines(), jsonl_sink(out))
    lines = out.read_text().splitlines()
    assert len(lines) == 2
    assert json.loads(lines[0])["stream"] == "users"
