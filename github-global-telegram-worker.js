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
  const labels = Array.isArray(issue.labels) && issue.labels.length
    ? issue.labels
        .map((label) =>
          typeof label === "string" ? label : label?.name || ""
        )
        .filter(Boolean)
        .join(", ")
    : "없음";
  const stateText = issue.state === "open"
    ? "열림"
    : issue.state === "closed"
      ? "닫힘"
      : issue.state || "-";

  return [
    `🧩 <b>이슈 ${escapeHtml(issueActionLabel(action))}</b>`,
    `저장소: <code>${escapeHtml(repo)}</code>`,
    `번호: <code>#${issue.number}</code>`,
    `제목: ${escapeHtml(issue.title || "(제목 없음)")}`,
    `작성자: ${escapeHtml(actor)}`,
    `현재 상태: ${escapeHtml(stateText)}`,
    `라벨: ${escapeHtml(labels)}`,
    issue.html_url
      ? `<a href="${escapeHtml(issue.html_url)}">GitHub에서 이슈 보기</a>`
      : "",
  ]
    .filter(Boolean)
    .join("\n");
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
  const head = pr.head?.ref || "-";
  const base = pr.base?.ref || "-";
  const stateText = pr.merged ? "merged" : pr.state || "-";
  const draftText = pr.draft ? "예" : "아니오";
  const commitCount =
    typeof pr.commits === "number" ? `${pr.commits}` : "-";
  const fileCount =
    typeof pr.changed_files === "number" ? `${pr.changed_files}` : "-";
  const requestedReviewer = payload?.requested_reviewer?.login || "";

  return [
    `🔀 <b>PR ${escapeHtml(pullRequestActionLabel(action, pr))}</b>`,
    `저장소: <code>${escapeHtml(repo)}</code>`,
    `번호: <code>#${pr.number}</code>`,
    `제목: ${escapeHtml(pr.title || "(제목 없음)")}`,
    `작성자: ${escapeHtml(actor)}`,
    `브랜치: <code>${escapeHtml(head)} → ${escapeHtml(base)}</code>`,
    `상태: ${escapeHtml(stateText)}`,
    `Draft: ${escapeHtml(draftText)}`,
    `변경 규모: 커밋 ${escapeHtml(commitCount)}개 / 파일 ${escapeHtml(fileCount)}개`,
    action === "review_requested" && requestedReviewer
      ? `요청된 리뷰어: ${escapeHtml(requestedReviewer)}`
      : "",
    pr.merged && pr.merge_commit_sha
      ? `머지 커밋: <code>${escapeHtml(shortSha(pr.merge_commit_sha))}</code>`
      : "",
    pr.html_url
      ? `<a href="${escapeHtml(pr.html_url)}">GitHub에서 PR 보기</a>`
      : "",
  ]
    .filter(Boolean)
    .join("\n");
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
        .map((c, index) => {
          const sha = shortSha(c.id || "");
          const msg = cut(firstLine(c.message || ""), 90);
          const author = c.author?.name || "-";
          return `${index + 1}. <code>${escapeHtml(sha)}</code> ${escapeHtml(msg)} <i>(${escapeHtml(author)})</i>`;
        })
        .join("\n")
    : "없음";

  const remaining = commits.length - shown.length;

  return [
    `📦 <b>${escapeHtml(pushActionLabel(payload, commits))}</b>`,
    `저장소: <code>${escapeHtml(repo)}</code>`,
    `브랜치: <code>${escapeHtml(branch)}</code>`,
    `푸시 사용자: ${escapeHtml(actor)}`,
    `커밋 수: <code>${commits.length}</code>`,
    "커밋 목록:",
    lines,
    remaining > 0 ? `외 ${remaining}개 커밋 더 있음` : "",
    payload?.compare
      ? `<a href="${escapeHtml(payload.compare)}">GitHub에서 변경 비교 보기</a>`
      : "",
  ]
    .filter(Boolean)
    .join("\n");
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

function issueActionLabel(action) {
  if (action === "opened") return "생성";
  if (action === "edited") return "수정";
  if (action === "closed") return "종료";
  if (action === "reopened") return "재오픈";
  return action || "-";
}

function pullRequestActionLabel(action, pr) {
  if (action === "opened") return "생성";
  if (action === "reopened") return "재오픈";
  if (action === "synchronize") return "커밋 업데이트";
  if (action === "ready_for_review") return "리뷰 준비 완료";
  if (action === "review_requested") return "리뷰 요청";
  if (action === "closed") return pr?.merged ? "머지 완료" : "종료";
  return action || "-";
}

function pushActionLabel(payload, commits) {
  if (payload?.deleted) return "브랜치 삭제";
  if (payload?.forced) return "강제 푸시";
  const prNumber = mergedPullRequestNumber(commits);
  if (prNumber) return `머지 반영 (PR #${prNumber})`;
  if (containsMergeCommit(commits)) return "머지 반영";
  if (payload?.created) return "새 브랜치 첫 푸시";
  return "푸시";
}

function containsMergeCommit(commits) {
  if (!Array.isArray(commits) || commits.length === 0) return false;
  return commits.some((commit) => {
    const line = firstLine(commit?.message || "");
    return /^Merge pull request #\d+/i.test(line) || /^Merge branch\b/i.test(line);
  });
}

function mergedPullRequestNumber(commits) {
  if (!Array.isArray(commits) || commits.length === 0) return "";
  for (const commit of commits) {
    const line = firstLine(commit?.message || "");
    const match = line.match(/^Merge pull request #(\d+)/i);
    if (match) return match[1];
  }
  return "";
}

function shortSha(sha) {
  return String(sha || "").slice(0, 7);
}

function escapeHtml(input) {
  return String(input || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
