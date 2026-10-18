"""Tests for the file-based StatGuardian gate (no engine, no network)."""

import json

from pyreverseetl.cli import validation_report_allows


def test_passing_report_allows_sync(tmp_path):
    path = tmp_path / "report.json"
    path.write_text(json.dumps({"passed": True}), encoding="utf-8")
    allowed, detail = validation_report_allows(str(path))
    assert allowed is True
    assert detail == {}


def test_failing_report_blocks_with_violations(tmp_path):
    path = tmp_path / "report.json"
    path.write_text(
        json.dumps({"passed": False, "violations": ["amount < 0"]}),
        encoding="utf-8",
    )
    allowed, detail = validation_report_allows(str(path))
    assert allowed is False
    assert detail["violations"] == ["amount < 0"]


def test_missing_or_malformed_report_fails_closed(tmp_path):
    allowed, _ = validation_report_allows(str(tmp_path / "missing.json"))
    assert allowed is False

    bad = tmp_path / "bad.json"
    bad.write_text("not json", encoding="utf-8")
    allowed, _ = validation_report_allows(str(bad))
    assert allowed is False

    no_flag = tmp_path / "noflag.json"
    no_flag.write_text(json.dumps({"violations": []}), encoding="utf-8")
    allowed, _ = validation_report_allows(str(no_flag))
    assert allowed is False
