"""Encrypted local state storage.

Workflow/activation configs can contain database and API credentials. The CLI
state file is encrypted with Fernet; the key lives in the OS keyring (or
``PYREVERSEETL_STATE_KEY`` for headless CI). Existing plaintext state is
read and transparently re-encrypted on the next save.
"""

from __future__ import annotations

import json
import os
import secrets
from pathlib import Path
from typing import Any

from cryptography.fernet import Fernet, InvalidToken

KEYRING_SERVICE = "pyreverseetl"
KEYRING_USER = "state-key"
ENV_KEY = "PYREVERSEETL_STATE_KEY"


class SecureStateError(RuntimeError):
    """Raised when state encryption/decryption cannot be performed."""


def _load_key() -> bytes:
    env = os.environ.get(ENV_KEY)
    if env:
        return env.encode("ascii")
    try:
        import keyring
    except ImportError as exc:
        raise SecureStateError(
            "keyring is required to store the state encryption key; install "
            "`pip install keyring` or set PYREVERSEETL_STATE_KEY."
        ) from exc
    stored = keyring.get_password(KEYRING_SERVICE, KEYRING_USER)
    if stored:
        return stored.encode("ascii")
    key = Fernet.generate_key()
    keyring.set_password(KEYRING_SERVICE, KEYRING_USER, key.decode("ascii"))
    return key


def encrypt_bytes(payload: bytes, key: bytes | None = None) -> bytes:
    return Fernet(key or _load_key()).encrypt(payload)


def decrypt_bytes(token: bytes, key: bytes | None = None) -> bytes:
    try:
        return Fernet(key or _load_key()).decrypt(token)
    except InvalidToken as exc:
        raise SecureStateError(
            "state file could not be decrypted with the stored key"
        ) from exc


def load_state(path: str | Path) -> dict[str, Any]:
    path = Path(path)
    if not path.is_file():
        return {}
    raw = path.read_bytes()
    if not raw.strip():
        return {}
    # Try encrypted first, then fall back to legacy plaintext JSON.
    try:
        decoded = decrypt_bytes(raw)
        data = json.loads(decoded.decode("utf-8"))
        return data if isinstance(data, dict) else {}
    except (SecureStateError, ValueError, json.JSONDecodeError):
        pass
    try:
        data = json.loads(raw.decode("utf-8"))
        return data if isinstance(data, dict) else {}
    except (ValueError, json.JSONDecodeError):
        raise SecureStateError(f"state file {path} is neither encrypted nor valid JSON")


def save_state(path: str | Path, data: dict[str, Any]) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    token = encrypt_bytes(json.dumps(data, indent=2).encode("utf-8"))
    tmp = path.with_name(f".{path.name}.{secrets.token_hex(4)}.tmp")
    tmp.write_bytes(token)
    os.replace(tmp, path)


__all__ = [
    "ENV_KEY",
    "SecureStateError",
    "decrypt_bytes",
    "encrypt_bytes",
    "load_state",
    "save_state",
]


def export_key(path: str | Path) -> str:
    """Export the active Fernet key (0600) for backup/recovery."""
    key = _load_key()
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(key)
    try:
        os.chmod(path, 0o600)
    except OSError:
        pass
    return str(path)


def import_key(path: str | Path) -> None:
    """Validate and install a key from a backup file into the keyring/env."""
    key = Path(path).read_bytes().strip()
    Fernet(key)  # validates key shape
    try:
        import keyring

        keyring.set_password(KEYRING_SERVICE, KEYRING_USER, key.decode("ascii"))
    except ImportError as exc:
        raise SecureStateError(
            "keyring is not installed; set PYREVERSEETL_STATE_KEY to the imported key"
        ) from exc


__all__ += ["export_key", "import_key"]
