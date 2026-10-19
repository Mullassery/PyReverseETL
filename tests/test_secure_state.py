"""Encrypted state tests (env key; no OS keyring required)."""

from __future__ import annotations

import pytest

pytest.importorskip("cryptography")

from cryptography.fernet import Fernet
from pyreverseetl.secure_state import SecureStateError, load_state, save_state


def test_round_trip_is_encrypted(tmp_path, monkeypatch):
    monkeypatch.setenv("PYREVERSEETL_STATE_KEY", Fernet.generate_key().decode())
    path = tmp_path / "state.json"
    save_state(path, {"workflows": {"w": {"password": "secret"}}})
    raw = path.read_bytes()
    assert b"secret" not in raw
    assert load_state(path)["workflows"]["w"]["password"] == "secret"


def test_legacy_plaintext_is_read(tmp_path, monkeypatch):
    monkeypatch.setenv("PYREVERSEETL_STATE_KEY", Fernet.generate_key().decode())
    path = tmp_path / "state.json"
    path.write_text('{"workflows": {"w": {"password": "legacy"}}}')
    assert load_state(path)["workflows"]["w"]["password"] == "legacy"


def test_wrong_key_is_actionable(tmp_path, monkeypatch):
    path = tmp_path / "state.json"
    monkeypatch.setenv("PYREVERSEETL_STATE_KEY", Fernet.generate_key().decode())
    save_state(path, {"x": 1})
    monkeypatch.setenv("PYREVERSEETL_STATE_KEY", Fernet.generate_key().decode())
    with pytest.raises(SecureStateError):
        load_state(path)


def test_export_and_import_key(tmp_path, monkeypatch):
    from pyreverseetl.secure_state import export_key, import_key

    monkeypatch.setenv("PYREVERSEETL_STATE_KEY", Fernet.generate_key().decode())
    keyfile = tmp_path / "key"
    export_key(keyfile)
    assert keyfile.stat().st_mode & 0o777 == 0o600
    import_key(keyfile)  # keyring may be absent in CI; test env key remains valid
