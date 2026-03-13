# GithubTelegramNotifier

GitHub App 기반으로 모든 저장소의 이슈/PR/푸시/머지 이벤트를 수집해 Telegram으로 보내는 중앙 웹훅 브리지입니다.

Centralized GitHub App webhook bridge that sends issue, PR, push, and merge events from all repositories to Telegram.

## What it does
- Receives GitHub App webhook events in one endpoint
- Verifies webhook signature (`X-Hub-Signature-256`)
- Formats notifications for Telegram (HTML mode)
- Supports `issues`, `pull_request`, and `push` events

## Architecture
1. GitHub App (installed with **All repositories**) sends webhooks
2. Cloudflare Worker receives and validates requests
3. Worker sends formatted messages to Telegram Bot API

## Files
- `github-global-telegram-worker.js`: Worker source
- `wrangler.toml.example`: example Wrangler config
- `GITHUB_ALL_REPOS_TELEGRAM_SETUP.md`: full setup guide

## Prerequisites
- Cloudflare account
- Telegram bot token and chat ID
- GitHub account (or org admin rights for GitHub App install)

## Required secrets / vars
- `GITHUB_WEBHOOK_SECRET` (secret)
- `TELEGRAM_BOT_TOKEN` (secret)
- `TELEGRAM_CHAT_ID` (var or secret)
- `INCLUDE_EVENTS` (optional, default: `issues,pull_request,push`)

## Local setup
```bash
npm install
cp wrangler.toml.example wrangler.toml
```

## Deploy (Cloudflare Worker)
```bash
export CLOUDFLARE_API_TOKEN='<YOUR_CF_API_TOKEN>'
export CLOUDFLARE_ACCOUNT_ID='<YOUR_CF_ACCOUNT_ID>'
npx wrangler secret put TELEGRAM_BOT_TOKEN
npx wrangler secret put GITHUB_WEBHOOK_SECRET
npx wrangler deploy
```

## GitHub App setup (summary)
- Webhook URL: your deployed Worker URL
- Webhook secret: same value as `GITHUB_WEBHOOK_SECRET`
- Repository permissions: Issues (RO), Pull requests (RO), Contents (RO), Metadata (RO)
- Subscribe events: Issues, Pull request, Push
- Install App to **All repositories**

## Test checklist
- Create or edit an issue
- Create/update/merge a PR
- Push commits to any repository
- Verify Telegram message delivery

## Security notes
- Rotate Telegram bot token if exposed
- Keep webhook secret long and random
- Do not commit real tokens/secrets

## License
MIT
