# Compatibility

Yoctui requires stable Rust and an initialized Yocto environment for production BitBake control. The bridge negotiates protocol version 1 and reports unsupported server commands explicitly. BitBake event-object adaptation is intentionally localized to the Python bridge; unknown events are retained as diagnostics rather than guessed.
