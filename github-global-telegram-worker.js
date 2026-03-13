/**
 * GitHub App Webhook -> Telegram notifier (Cloudflare Worker)
 *
 * Required environment variables:
 * - TELEGRAM_BOT_TOKEN
 * - TELEGRAM_CHAT_ID
 * - GITHUB_WEBHOOK_SECRET
 *
 * Optional:
 * - INCLUDE_EVENTS (comma-separated, default: issues,pull_request,push)
 */

export default {
  async fetch(request, env) {
    if (request.method !== "POST") {
      return new Response("ok", { status: 200 });
    }

    const signature = request.headers.get("x-hub-signature-256") || "";
    const event = request.headers.get("x-github-event") || "";
    const delivery = request.headers.get("x-github-delivery") || "unknown";
    const rawBody = await request.text();

    if (!env.GITHUB_WEBHOOK_SECRET) {
      return new Response("Missing GITHUB_WEBHOOK_SECRET", { status: 500 });
    }

    const valid = await verifyGitHubSignature(
      rawBody,
      signature,
      env.GITHUB_WEBHOOK_SECRET
    );
    if (!valid) {
      return new Response("Invalid signature", { status: 401 });
    }

    let payload;
    try {
      payload = JSON.parse(rawBody);
    } catch {
      return new Response("Invalid JSON", { status: 400 });
    }

    if (event === "ping") {
      return new Response("pong", { status: 200 });
    }

    const includeEvents = getIncludeEvents(env.INCLUDE_EVENTS);
    if (!includeEvents.has(event)) {
      return new Response(`Ignored event: ${event}`, { status: 200 });
    }

    const message = buildMessage(event, payload);
    if (!message) {
      return new Response(`Ignored action for event: ${event}`, { status: 200 });
    }

    try {
      await sendTelegram(message, env);
      return new Response(`ok: ${delivery}`, { status: 200 });
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      return new Response(`Telegram send failed: ${msg}`, { status: 500 });
    }
  },
};

function getIncludeEvents(value) {
  const defaultEvents = ["issues", "pull_request", "push"];
  if (!value) return new Set(defaultEvents);
  return new Set(
    String(value)
      .split(",")
      .map((v) => v.trim())
      .filter(Boolean)
  );
}

async function verifyGitHubSignature(rawBody, signatureHeader, secret) {
  if (!signatureHeader || !signatureHeader.startsWith("sha256=")) {
    return false;
  }

  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );

  const digest = await crypto.subtle.sign(
    "HMAC",
    key,
    new TextEncoder().encode(rawBody)
  );

  const expected = `sha256=${toHex(digest)}`;
  return timingSafeEqual(expected, signatureHeader);
}

function toHex(buffer) {
  return [...new Uint8Array(buffer)]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function timingSafeEqual(a, b) {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i += 1) {
    diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return diff === 0;
}

function buildMessage(event, payload) {
  if (event === "issues") return buildIssueMessage(payload);
  if (event === "pull_request") return buildPullRequestMessage(payload);
  if (event === "push") return buildPushMessage(payload);
  return "";
}

function buildIssueMessage(payload) {
  const action = payload?.action || "";
  const allowed = new Set(["opened", "edited", "closed", "reopened"]);
  if (!allowed.has(action)) return "";

  const issue = payload?.issue;
  if (!issue) return "";

  const repo = payload?.repository?.full_name || "-";
  const actor = payload?.sender?.login || "-";

  const title = `<b>Issue ${escapeHtml(action.toUpperCase())}</b>`;
  return [
    title,
    `저장소: <code>${escapeHtml(repo)}</code>`,
    `#${issue.number} ${escapeHtml(issue.title || "")}`,
    `작성/수정자: ${escapeHtml(actor)}`,
    `<a href="${escapeHtml(issue.html_url || "")}">이슈 보기</a>`,
  ].join("\n\n");
}

function buildPullRequestMessage(payload) {
  const action = payload?.action || "";
  const allowed = new Set([
    "opened",
    "reopened",
    "synchronize",
    "ready_for_review",
    "review_requested",
    "closed",
  ]);
  if (!allowed.has(action)) return "";

  const pr = payload?.pull_request;
  if (!pr) return "";

  const repo = payload?.repository?.full_name || "-";
  const actor = payload?.sender?.login || "-";
  const prState = pr.merged ? "MERGED" : action.toUpperCase();
  const mergeInfo = pr.merged
    ? `머지 커밋: <code>${escapeHtml(pr.merge_commit_sha || "-")}</code>`
    : `상태: ${escapeHtml(pr.state || "-")}`;

  return [
    `<b>PR ${escapeHtml(prState)}</b>`,
    `저장소: <code>${escapeHtml(repo)}</code>`,
    `#${pr.number} ${escapeHtml(pr.title || "")}`,
    `작성/수정자: ${escapeHtml(actor)}`,
    mergeInfo,
    `<a href="${escapeHtml(pr.html_url || "")}">PR 보기</a>`,
  ].join("\n\n");
}

function buildPushMessage(payload) {
  const repo = payload?.repository?.full_name || "-";
  const actor = payload?.sender?.login || payload?.pusher?.name || "-";
  const ref = payload?.ref || "";
  const branch = ref.startsWith("refs/heads/")
    ? ref.replace("refs/heads/", "")
    : ref || "-";
  const commits = Array.isArray(payload?.commits) ? payload.commits : [];
  const shown = commits.slice(0, 5);

  const lines = shown.length
    ? shown
        .map((c) => {
          const sha = (c.id || "").slice(0, 7);
          const msg = cut(firstLine(c.message || ""), 90);
          const author = c.author?.name || "-";
          return `- <code>${escapeHtml(sha)}</code> ${escapeHtml(msg)} (${escapeHtml(author)})`;
        })
        .join("\n")
    : "- 커밋 정보 없음";

  const remaining = commits.length - shown.length;

  return [
    `<b>Push on ${escapeHtml(branch)}</b>`,
    `저장소: <code>${escapeHtml(repo)}</code>`,
    `푸시 사용자: ${escapeHtml(actor)}`,
    `커밋 수: ${commits.length}`,
    lines,
    remaining > 0 ? `…외 ${remaining}개 커밋` : "",
    payload?.compare
      ? `<a href="${escapeHtml(payload.compare)}">비교 보기</a>`
      : "",
  ]
    .filter(Boolean)
    .join("\n\n");
}

async function sendTelegram(message, env) {
  if (!env.TELEGRAM_BOT_TOKEN || !env.TELEGRAM_CHAT_ID) {
    throw new Error("Missing TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID");
  }

  const endpoint = `https://api.telegram.org/bot${env.TELEGRAM_BOT_TOKEN}/sendMessage`;

  const body = new URLSearchParams({
    chat_id: env.TELEGRAM_CHAT_ID,
    text: message,
    parse_mode: "HTML",
    disable_web_page_preview: "true",
  });

  const resp = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body,
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`status=${resp.status}, body=${text}`);
  }
}

function firstLine(text) {
  return String(text).split("\n")[0].trim();
}

function cut(text, max) {
  const s = String(text || "");
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

function escapeHtml(input) {
  return String(input || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
