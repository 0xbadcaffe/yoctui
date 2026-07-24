#!/usr/bin/env python3
"""Drive the production bridge against an explicitly enabled live BitBake build."""

import argparse
import json
import os
import selectors
import subprocess
import sys
import time
from pathlib import Path


class SmokeFailure(RuntimeError):
    pass


class BridgeDriver:
    def __init__(self, bridge: Path, build_dir: Path, timeout: float):
        self.timeout = timeout
        self.sequence = 0
        self.correlation = 0
        self.process = subprocess.Popen(
            [sys.executable, str(bridge)],
            cwd=build_dir,
            env=os.environ.copy(),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=None,
            bufsize=0,
        )
        if self.process.stdin is None or self.process.stdout is None:
            raise SmokeFailure("could not open bridge standard I/O")
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)

    def send(self, message):
        self.sequence += 1
        self.correlation += 1
        envelope = {
            "protocol_version": 1,
            "sequence": self.sequence,
            "correlation_id": self.correlation,
            "message": message,
        }
        payload = json.dumps(envelope, separators=(",", ":")).encode() + b"\n"
        self.process.stdin.write(payload)
        self.process.stdin.flush()
        return self.correlation

    def receive(self, deadline):
        remaining = deadline - time.monotonic()
        if remaining <= 0 or not self.selector.select(remaining):
            raise SmokeFailure("timed out waiting for a bridge event")
        raw = self.process.stdout.readline()
        if not raw:
            code = self.process.poll()
            raise SmokeFailure(f"bridge exited before replying (exit code {code})")
        try:
            return json.loads(raw)
        except json.JSONDecodeError as exc:
            raise SmokeFailure(f"bridge stdout was not NDJSON: {raw!r}") from exc

    def wait_for(self, correlation, event_type):
        deadline = time.monotonic() + self.timeout
        observed = []
        while True:
            envelope = self.receive(deadline)
            message = envelope.get("message", {})
            observed.append(message)
            if os.environ.get("YOCTUI_LIVE_TRACE") == "1":
                print(
                    "live event: "
                    f"correlation={envelope.get('correlation_id')} "
                    f"type={message.get('type')}",
                    file=sys.stderr,
                )
            if message.get("type") == "command_failed":
                raise SmokeFailure(
                    f"bridge command failed: {message.get('code')}: "
                    f"{message.get('message')}"
                )
            if (
                envelope.get("correlation_id") == correlation
                and message.get("type") == event_type
            ):
                return message, observed

    def close(self):
        self.selector.close()
        if self.process.stdin is not None:
            self.process.stdin.close()
        try:
            return self.process.wait(timeout=10)
        except subprocess.TimeoutExpired as exc:
            self.process.terminate()
            self.process.wait(timeout=5)
            raise SmokeFailure("bridge did not exit after shutdown") from exc


def require_event_types(events, required, phase):
    observed = {event.get("type") for event in events}
    missing = sorted(required - observed)
    if missing:
        raise SmokeFailure(
            f"{phase} did not expose required real events: {', '.join(missing)}; "
            f"observed: {', '.join(sorted(item for item in observed if item))}"
        )


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bridge", type=Path, required=True)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--target", default="base-files")
    parser.add_argument("--task", default="listtasks")
    parser.add_argument("--cancel-target", default="core-image-minimal")
    parser.add_argument("--timeout", type=float, default=300.0)
    return parser.parse_args()


def main():
    args = parse_args()
    driver = BridgeDriver(args.bridge, args.build_dir, args.timeout)
    summary = {}
    try:
        correlation = driver.send({"type": "hello"})
        hello, _ = driver.wait_for(correlation, "hello_ack")
        version = hello.get("bitbake_version")
        if not version:
            raise SmokeFailure("live bridge did not report a BitBake version")
        summary["bitbake_version"] = version

        correlation = driver.send({"type": "inspect_workspace"})
        workspace, _ = driver.wait_for(correlation, "workspace")
        data = workspace.get("data", {})
        if Path(data.get("build_dir", "")).resolve() != args.build_dir.resolve():
            raise SmokeFailure("bridge inspected a different build directory")
        summary["release"] = data.get("release")

        correlation = driver.send(
            {"type": "get_variable", "name": "MACHINE", "recipe": None}
        )
        variable, _ = driver.wait_for(correlation, "variable")
        if not variable.get("value"):
            raise SmokeFailure("live MACHINE lookup returned no value")
        summary["machine"] = variable["value"]

        correlation = driver.send({"type": "list_recipes", "filter": args.target})
        recipes, _ = driver.wait_for(correlation, "recipes")
        if not any(item.get("name") == args.target for item in recipes["recipes"]):
            raise SmokeFailure(f"live recipe listing did not contain {args.target}")

        correlation = driver.send({"type": "list_layers"})
        layers, _ = driver.wait_for(correlation, "layers")
        if not layers.get("layers"):
            raise SmokeFailure("live layer listing was empty")
        summary["layer_count"] = len(layers["layers"])

        correlation = driver.send(
            {
                "type": "start_build",
                "targets": [args.target],
                "task": args.task,
            }
        )
        completion, build_events = driver.wait_for(correlation, "build_completed")
        require_event_types(
            build_events,
            {"build_started", "parse_progress", "task_queued", "task_started", "log"},
            "normal build",
        )
        queued = [
            event for event in build_events if event.get("type") == "task_queued"
        ]
        if not any(
            isinstance(event.get("stats"), dict)
            and isinstance(event["stats"].get("total"), int)
            and event["stats"]["total"] > 0
            for event in queued
        ):
            raise SmokeFailure(
                "live queued-task events did not expose a positive authoritative task total"
            )
        if not completion.get("success"):
            raise SmokeFailure(
                f"live build failed with exit code {completion.get('exit_code')}"
            )
        summary["normal_build_events"] = sorted(
            {event.get("type") for event in build_events if event.get("type")}
        )

        correlation = driver.send(
            {
                "type": "start_build",
                "targets": [args.cancel_target],
                "task": None,
            }
        )
        _, cancel_events = driver.wait_for(correlation, "build_started")
        driver.send({"type": "cancel_build"})
        completion, more_events = driver.wait_for(correlation, "build_completed")
        cancel_events.extend(more_events)
        if completion.get("success"):
            raise SmokeFailure("cancelled live build reported success")
        summary["cancellation_events"] = sorted(
            {event.get("type") for event in cancel_events if event.get("type")}
        )

        correlation = driver.send({"type": "shutdown"})
        driver.wait_for(correlation, "bridge_shutdown")
    finally:
        code = driver.close()
    if code != 0:
        raise SmokeFailure(f"bridge exited with code {code}")
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except SmokeFailure as exc:
        print(f"live BitBake smoke failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
