#!/usr/bin/env python3
"""NDJSON BitBake bridge. Diagnostics are deliberately written only to stderr."""
import json, os, sys
VERSION=1; MAX_LINE_BYTES=1024*1024
sequence=0
def emit(message, correlation_id=None):
    global sequence
    sequence += 1
    value={"protocol_version": VERSION, "sequence": sequence, "message": message}
    if correlation_id is not None: value["correlation_id"]=correlation_id
    sys.stdout.write(json.dumps(value, ensure_ascii=False, separators=(",",":"))+"\n"); sys.stdout.flush()
def error(code,message,correlation_id=None): emit({"type":"command_failed","code":code,"message":message},correlation_id)
def workspace():
    variables={key:os.environ[key] for key in ("MACHINE","DISTRO","DL_DIR","SSTATE_DIR","TMPDIR","BB_NUMBER_THREADS","PARALLEL_MAKE") if key in os.environ}
    return {"type":"workspace","data":{"build_dir":os.getcwd(),"variables":variables,"layers":[],"recipes":[]}}
def handle(command,correlation_id):
    kind=command.get("type") if isinstance(command,dict) else None
    if kind=="hello": emit({"type":"hello_ack","bitbake_version":None},correlation_id)
    elif kind=="inspect_workspace": emit(workspace(),correlation_id)
    elif kind=="start_build":
        targets=command.get("targets")
        if not isinstance(targets,list) or not all(isinstance(t,str) and t for t in targets): error("invalid_request","start_build requires non-empty string targets",correlation_id)
        else: emit({"type":"build_started"},correlation_id); error("unsupported","direct build control requires a compatible BitBake server adapter",correlation_id)
    elif kind in ("cancel_build","list_recipes","list_layers","get_variable"): error("unsupported","command needs a connected BitBake server",correlation_id)
    elif kind=="shutdown": emit({"type":"bridge_shutdown"},correlation_id); return False
    else: error("unknown_command",f"unknown command: {kind!r}",correlation_id)
    return True
def main():
    for raw in sys.stdin.buffer:
        if len(raw)>MAX_LINE_BYTES: error("message_too_large",f"limit is {MAX_LINE_BYTES} bytes"); continue
        try:
            data=json.loads(raw.decode("utf-8"))
            if data.get("protocol_version")!=VERSION: error("version_mismatch",f"supported version is {VERSION}",data.get("correlation_id")); continue
            if not handle(data.get("message"),data.get("correlation_id")): return
        except (UnicodeDecodeError,json.JSONDecodeError,AttributeError) as exc: error("malformed_command",str(exc))
if __name__=="__main__": main()
