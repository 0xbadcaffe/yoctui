#!/usr/bin/env python3
"""Read actual isolated daemon build authority without starting any operation."""
import json
import os
from pathlib import Path
import socket
import struct

MAX_FRAME = 4 * 1024 * 1024
VERSION = {"major": 1, "minor": 5}


def receive(connection):
    def exact(count):
        result = bytearray()
        while len(result) < count:
            chunk = connection.recv(count - len(result))
            if not chunk:
                raise EOFError("daemon closed a partial frame")
            result.extend(chunk)
        return result

    length = struct.unpack(">I", exact(4))[0]
    if not 0 < length <= MAX_FRAME:
        raise ValueError("invalid daemon frame length")
    return json.loads(exact(length))


def send(connection, message):
    payload = json.dumps(message).encode()
    connection.sendall(struct.pack(">I", len(payload)) + payload)


def inspect(connection, build_dir):
    send(connection, {
        "type": "hello", "minimum_version": VERSION, "maximum_version": VERSION,
        "client_id": list(os.urandom(16)), "client_name": "zcu102-readonly-evidence",
        "capabilities": ["state_snapshots", "incremental_events", "background_jobs"],
    })
    hello = receive(connection)
    if hello.get("type") != "hello" or hello.get("selected_version") != VERSION:
        raise ValueError("unexpected daemon handshake")
    send(connection, {
        "type": "attach", "workspace": None, "resume": None,
        "subscription": {"state": True, "jobs": True, "logs": False, "pty_sessions": []},
    })
    reply = receive(connection)
    if reply.get("type") != "attached":
        raise ValueError("unexpected daemon attach response")
    snapshot = reply["snapshot"]
    if snapshot["daemon_instance_id"] != hello["daemon_instance_id"]:
        raise ValueError("daemon instance changed")
    workspace = snapshot.get("workspace") or {}
    if workspace.get("canonical_build") != str(build_dir):
        raise ValueError("daemon workspace is not the isolated ZCU102 build")
    send(connection, {"type": "detach"})
    events = snapshot.get("build_events", [])
    return {
        "daemon_instance_id": snapshot["daemon_instance_id"],
        "sequence": snapshot["sequence"], "workspace": workspace,
        "build_progress": snapshot.get("build_progress"),
        "jobs": snapshot.get("jobs", []),
        "recent_task_failures": [event for event in events
                                 if event.get("type") == "task_completed"
                                 and event.get("success") is False],
        "last_build_events": events[-4:],
    }


def main():
    root = Path(os.environ.get("YOCTUI_ZCU102_ROOT",
                               Path.home() / "src/yoctui-zcu102-2026.1"))
    if not root.is_absolute() or root.resolve() != root:
        raise ValueError("validation root must be a canonical absolute path")
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(10)
        connection.connect(str(root / "runtime/yoctui/daemon.sock"))
        _, uid, _ = struct.unpack("3i", connection.getsockopt(
            socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i")))
        if uid != os.getuid():
            raise ValueError("daemon peer does not belong to this user")
        print(json.dumps(inspect(connection, root / "build"), indent=2))


if __name__ == "__main__":
    main()
