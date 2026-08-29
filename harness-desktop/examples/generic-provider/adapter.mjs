#!/usr/bin/env node

import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";

const PROTOCOL_VERSION = "1.0";
const ADAPTER_ID = "harness.reference-generic";
const ADAPTER_VERSION = "0.1.0";
const MODEL_ID = "reference-echo-v1";
const MAX_FRAME_BYTES = 1_048_576;
const MAX_TEXT_BYTES = 65_536;
const MAX_MESSAGES = 100;
const MAX_CHUNKS = 2_048;

const CAPABILITIES = Object.freeze([
  "reasoning",
  "local",
  "offline",
  "cheap",
  "fast",
  "session_resume",
]);

const CAPABILITY_REVISION = `sha256:${createHash("sha256")
  .update(JSON.stringify(CAPABILITIES))
  .digest("hex")}`;

const MODEL = Object.freeze({
  id: MODEL_ID,
  display_name: "Deterministic Reference Echo",
  version: ADAPTER_VERSION,
  capabilities: CAPABILITIES,
  context_length: null,
  input_modalities: ["text"],
  output_modalities: ["text"],
  availability: "AVAILABLE",
  cost_tier: "FREE",
  latency_tier: "FAST",
  local: true,
  metadata_revision: CAPABILITY_REVISION,
});

class RpcFault extends Error {
  constructor(rpcCode, message, data) {
    super(message);
    this.name = "RpcFault";
    this.rpcCode = rpcCode;
    this.data = data;
  }
}

const state = {
  initialized: false,
  shuttingDown: false,
  closeAfterResponse: false,
  maxFrameBytes: MAX_FRAME_BYTES,
  activeRuns: new Map(),
  runHistory: new Map(),
  sessions: new Map(),
  acceptedRunCount: 0,
  completedDurationMs: 0,
};

function typedError(code, retryable = false, userAction = null) {
  return {
    code,
    retryable,
    retry_after_ms: null,
    user_action: userAction,
    details: { redacted: true },
  };
}

function invalidParams(message) {
  throw new RpcFault(-32602, message, typedError("PROTOCOL_ERROR"));
}

function providerFault(code, message, retryable = false) {
  throw new RpcFault(-32000, message, typedError(code, retryable));
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function requireObject(value, label) {
  if (!isObject(value)) {
    invalidParams(`${label} must be an object.`);
  }
  return value;
}

function assertOnlyKeys(value, allowedKeys, label) {
  const unknown = Object.keys(value).find((key) => !allowedKeys.has(key));
  if (unknown !== undefined) {
    invalidParams(`${label} contains an unsupported field.`);
  }
}

function requireIdentifier(value, label) {
  if (
    typeof value !== "string" ||
    value.length < 1 ||
    value.length > 128 ||
    !/^[A-Za-z0-9][A-Za-z0-9._:-]*$/.test(value)
  ) {
    invalidParams(`${label} must be a safe identifier between 1 and 128 characters.`);
  }
  return value;
}

function requireInteger(value, label, minimum, maximum) {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    invalidParams(`${label} must be an integer from ${minimum} through ${maximum}.`);
  }
  return value;
}

function requireString(value, label, maximumLength = 4_096) {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.length > maximumLength ||
    value.includes("\0")
  ) {
    invalidParams(`${label} must be a non-empty string within its size limit.`);
  }
  return value;
}

function isWithin(root, candidate) {
  const relative = path.relative(root, candidate);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}

function validateWorkspace(value) {
  const workspace = requireObject(value, "workspace");
  assertOnlyKeys(
    workspace,
    new Set(["root", "read_paths", "write_paths", "symlink_policy"]),
    "workspace",
  );

  const rootInput = requireString(workspace.root, "workspace.root");
  if (!path.isAbsolute(rootInput)) {
    invalidParams("workspace.root must be absolute.");
  }
  const root = path.resolve(rootInput);

  if (!Array.isArray(workspace.read_paths) || workspace.read_paths.length > 32) {
    invalidParams("workspace.read_paths must be an array with at most 32 entries.");
  }
  for (const entry of workspace.read_paths) {
    const input = requireString(entry, "workspace.read_paths entry");
    if (!path.isAbsolute(input) || !isWithin(root, path.resolve(input))) {
      providerFault("POLICY_BLOCKED", "A read path is outside the declared workspace.");
    }
  }

  if (!Array.isArray(workspace.write_paths)) {
    invalidParams("workspace.write_paths must be an array.");
  }
  if (workspace.write_paths.length !== 0) {
    providerFault("CAPABILITY_UNAVAILABLE", "This reference adapter is read-only.");
  }
  if (workspace.symlink_policy !== "deny-escape") {
    providerFault("POLICY_BLOCKED", "workspace.symlink_policy must be deny-escape.");
  }

  return root;
}

function validateConversation(value) {
  const conversation = requireObject(value, "conversation");
  assertOnlyKeys(
    conversation,
    new Set(["provider_session_id", "messages"]),
    "conversation",
  );
  if (!Array.isArray(conversation.messages) || conversation.messages.length === 0) {
    invalidParams("conversation.messages must be a non-empty array.");
  }
  if (conversation.messages.length > MAX_MESSAGES) {
    invalidParams(`conversation.messages cannot exceed ${MAX_MESSAGES} entries.`);
  }

  let totalBytes = 0;
  let lastUserText = null;
  for (const message of conversation.messages) {
    const item = requireObject(message, "message");
    assertOnlyKeys(item, new Set(["role", "parts"]), "message");
    if (!["system", "user", "assistant", "agent"].includes(item.role)) {
      invalidParams("message.role is unsupported.");
    }
    if (!Array.isArray(item.parts) || item.parts.length === 0 || item.parts.length > 32) {
      invalidParams("message.parts must contain between 1 and 32 entries.");
    }

    const textParts = [];
    for (const part of item.parts) {
      const contentPart = requireObject(part, "message part");
      assertOnlyKeys(contentPart, new Set(["type", "text"]), "message part");
      if (contentPart.type !== "text" || typeof contentPart.text !== "string") {
        providerFault("CAPABILITY_UNAVAILABLE", "Only text message parts are supported.");
      }
      const byteLength = Buffer.byteLength(contentPart.text, "utf8");
      totalBytes += byteLength;
      if (totalBytes > MAX_TEXT_BYTES) {
        invalidParams(`Conversation text cannot exceed ${MAX_TEXT_BYTES} UTF-8 bytes.`);
      }
      textParts.push(contentPart.text);
    }
    if (item.role === "user") {
      lastUserText = textParts.join("");
    }
  }

  if (typeof lastUserText !== "string" || lastUserText.trim().length === 0) {
    invalidParams("conversation must contain a non-empty user text message.");
  }
  return lastUserText.trim();
}

function validateCapabilities(value) {
  if (!Array.isArray(value) || value.length > 32) {
    invalidParams("capabilities_required must be an array with at most 32 entries.");
  }
  for (const capability of value) {
    if (typeof capability !== "string") {
      invalidParams("capabilities_required entries must be strings.");
    }
    if (!CAPABILITIES.includes(capability)) {
      providerFault(
        "CAPABILITY_UNAVAILABLE",
        "A required capability is not available from this provider.",
      );
    }
  }
}

function validateTools(value) {
  if (!Array.isArray(value)) {
    invalidParams("tools must be an array.");
  }
  if (value.length !== 0) {
    providerFault("CAPABILITY_UNAVAILABLE", "This reference adapter does not execute tools.");
  }
}

function validateLimits(value, outputBytes) {
  if (value === undefined) {
    return null;
  }
  const limits = requireObject(value, "limits");
  assertOnlyKeys(
    limits,
    new Set(["deadline", "max_output_bytes", "max_tool_calls", "budget_units"]),
    "limits",
  );
  if (limits.max_output_bytes !== undefined && limits.max_output_bytes !== null) {
    requireInteger(limits.max_output_bytes, "limits.max_output_bytes", 1, MAX_FRAME_BYTES);
    if (outputBytes > limits.max_output_bytes) {
      providerFault("POLICY_BLOCKED", "The deterministic result exceeds max_output_bytes.");
    }
  }
  if (limits.max_tool_calls !== undefined && limits.max_tool_calls !== null) {
    requireInteger(limits.max_tool_calls, "limits.max_tool_calls", 0, 1_000_000);
  }
  if (limits.deadline !== undefined && limits.deadline !== null) {
    if (typeof limits.deadline !== "string" || !Number.isFinite(Date.parse(limits.deadline))) {
      invalidParams("limits.deadline must be an RFC3339 timestamp.");
    }
    const deadlineMs = Date.parse(limits.deadline);
    if (deadlineMs <= Date.now()) {
      providerFault("TIMEOUT", "The execution deadline has already passed.");
    }
    return deadlineMs;
  }
  return null;
}

function validatePolicy(value) {
  if (value === undefined) {
    return;
  }
  const policy = requireObject(value, "policy");
  assertOnlyKeys(
    policy,
    new Set(["network", "data_labels_allowed", "raw_reasoning"]),
    "policy",
  );
  if (policy.network !== undefined && typeof policy.network !== "string") {
    invalidParams("policy.network must be a string.");
  }
  if (
    policy.data_labels_allowed !== undefined &&
    (!Array.isArray(policy.data_labels_allowed) ||
      policy.data_labels_allowed.some((entry) => typeof entry !== "string"))
  ) {
    invalidParams("policy.data_labels_allowed must be an array of strings.");
  }
  if (
    policy.raw_reasoning !== undefined &&
    !["discard", "summary-only"].includes(policy.raw_reasoning)
  ) {
    providerFault("POLICY_BLOCKED", "Raw reasoning cannot be retained by this adapter.");
  }
}

function validateOptions(value, output) {
  if (value === undefined) {
    return { chunkSize: 24, delayMs: 0, failAfterChunks: null };
  }
  const options = requireObject(value, "options");
  assertOnlyKeys(
    options,
    new Set(["chunk_size", "delay_ms", "fail_after_chunks"]),
    "options",
  );
  const chunkSize =
    options.chunk_size === undefined
      ? 24
      : requireInteger(options.chunk_size, "options.chunk_size", 1, 4_096);
  const delayMs =
    options.delay_ms === undefined
      ? 0
      : requireInteger(options.delay_ms, "options.delay_ms", 0, 250);
  const failAfterChunks =
    options.fail_after_chunks === undefined || options.fail_after_chunks === null
      ? null
      : requireInteger(options.fail_after_chunks, "options.fail_after_chunks", 1, MAX_CHUNKS);
  if (Math.ceil(Array.from(output).length / chunkSize) > MAX_CHUNKS) {
    invalidParams(`Execution cannot emit more than ${MAX_CHUNKS} text chunks.`);
  }
  return { chunkSize, delayMs, failAfterChunks };
}

function validateExecution(value, { resumeSessionId = null } = {}) {
  const params = requireObject(value, "params");
  assertOnlyKeys(
    params,
    new Set([
      "run_id",
      "task_id",
      "session_id",
      "conversation",
      "model",
      "capabilities_required",
      "workspace",
      "tools",
      "limits",
      "policy",
      "idempotency_key",
      "options",
    ]),
    "params",
  );

  const runId = requireIdentifier(params.run_id, "run_id");
  if (state.activeRuns.has(runId) || state.runHistory.has(runId)) {
    providerFault("PROTOCOL_ERROR", "run_id has already been used.");
  }
  if (params.task_id !== undefined && params.task_id !== null) {
    requireIdentifier(params.task_id, "task_id");
  }
  if (params.idempotency_key !== undefined && params.idempotency_key !== null) {
    requireIdentifier(params.idempotency_key, "idempotency_key");
  }
  if (params.model !== MODEL_ID) {
    providerFault("MODEL_UNAVAILABLE", "The requested model is unavailable.");
  }
  validateCapabilities(params.capabilities_required ?? []);
  const workspaceRoot = validateWorkspace(params.workspace);
  validateTools(params.tools ?? []);
  validatePolicy(params.policy);

  const userText = validateConversation(params.conversation);
  const prefix = resumeSessionId === null ? "Reference adapter received: " : "Reference adapter resumed: ";
  const output = `${prefix}${userText}`;
  const outputBytes = Buffer.byteLength(output, "utf8");
  const deadlineMs = validateLimits(params.limits, outputBytes);
  const options = validateOptions(params.options, output);

  return {
    runId,
    taskId: params.task_id ?? null,
    sessionId: resumeSessionId ?? `reference:${runId}`,
    model: MODEL_ID,
    workspaceRoot,
    input: userText,
    output,
    deadlineMs,
    ...options,
  };
}

function writeFrame(frame) {
  process.stdout.write(`${JSON.stringify(frame)}\n`);
}

function writeSuccess(id, result) {
  writeFrame({ jsonrpc: "2.0", id, result });
}

function writeFailure(id, fault) {
  writeFrame({
    jsonrpc: "2.0",
    id,
    error: {
      code: fault instanceof RpcFault ? fault.rpcCode : -32603,
      message: fault instanceof RpcFault ? fault.message : "Internal adapter error.",
      data:
        fault instanceof RpcFault
          ? fault.data
          : typedError("EXECUTION_FAILED", false),
    },
  });
}

function emitEvent(run, kind, payload = {}) {
  if (run.terminalKind !== null) {
    return;
  }
  run.sequence += 1;
  writeFrame({
    jsonrpc: "2.0",
    method: "agent.event",
    params: {
      run_id: run.runId,
      sequence: run.sequence,
      event_id: randomUUID(),
      kind,
      occurred_at: new Date().toISOString(),
      payload_version: 1,
      payload,
      provider_metadata: {
        session_id: run.sessionId,
        model: run.model,
      },
    },
  });
}

function usageFor(run, isFinal) {
  const durationMs = Math.max(0, Date.now() - run.startedAt);
  return {
    input_tokens: null,
    output_tokens: null,
    cached_tokens: null,
    reasoning_tokens: null,
    request_count: 1,
    turn_count: null,
    tool_calls: 0,
    duration_ms: durationMs,
    estimated_cost: null,
    currency: null,
    quota_units: null,
    local_tokens: null,
    local_compute_ms: durationMs,
    source: "adapter-runtime",
    is_final: isFinal,
  };
}

function finishRun(run, kind, payload) {
  if (run.terminalKind !== null) {
    return;
  }
  const usage = usageFor(run, true);
  emitEvent(run, "USAGE", usage);
  emitEvent(run, kind, payload);
  run.terminalKind = kind;
  run.usage = usage;
  state.completedDurationMs += usage.duration_ms;
  state.activeRuns.delete(run.runId);
  state.runHistory.set(run.runId, {
    run_id: run.runId,
    session_id: run.sessionId,
    terminal_kind: kind,
    usage,
  });
  const session = state.sessions.get(run.sessionId);
  if (session !== undefined) {
    session.last_run_id = run.runId;
    session.last_terminal_kind = kind;
    session.output = kind === "COMPLETED" ? run.output : null;
  }
  run.resolveTerminal();
}

function waitForNextStep(delayMs) {
  return new Promise((resolve) => setTimeout(resolve, delayMs));
}

async function driveRun(run) {
  try {
    if (run.cancelRequested) {
      finishRun(run, "CANCELLED", {
        code: "CANCELLED",
        message: "Run cancelled cooperatively.",
        retryable: false,
      });
      return;
    }

    run.startedAt = Date.now();
    emitEvent(run, "STARTED", { task_id: run.taskId });
    emitEvent(run, run.isResume ? "SESSION_RESUMED" : "SESSION_CREATED", {
      session_id: run.sessionId,
    });

    const characters = Array.from(run.output);
    let emittedChunks = 0;
    for (let offset = 0; offset < characters.length; offset += run.chunkSize) {
      await waitForNextStep(run.delayMs);
      if (run.cancelRequested) {
        finishRun(run, "CANCELLED", {
          code: "CANCELLED",
          message: "Run cancelled cooperatively.",
          retryable: false,
        });
        return;
      }
      if (run.interruptRequested) {
        finishRun(run, "INTERRUPTED", {
          code: "EXECUTION_FAILED",
          message: "Host input closed before completion.",
          retryable: true,
        });
        return;
      }
      if (run.deadlineMs !== null && Date.now() >= run.deadlineMs) {
        finishRun(run, "FAILED", {
          code: "TIMEOUT",
          message: "The execution deadline elapsed.",
          retryable: true,
        });
        return;
      }

      emitEvent(run, "TEXT_DELTA", {
        delta: characters.slice(offset, offset + run.chunkSize).join(""),
      });
      emittedChunks += 1;
      if (run.failAfterChunks === emittedChunks) {
        finishRun(run, "FAILED", {
          code: "EXECUTION_FAILED",
          message: "Deterministic failure requested by the caller.",
          retryable: false,
        });
        return;
      }
    }

    emitEvent(run, "RESULT", { role: "assistant", text: run.output });
    finishRun(run, "COMPLETED", { status: "completed" });
  } catch {
    finishRun(run, "FAILED", {
      code: "EXECUTION_FAILED",
      message: "The adapter encountered an internal error.",
      retryable: false,
    });
  }
}

function createRun(validated, isResume) {
  let resolveTerminal;
  const terminalPromise = new Promise((resolve) => {
    resolveTerminal = resolve;
  });
  const run = {
    ...validated,
    isResume,
    sequence: 0,
    startedAt: Date.now(),
    cancelRequested: false,
    interruptRequested: false,
    terminalKind: null,
    usage: null,
    terminalPromise,
    resolveTerminal,
  };
  state.activeRuns.set(run.runId, run);
  state.sessions.set(run.sessionId, {
    session_id: run.sessionId,
    model: run.model,
    workspace_root: run.workspaceRoot,
    last_run_id: run.runId,
    last_terminal_kind: null,
    output: null,
  });
  state.acceptedRunCount += 1;
  setImmediate(() => {
    void driveRun(run);
  });
  return run;
}

function ensureInitialized() {
  if (!state.initialized) {
    providerFault("PROTOCOL_ERROR", "provider.initialize must be called first.");
  }
}

function parseProtocolRange(value) {
  const protocol = requireObject(value, "protocol");
  assertOnlyKeys(protocol, new Set(["min", "max"]), "protocol");
  const minimum = requireString(protocol.min, "protocol.min", 16);
  const maximum = requireString(protocol.max, "protocol.max", 16);
  if (minimum > PROTOCOL_VERSION || maximum < PROTOCOL_VERSION) {
    providerFault("INCOMPATIBLE_VERSION", "No compatible protocol version is available.");
  }
}

async function initialize(value) {
  const params = requireObject(value ?? {}, "params");
  assertOnlyKeys(
    params,
    new Set([
      "protocol",
      "host_version",
      "platform",
      "locale",
      "max_frame_bytes",
      "staging_root",
      "features",
    ]),
    "params",
  );
  parseProtocolRange(params.protocol);
  if (params.max_frame_bytes !== undefined) {
    state.maxFrameBytes = Math.min(
      MAX_FRAME_BYTES,
      requireInteger(params.max_frame_bytes, "max_frame_bytes", 4_096, MAX_FRAME_BYTES),
    );
  }
  state.initialized = true;
  return {
    protocol: PROTOCOL_VERSION,
    adapter: { id: ADAPTER_ID, version: ADAPTER_VERSION },
    provider: { id: ADAPTER_ID, display_name: "Generic Reference Provider" },
    capability_revision: CAPABILITY_REVISION,
    features: ["cancellation", "resume"],
    max_frame_bytes: state.maxFrameBytes,
  };
}

async function getStatus(value) {
  ensureInitialized();
  const params = requireObject(value ?? {}, "params");
  assertOnlyKeys(params, new Set(), "params");
  return {
    state: state.shuttingDown ? "DISABLED" : "CONNECTED",
    installed: true,
    authenticated: true,
    available: !state.shuttingDown,
    version: ADAPTER_VERSION,
    reason: state.shuttingDown ? "shutdown" : null,
  };
}

async function getCapabilities(value) {
  ensureInitialized();
  const params = requireObject(value ?? {}, "params");
  assertOnlyKeys(params, new Set(), "params");
  return { revision: CAPABILITY_REVISION, capabilities: [...CAPABILITIES] };
}

async function getModels(value) {
  ensureInitialized();
  const params = requireObject(value ?? {}, "params");
  assertOnlyKeys(params, new Set(), "params");
  return { models: [MODEL] };
}

async function execute(value) {
  ensureInitialized();
  if (state.shuttingDown) {
    providerFault("EXECUTION_FAILED", "The provider is shutting down.");
  }
  const run = createRun(validateExecution(value), false);
  return {
    accepted: true,
    run_id: run.runId,
    session_id: run.sessionId,
  };
}

async function resume(value) {
  ensureInitialized();
  if (state.shuttingDown) {
    providerFault("EXECUTION_FAILED", "The provider is shutting down.");
  }
  const params = requireObject(value, "params");
  const sessionId = requireIdentifier(params.session_id, "session_id");
  if (!state.sessions.has(sessionId)) {
    providerFault("EXECUTION_FAILED", "The requested session is unavailable.");
  }
  const run = createRun(validateExecution(params, { resumeSessionId: sessionId }), true);
  return {
    accepted: true,
    run_id: run.runId,
    session_id: run.sessionId,
    resumed: true,
  };
}

async function cancel(value) {
  ensureInitialized();
  const params = requireObject(value, "params");
  assertOnlyKeys(params, new Set(["run_id"]), "params");
  const runId = requireIdentifier(params.run_id, "run_id");
  const run = state.activeRuns.get(runId);
  if (run === undefined) {
    const history = state.runHistory.get(runId);
    if (history !== undefined) {
      return { accepted: false, run_id: runId, state: history.terminal_kind };
    }
    providerFault("EXECUTION_FAILED", "The requested run is unavailable.");
  }
  if (!run.cancelRequested) {
    run.cancelRequested = true;
    emitEvent(run, "CANCEL_REQUESTED", { requested_by: "host" });
  }
  return { accepted: true, run_id: runId, state: "CANCELLING" };
}

async function getUsage(value) {
  ensureInitialized();
  const params = requireObject(value ?? {}, "params");
  assertOnlyKeys(params, new Set(["run_id"]), "params");
  if (params.run_id !== undefined) {
    const runId = requireIdentifier(params.run_id, "run_id");
    const active = state.activeRuns.get(runId);
    if (active !== undefined) {
      return { run_id: runId, terminal_kind: null, usage: usageFor(active, false) };
    }
    const history = state.runHistory.get(runId);
    if (history === undefined) {
      providerFault("EXECUTION_FAILED", "Usage for the requested run is unavailable.");
    }
    return history;
  }

  const activeDurationMs = [...state.activeRuns.values()].reduce(
    (total, run) => total + Math.max(0, Date.now() - run.startedAt),
    0,
  );
  return {
    input_tokens: null,
    output_tokens: null,
    cached_tokens: null,
    reasoning_tokens: null,
    request_count: state.acceptedRunCount,
    turn_count: null,
    tool_calls: 0,
    duration_ms: state.completedDurationMs + activeDurationMs,
    estimated_cost: null,
    currency: null,
    quota_units: null,
    local_tokens: null,
    local_compute_ms: state.completedDurationMs + activeDurationMs,
    source: "adapter-runtime",
    is_final: state.activeRuns.size === 0,
  };
}

async function shutdown(value) {
  const params = requireObject(value ?? {}, "params");
  assertOnlyKeys(params, new Set(), "params");
  state.shuttingDown = true;
  const active = [...state.activeRuns.values()];
  for (const run of active) {
    if (!run.cancelRequested) {
      run.cancelRequested = true;
      emitEvent(run, "CANCEL_REQUESTED", { requested_by: "shutdown" });
    }
  }
  await Promise.all(active.map((run) => run.terminalPromise));
  state.closeAfterResponse = true;
  return { stopped_runs: active.length, status: "SHUTDOWN" };
}

const handlers = new Map([
  ["provider.initialize", initialize],
  ["provider.getStatus", getStatus],
  ["provider.getCapabilities", getCapabilities],
  ["provider.getModels", getModels],
  ["provider.execute", execute],
  ["provider.cancel", cancel],
  ["provider.resume", resume],
  ["provider.getUsage", getUsage],
  ["provider.shutdown", shutdown],
]);

function validateRequest(frame) {
  if (!isObject(frame)) {
    throw new RpcFault(-32600, "Invalid Request", typedError("PROTOCOL_ERROR"));
  }
  assertOnlyKeys(frame, new Set(["jsonrpc", "id", "method", "params"]), "request");
  if (frame.jsonrpc !== "2.0") {
    throw new RpcFault(-32600, "Invalid Request", typedError("PROTOCOL_ERROR"));
  }
  if (
    !(typeof frame.id === "string" || Number.isSafeInteger(frame.id)) ||
    (typeof frame.id === "string" && frame.id.length > 128)
  ) {
    throw new RpcFault(-32600, "Invalid Request", typedError("PROTOCOL_ERROR"));
  }
  if (typeof frame.method !== "string") {
    throw new RpcFault(-32600, "Invalid Request", typedError("PROTOCOL_ERROR"));
  }
}

async function handleLine(line) {
  if (Buffer.byteLength(line, "utf8") > state.maxFrameBytes) {
    writeFailure(null, new RpcFault(-32600, "Frame exceeds the negotiated size limit.", typedError("PROTOCOL_ERROR")));
    return;
  }

  let frame;
  try {
    frame = JSON.parse(line);
  } catch {
    writeFailure(null, new RpcFault(-32700, "Parse error", typedError("PROTOCOL_ERROR")));
    return;
  }

  const responseId = isObject(frame) && (typeof frame.id === "string" || Number.isSafeInteger(frame.id))
    ? frame.id
    : null;
  try {
    validateRequest(frame);
    const handler = handlers.get(frame.method);
    if (handler === undefined) {
      throw new RpcFault(-32601, "Method not found", typedError("PROTOCOL_ERROR"));
    }
    const result = await handler(frame.params);
    writeSuccess(frame.id, result);
    if (state.closeAfterResponse) {
      setImmediate(() => {
        lineReader.close();
        process.stdin.pause();
      });
    }
  } catch (error) {
    writeFailure(responseId, error);
  }
}

const lineReader = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
  terminal: false,
});

let requestQueue = Promise.resolve();
lineReader.on("line", (line) => {
  requestQueue = requestQueue.then(() => handleLine(line));
});

lineReader.on("close", () => {
  if (state.shuttingDown) {
    return;
  }
  for (const run of state.activeRuns.values()) {
    run.interruptRequested = true;
  }
});

process.stdout.on("error", () => {
  process.exitCode = 1;
});
