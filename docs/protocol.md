# Bridge protocol

Each UTF-8 line is one JSON envelope: `protocol_version` (currently 1), monotonic `sequence`, optional `correlation_id`, and tagged `message`. Maximum line length is 1 MiB. Both the Python bridge and Rust transport reject oversized partial lines before processing a complete frame. Unsupported versions, malformed input, and unknown commands produce typed `command_failed` responses. Unknown incoming events deserialize safely.

Commands: `hello`, `inspect_workspace`, `start_build`, `cancel_build`, `list_recipes`, `list_layers`, `get_variable`, `shutdown`. Events: `hello_ack`, `workspace`, lifecycle/task/log events, `command_failed`, `protocol_error`, and `bridge_shutdown`. New optional fields are allowed; consumers must not reinterpret unknown events.
