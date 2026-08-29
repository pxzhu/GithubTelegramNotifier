import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import readline from "node:readline";
import test from "node:test";
import { fileURLToPath } from "node:url";

const directory = path.dirname(fileURLToPath(import.meta.url));
const adapterPath = path.resolve(directory, "../adapter.mjs");

class AdapterClient {
  constructor() {
    this.frames = [];
    this.pending = new Map();
    this.waiters = new Set();
    this.nextId = 1;
    this.exited = false;
    this.stderr = "";
    this.child = spawn(process.execPath, [adapterPath], {
      cwd: path.dirname(adapterPath),
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    this.lines = readline.createInterface({ input: this.child.stdout, crlfDelay: Infinity });
    this.lines.on("line", (line) => this.#acceptFrame(JSON.parse(line)));
    this.child.stderr.setEncoding("utf8");
    this.child.stderr.on("data", (chunk) => {
      this.stderr += chunk;
    });
    this.exitPromise = new Promise((resolve, reject) => {
      this.child.once("error", reject);
      this.child.once("exit", (code, signal) => {
        this.exited = true;
        resolve({ code, signal });
      });
    });
  }

  #acceptFrame(frame) {
    this.frames.push(frame);
    if (Object.hasOwn(frame, "id") && this.pending.has(frame.id)) {
      const { resolve, reject } = this.pending.get(frame.id);
      this.pending.delete(frame.id);
      if (frame.error) {
        const error = new Error(frame.error.message);
        error.response = frame;
        reject(error);
      } else {
        resolve(frame.result);
      }
    }
    for (const waiter of [...this.waiters]) {
      if (waiter.predicate(frame)) {
        this.waiters.delete(waiter);
        clearTimeout(waiter.timer);
        waiter.resolve(frame);
      }
    }
  }

  request(method, params = {}) {
    const id = `request-${this.nextId++}`;
    const promise = new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
    this.child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
    return promise;
  }

  sendRaw(line) {
    this.child.stdin.write(`${line}\n`);
  }

  waitFor(predicate, timeoutMs = 3_000) {
    const existing = this.frames.find(predicate);
    if (existing !== undefined) {
      return Promise.resolve(existing);
    }
    return new Promise((resolve, reject) => {
      const waiter = {
        predicate,
        resolve,
        timer: setTimeout(() => {
          this.waiters.delete(waiter);
          reject(new Error("Timed out waiting for adapter frame."));
        }, timeoutMs),
      };
      this.waiters.add(waiter);
    });
  }

  events(runId) {
    return this.frames
      .filter((frame) => frame.method === "agent.event" && frame.params.run_id === runId)
      .map((frame) => frame.params);
  }

  async stop() {
    if (this.exited) {
      return;
    }
    try {
      await this.request("provider.shutdown", {});
    } catch {
      this.child.stdin.end();
    }
    const exit = await this.exitPromise;
    assert.equal(exit.code, 0, this.stderr);
  }
}

async function fixture(t) {
  const workspace = await mkdtemp(path.join(os.tmpdir(), "harness-provider-test-"));
  const client = new AdapterClient();
  t.after(async () => {
    await client.stop();
    await rm(workspace, { recursive: true, force: true });
  });
  return { client, workspace };
}

async function initialize(client) {
  return client.request("provider.initialize", {
    protocol: { min: "1.0", max: "1.0" },
    host_version: "0.1.0",
    platform: { os: process.platform, arch: process.arch },
    locale: "en-US",
    max_frame_bytes: 1_048_576,
    features: ["resume"],
  });
}

function execution(workspace, runId, text = "hello") {
  return {
    run_id: runId,
    task_id: `task:${runId}`,
    conversation: {
      provider_session_id: null,
      messages: [{ role: "user", parts: [{ type: "text", text }] }],
    },
    model: "reference-echo-v1",
    capabilities_required: ["offline"],
    workspace: {
      root: workspace,
      read_paths: [workspace],
      write_paths: [],
      symlink_policy: "deny-escape",
    },
    tools: [],
    limits: { max_output_bytes: 65_536, max_tool_calls: 0, budget_units: null },
    policy: {
      network: "offline",
      data_labels_allowed: ["PROJECT"],
      raw_reasoning: "discard",
    },
    idempotency_key: `key:${runId}`,
  };
}

function terminalFor(runId) {
  return (frame) =>
    frame.method === "agent.event" &&
    frame.params.run_id === runId &&
    ["COMPLETED", "FAILED", "CANCELLED", "INTERRUPTED"].includes(frame.params.kind);
}

test("negotiates protocol and reports status, capabilities, and models", async (t) => {
  const { client } = await fixture(t);
  const handshake = await initialize(client);
  assert.equal(handshake.protocol, "1.0");
  assert.equal(handshake.adapter.id, "harness.reference-generic");
  assert.match(handshake.capability_revision, /^sha256:[a-f0-9]{64}$/);

  const status = await client.request("provider.getStatus");
  assert.equal(status.state, "CONNECTED");
  const capabilities = await client.request("provider.getCapabilities");
  assert.deepEqual(capabilities.capabilities, [
    "reasoning",
    "local",
    "offline",
    "cheap",
    "fast",
    "session_resume",
  ]);
  const models = await client.request("provider.getModels");
  assert.equal(models.models[0].id, "reference-echo-v1");
});

test("streams normalized events after acceptance and completes exactly once", async (t) => {
  const { client, workspace } = await fixture(t);
  await initialize(client);
  const runId = "run:complete";
  const accepted = await client.request("provider.execute", {
    ...execution(workspace, runId, "deterministic output"),
    options: { chunk_size: 5, delay_ms: 0 },
  });
  assert.equal(accepted.accepted, true);
  await client.waitFor(terminalFor(runId));

  const events = client.events(runId);
  assert.deepEqual(
    events.map((event) => event.sequence),
    events.map((_, index) => index + 1),
  );
  assert.equal(events[0].kind, "STARTED");
  assert.equal(events[1].kind, "SESSION_CREATED");
  assert.equal(events.at(-1).kind, "COMPLETED");
  assert.equal(events.filter((event) => terminalFor(runId)({ method: "agent.event", params: event })).length, 1);
  assert.equal(
    events.filter((event) => event.kind === "TEXT_DELTA").map((event) => event.payload.delta).join(""),
    "Reference adapter received: deterministic output",
  );
  const usage = events.find((event) => event.kind === "USAGE").payload;
  assert.equal(usage.input_tokens, null);
  assert.equal(usage.output_tokens, null);
  assert.equal(usage.request_count, 1);

  const responseIndex = client.frames.findIndex(
    (frame) => frame.result?.run_id === runId && frame.result?.accepted === true,
  );
  const eventIndex = client.frames.findIndex(
    (frame) => frame.method === "agent.event" && frame.params.run_id === runId,
  );
  assert.ok(responseIndex >= 0 && responseIndex < eventIndex);
});

test("cooperatively cancels an active run with one terminal event", async (t) => {
  const { client, workspace } = await fixture(t);
  await initialize(client);
  const runId = "run:cancel";
  await client.request("provider.execute", {
    ...execution(workspace, runId, "x".repeat(300)),
    options: { chunk_size: 1, delay_ms: 10 },
  });
  await client.waitFor(
    (frame) =>
      frame.method === "agent.event" &&
      frame.params.run_id === runId &&
      frame.params.kind === "TEXT_DELTA",
  );
  const cancellation = await client.request("provider.cancel", { run_id: runId });
  assert.equal(cancellation.state, "CANCELLING");
  await client.waitFor(terminalFor(runId));

  const kinds = client.events(runId).map((event) => event.kind);
  assert.ok(kinds.includes("CANCEL_REQUESTED"));
  assert.equal(kinds.at(-1), "CANCELLED");
  assert.equal(kinds.filter((kind) => ["COMPLETED", "FAILED", "CANCELLED", "INTERRUPTED"].includes(kind)).length, 1);
  const usage = await client.request("provider.getUsage", { run_id: runId });
  assert.equal(usage.terminal_kind, "CANCELLED");
});

test("resumes an in-memory session with a fresh run and aggregates usage", async (t) => {
  const { client, workspace } = await fixture(t);
  await initialize(client);
  const first = await client.request("provider.execute", execution(workspace, "run:first", "first"));
  await client.waitFor(terminalFor("run:first"));

  const secondRequest = execution(workspace, "run:second", "second");
  const resumed = await client.request("provider.resume", {
    ...secondRequest,
    session_id: first.session_id,
  });
  assert.equal(resumed.resumed, true);
  assert.equal(resumed.session_id, first.session_id);
  await client.waitFor(terminalFor("run:second"));
  assert.ok(client.events("run:second").some((event) => event.kind === "SESSION_RESUMED"));

  const usage = await client.request("provider.getUsage");
  assert.equal(usage.request_count, 2);
  assert.equal(usage.input_tokens, null);
  assert.equal(usage.is_final, true);
});

test("rejects unsafe workspace grants and unsupported capabilities before acceptance", async (t) => {
  const { client, workspace } = await fixture(t);
  await initialize(client);

  const unsafe = execution(workspace, "run:unsafe");
  unsafe.workspace.read_paths = [path.resolve(workspace, "..")];
  await assert.rejects(
    client.request("provider.execute", unsafe),
    (error) => error.response.error.data.code === "POLICY_BLOCKED",
  );
  assert.equal(client.events("run:unsafe").length, 0);

  const unsupported = execution(workspace, "run:unsupported");
  unsupported.capabilities_required = ["shell"];
  await assert.rejects(
    client.request("provider.execute", unsupported),
    (error) => error.response.error.data.code === "CAPABILITY_UNAVAILABLE",
  );
  assert.equal(client.events("run:unsupported").length, 0);
});

test("isolates malformed requests and emits a deterministic failed run", async (t) => {
  const { client, workspace } = await fixture(t);
  client.sendRaw("{not-json");
  const parseFailure = await client.waitFor(
    (frame) => frame.id === null && frame.error?.code === -32700,
  );
  assert.equal(parseFailure.error.data.code, "PROTOCOL_ERROR");

  await initialize(client);
  const runId = "run:failure";
  await client.request("provider.execute", {
    ...execution(workspace, runId, "failure fixture"),
    options: { chunk_size: 2, delay_ms: 0, fail_after_chunks: 2 },
  });
  const terminal = await client.waitFor(terminalFor(runId));
  assert.equal(terminal.params.kind, "FAILED");
  assert.equal(terminal.params.payload.code, "EXECUTION_FAILED");
  assert.equal(
    client.events(runId).filter((event) => ["COMPLETED", "FAILED", "CANCELLED", "INTERRUPTED"].includes(event.kind)).length,
    1,
  );
});
