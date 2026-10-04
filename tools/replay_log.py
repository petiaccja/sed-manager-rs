#!/usr/bin/env python3
# Copyright (C) Péter Kardos
# Please refer to the full license distributed with this software.

"""Replays a log file written by the OTLP file span exporter to an OTLP/HTTP endpoint.

Each line of the log file is an OTLP/JSON `ExportTraceServiceRequest`, which is
sent as-is in a separate request.

The endpoint is configured by the standard OpenTelemetry environment variables:
- OTEL_EXPORTER_OTLP_TRACES_ENDPOINT: full URL of the traces endpoint.
- OTEL_EXPORTER_OTLP_ENDPOINT: base URL, `/v1/traces` is appended.
- OTEL_EXPORTER_OTLP_HEADERS: extra headers as `key1=value1,key2=value2`.
If neither endpoint is set, `http://localhost:4318/v1/traces` is used.

Note: the endpoint must accept OTLP over HTTP (usually port 4318), not gRPC (usually port 4317).
Note: this code is AI-generated.
"""

import argparse
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import unquote

DEFAULT_ENDPOINT = "http://localhost:4318/v1/traces"


def traces_endpoint() -> str:
    if endpoint := os.environ.get("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT"):
        return endpoint
    if base := os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT"):
        return f"{base.rstrip('/')}/v1/traces"
    return DEFAULT_ENDPOINT


def extra_headers() -> dict[str, str]:
    """Parses `OTEL_EXPORTER_OTLP_HEADERS`, whose values may be URL-encoded."""
    headers = {}
    for pair in os.environ.get("OTEL_EXPORTER_OTLP_HEADERS", "").split(","):
        key, separator, value = pair.partition("=")
        if separator:
            headers[key.strip()] = unquote(value.strip())
    return headers


class RejectedError(Exception):
    """The endpoint received the request, but rejected it."""


def send(endpoint: str, headers: dict[str, str], body: str) -> None:
    """Sends one request.

    Raises `RejectedError` if the endpoint rejects the request, and `urllib.error.URLError`
    if the endpoint can't be reached.
    """
    request = urllib.request.Request(
        endpoint,
        data=body.encode(),
        headers={**headers, "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request):
            pass
    except urllib.error.HTTPError as error:
        raise RejectedError(
            f"HTTP {error.code}: {error.read().decode(errors='replace')}"
        ) from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("log_file", type=Path, help="the .jsonl log file to replay")
    args = parser.parse_args()

    if not args.log_file.is_file():
        parser.error(f"no such file: {args.log_file}")

    endpoint = traces_endpoint()
    headers = extra_headers()
    sent = failed = 0

    with args.log_file.open(encoding="utf-8") as log_file:
        for line_number, line in enumerate(log_file, start=1):
            if not line.strip():
                continue
            try:
                send(endpoint, headers, line)
                sent += 1
            except RejectedError as error:
                print(f"failed to send line {line_number}: {error}", file=sys.stderr)
                failed += 1
            except urllib.error.URLError as error:
                # The remaining lines would fail the same way.
                print(
                    f"failed to connect to {endpoint}: {error.reason}", file=sys.stderr
                )
                return 1

    print(f"sent {sent} requests to {endpoint}, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
