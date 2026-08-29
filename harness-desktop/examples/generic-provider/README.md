# Generic CLI Provider reference adapter

This directory is a dependency-free reference implementation of the Harness out-of-process Provider protocol. It runs on Node.js 22, reads newline-delimited JSON-RPC 2.0 requests from stdin, and writes only protocol frames to stdout.

It is deliberately a deterministic echo provider, not an LLM or a wrapper around another executable. It never starts subprocesses, opens network connections, or reads/writes workspace files. Every execution still requires an explicit absolute workspace root; declared read paths must remain lexically inside it, write grants and tools are rejected, and `deny-escape` is mandatory. These constraints make the example safe to run offline and safe to use as the starting point for a real adapter.

## Run and test

```sh
node adapter.mjs
node --test test/*.test.mjs
```

The host sends one JSON object per line. The adapter reserves stdout for responses and `agent.event` notifications. Its supported methods are:

- `provider.initialize`
- `provider.getStatus`
- `provider.getCapabilities`
- `provider.getModels`
- `provider.execute`
- `provider.cancel`
- `provider.resume`
- `provider.getUsage`
- `provider.shutdown`

`provider.initialize` must be the first functional call. The only exception is `provider.shutdown`, which is safe before initialization.

## Minimal exchange

```json
{"jsonrpc":"2.0","id":"init-1","method":"provider.initialize","params":{"protocol":{"min":"1.0","max":"1.0"},"host_version":"0.1.0","max_frame_bytes":1048576}}
{"jsonrpc":"2.0","id":"run-1","method":"provider.execute","params":{"run_id":"demo-run","task_id":"demo-task","conversation":{"provider_session_id":null,"messages":[{"role":"user","parts":[{"type":"text","text":"hello"}]}]},"model":"reference-echo-v1","capabilities_required":["offline"],"workspace":{"root":"/absolute/project","read_paths":["/absolute/project"],"write_paths":[],"symlink_policy":"deny-escape"},"tools":[],"limits":{"max_output_bytes":4096,"max_tool_calls":0,"budget_units":null},"policy":{"network":"offline","data_labels_allowed":["PROJECT"],"raw_reasoning":"discard"},"idempotency_key":"demo-1"}}
```

The `execute` response is written before streaming begins. A run emits monotonically sequenced normalized events and exactly one terminal event: `COMPLETED`, `FAILED`, `CANCELLED`, or `INTERRUPTED`. Token and price fields remain `null` because this adapter has no tokenizer or billable model; it does not invent estimates.

The optional execution fields below exist to make conformance behavior reproducible:

- `options.chunk_size`: Unicode code points per `TEXT_DELTA` (1–4096).
- `options.delay_ms`: cooperative delay between chunks (0–250 ms).
- `options.fail_after_chunks`: emit a deterministic `FAILED` terminal event after N chunks.

Sessions live only for the lifetime of the adapter process. `resume` requires a session ID returned by a prior accepted execution and a fresh `run_id`. This sample has no credential or persistence behavior.

## Security boundary

This adapter intentionally offers no `file_access`, `shell`, `git`, `tool_calling`, or network capability. Adding any of those to a production adapter requires canonical path checks (including symlink resolution), explicit Harness grants, timeout/output limits, redacted errors, and dedicated security tests. Do not turn the manifest's human-readable `entrypoint` metadata into a shell command; a Provider Host should launch a verified executable and fixed argument vector directly.
