# Current Task

## Task

**ID:** CLIENT-RUNTIME-QA-SECURITY-001
**Title:** Move QA and security jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed QA, CVE, SPDX and security mapper jobs through daemon ownership.

## Verification

```bash
cargo test -p yoctui client_runtime_qa_security
```
