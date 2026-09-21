def main():
    isolate_protocol_output()
    adapter = select_adapter()
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.buffer, selectors.EVENT_READ)
    try:
        while True:
            # Poll quickly during builds and occasionally while idle so native
            # events cannot be stranded between adjacent client commands.
            ready = selector.select(0.1 if adapter.build_active else 1.0)
            if not ready:
                if adapter.build_active:
                    emit_adapter_events(adapter)
                continue
            raw = sys.stdin.buffer.readline()
            if not raw:
                return
            if len(raw) > MAX_LINE_BYTES:
                error("message_too_large", f"limit is {MAX_LINE_BYTES} bytes")
                continue
            try:
                data = json.loads(raw.decode("utf-8"))
                if data.get("protocol_version") != VERSION:
                    error(
                        "version_mismatch",
                        f"supported version is {VERSION}",
                        data.get("correlation_id"),
                    )
                    continue
                message = data.get("message")
                if isinstance(message, dict) and message.get("type") == "hello":
                    try:
                        adapter.shutdown()
                        adapter, generation, capabilities = configure_compatibility(
                            message
                        )
                    except (CompatibilityError, ServerUnavailable) as exc:
                        error(
                            "compatibility_negotiation_failed",
                            str(exc),
                            data.get("correlation_id"),
                        )
                    else:
                        emit(
                            {
                                "type": "hello_ack",
                                "bitbake_version": adapter.version,
                                "compatibility_generation": generation,
                                "capabilities": capabilities,
                            },
                            data.get("correlation_id"),
                        )
                    continue
                if not handle(message, data.get("correlation_id"), adapter):
                    return
                if adapter.build_active:
                    emit_adapter_events(adapter)
            except (UnicodeDecodeError, json.JSONDecodeError, AttributeError) as exc:
                error("malformed_command", str(exc))
    finally:
        selector.close()
        try:
            adapter.shutdown()
        except Exception as exc:
            print(f"bridge shutdown warning: {exc}", file=sys.stderr)


if __name__ == "__main__":
    if sys.argv[1:] == ["--probe-capabilities"]:
        isolate_protocol_output()
        try:
            report = probe_backend_capabilities()
        except Exception as exc:
            print(f"backend capability probe failed: {exc}", file=sys.stderr)
            sys.exit(1)
        protocol_output.write(json.dumps(report) + "\n")
        protocol_output.flush()
    else:
        main()
