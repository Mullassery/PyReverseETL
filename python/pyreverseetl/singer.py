"""Singer protocol target adapter.

Singer is the open tap/target protocol used by Meltano and many connectors.
This module parses Singer NDJSON messages and forwards RECORD batches to a
caller-provided sink. It is a protocol adapter, not a replacement for
PyReverseETL's native connectors.
"""

from __future__ import annotations

import json
import sys
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable


@dataclass
class SingerSchema:
    stream: str
    schema: dict[str, Any]
    key_properties: list[str] = field(default_factory=list)


@dataclass
class SingerBatch:
    schema: SingerSchema | None = None
    records: list[dict[str, Any]] = field(default_factory=list)
    state: dict[str, Any] | None = None


def parse_messages(lines: Iterable[str]) -> Iterable[dict[str, Any]]:
    for line in lines:
        line = line.strip()
        if not line:
            continue
        message = json.loads(line)
        if isinstance(message, dict) and "type" in message:
            yield message


def run_target(
    lines: Iterable[str],
    sink: Callable[[SingerSchema, list[dict[str, Any]]], None],
    *,
    batch_size: int = 1000,
) -> int:
    """Consume Singer messages, batching RECORDs per stream into ``sink``."""
    schema: SingerSchema | None = None
    batch: list[dict[str, Any]] = []
    written = 0

    def flush() -> None:
        nonlocal batch, written
        if schema is not None and batch:
            sink(schema, batch)
            written += len(batch)
            batch = []

    for message in parse_messages(lines):
        kind = message.get("type")
        if kind == "SCHEMA":
            flush()
            schema = SingerSchema(
                stream=str(message.get("stream") or "default"),
                schema=message.get("schema") or {},
                key_properties=list(message.get("key_properties") or []),
            )
        elif kind == "RECORD":
            if schema is None:
                raise ValueError("RECORD received before SCHEMA")
            batch.append(dict(message.get("record") or {}))
            if len(batch) >= batch_size:
                flush()
        elif kind == "STATE":
            flush()
    flush()
    return written


def jsonl_sink(
    path: str | Path,
) -> Callable[[SingerSchema, list[dict[str, Any]]], None]:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)

    def sink(schema: SingerSchema, records: list[dict[str, Any]]) -> None:
        with path.open("a", encoding="utf-8") as handle:
            for record in records:
                handle.write(
                    json.dumps({"stream": schema.stream, "record": record}) + "\n"
                )

    return sink


def main(argv: list[str] | None = None) -> int:
    import argparse

    parser = argparse.ArgumentParser(prog="pyreverseetl-singer-target")
    parser.add_argument("--output", required=True, help="JSONL output path")
    parser.add_argument(
        "--input", default="-", help="Singer NDJSON file or - for stdin"
    )
    args = parser.parse_args(argv)
    if args.input == "-":
        count = run_target(sys.stdin, jsonl_sink(args.output))
    else:
        with open(args.input, encoding="utf-8") as stream:
            count = run_target(stream, jsonl_sink(args.output))
    print(f"wrote {count} record(s) to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


__all__ = [
    "SingerBatch",
    "SingerSchema",
    "jsonl_sink",
    "main",
    "parse_messages",
    "run_target",
]
