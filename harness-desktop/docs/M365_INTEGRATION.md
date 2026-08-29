# Microsoft 365 Copilot Integration

## 1. 결론

M365를 enterprise-knowledge Provider로 사용하는 방향은 기술적으로 가능하다. 다만 2026-08-29 기준 Microsoft 365 Copilot Chat API는 Microsoft Graph `/beta`이고 Microsoft 문서는 beta API의 production 사용을 지원하지 않는다고 명시한다. 따라서 초기 adapter는 feature flag가 있는 `Experimental`로 제공하고, tenant별 capability probe와 graceful disable을 필수로 한다.

Chat API는 multi-turn conversation, synchronous response, SSE streaming, enterprise/web grounding을 제공한다. 반면 text response만 지원하고 email 전송·파일 생성·회의 예약 같은 action, code interpreter/graphic tool, long-running task를 지원하지 않는다. M365 Provider를 coding/execution Agent로 선언해서는 안 된다. [Microsoft 365 Copilot Chat API 개요](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/chat/overview)

문서/knowledge chunk가 필요하면 Retrieval API를 별도 capability로 사용한다. 공식 문서상 SharePoint, OneDrive, Copilot connector에 대해 보안 trimming된 text extract를 반환하고 KQL filter를 지원한다. endpoint/API version과 data source별 GA 상태는 출시 시점에 다시 검증한다. [Microsoft 365 Copilot Retrieval API 개요](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/retrieval/overview), [Retrieval 요청/권한](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/retrieval/copilotroot-retrieval)

## 2. Capability 경계

권장 capability:

| Capability | 구현 경로 | 초기 상태 |
|---|---|---|
| `company.chat` | Copilot Chat API conversation | Experimental |
| `company.search` | Chat enterprise grounding 또는 Retrieval API | Experimental/endpoint별 검증 |
| `files.enterprise.read` | Retrieval API: SharePoint/OneDrive/connector | Tenant-gated |
| `mail.read` | Chat grounding 또는 별도 standard Graph adapter | Tenant-gated, provenance 구분 |
| `meetings.read` | Chat grounding 또는 별도 standard Graph endpoint | Tenant-gated |
| `streaming` | Chat API SSE | Experimental |
| `session.resume` | Copilot conversation ID mapping | API session 수명 검증 필요 |
| `file.write`, `shell`, `git`, `external.action` | 지원하지 않음 | Disabled |

Chat API는 기본적으로 enterprise search와 web search grounding을 모두 사용한다. web grounding을 끄는 선택은 매 chat message마다 적용해야 한다는 현재 제한이 있으므로 Project의 “enterprise only” 정책을 request마다 명시적으로 반영하고 event/audit에 기록한다. [Chat API 제한 사항](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/chat/overview)

## 3. 인증 구조

```text
User → Settings / Microsoft 365 / Connect
  → Rust Auth Broker creates state + nonce + PKCE verifier/challenge
  → system browser opens Entra authorize endpoint
  → exact registered loopback redirect (http://localhost, random listener port)
  → Auth Broker validates state and redeems code as public client
  → token cache serialized into OS secure storage
  → account/tenant/scope metadata only in SQLite
```

Desktop app는 client secret을 보관할 수 없는 public client다. system browser + Authorization Code/PKCE를 사용한다. Microsoft의 desktop app 문서는 system browser redirect에 `http://localhost`를 안내하며, public client에서 MSAL이 PKCE를 사용한다. [Desktop app registration](https://learn.microsoft.com/en-us/entra/identity-platform/scenario-desktop-app-configuration), [interactive token acquisition](https://learn.microsoft.com/en-us/entra/identity-platform/scenario-desktop-acquire-token-interactive)

요구사항:

- embedded WebView에 Microsoft password를 입력받지 않는다.
- client secret을 앱에 포함하지 않는다.
- state/nonce/PKCE S256, exact redirect, 짧은 callback timeout을 사용한다.
- access/refresh token과 MSAL cache는 macOS Keychain 또는 Windows 보호 저장소에 둔다.
- DB에는 `account_home_id`, `tenant_id`, consented scope, expiry metadata, secure-store reference만 둔다.
- token refresh는 Provider 내부에서 수행하고 UI/agent event에 token을 반환하지 않는다.
- multi-account/tenant switch는 task 실행 중 암묵적으로 바꾸지 않는다.
- logout은 local token cache 제거와 Provider session invalidation 결과를 각각 표시한다.

## 4. 권한과 Tenant 동의

현재 Chat conversation 생성 문서는 work/school delegated permission만 지원하고 personal Microsoft account와 application permission은 지원하지 않는다. 또한 다음 permission을 모두 요구한다고 명시한다: `Sites.Read.All`, `Mail.Read`, `People.Read.All`, `OnlineMeetingTranscript.Read.All`, `Chat.Read`, `ChannelMessage.Read.All`, `ExternalItem.Read.All`. 이는 매우 넓은 권한 집합이며 tenant admin/user consent 정책에 막힐 수 있다. [Chat conversation 생성 및 permission](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/chat/copilotroot-post-conversations)

따라서 다음 UX가 필요하다.

1. 연결 전 요청 permission과 사용 목적을 전부 표시한다.
2. tenant admin consent 필요 여부를 probe하고 `Admin approval required`를 별도 상태로 표시한다.
3. 거부/부분 동의 시 앱 전체가 아니라 M365 capability만 disable한다.
4. 조직이 broad Chat scope를 허용하지 않으면 Retrieval-only 또는 별도 standard Graph read adapter를 선택할 수 있게 한다.
5. application-only/daemon access로 우회하지 않는다.
6. scope 변화 시 incremental consent와 재승인을 명시한다.

Retrieval API 문서는 delegated work/school permissions를 사용한다. SharePoint/OneDrive retrieval에는 `Files.Read.All`과 `Sites.Read.All`이 필요하며 application permission은 지원하지 않는다. Copilot connector retrieval에는 `ExternalItem.Read.All`이 추가될 수 있다. API/data source별 최소 scope를 독립 capability로 관리한다. [Retrieval 요청/권한](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/retrieval/copilotroot-retrieval)

## 5. License와 배포 조건

현재 Chat API 개요는 Microsoft 365 Copilot add-on license 사용자를 대상으로 하며, 해당 license가 없는 사용자는 지원하지 않는다고 설명한다. 일반 Copilot Chat UI의 license 조건과 Copilot Chat API 조건을 동일하게 가정하면 안 된다. [Chat API licensing](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/chat/overview), [Copilot extensibility 비용/라이선스](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/cost-considerations)

Retrieval API는 add-on license 사용자에게 제공되며, 일부 tenant-level data source는 pay-as-you-go preview가 가능하지만 OneDrive 같은 user-level source는 조건이 다르다. Product UI는 license를 추론해서 확정 표시하지 않고 실제 endpoint probe 결과를 보여준다. [Retrieval API licensing](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/api/ai-services/retrieval/overview)

지원 상태 모델:

```text
NOT_CONFIGURED
LOGIN_REQUIRED
CONNECTED_CAPABILITY_UNKNOWN
ADMIN_CONSENT_REQUIRED
LICENSE_REQUIRED
PERMISSION_UNAVAILABLE
PREVIEW_DISABLED_BY_POLICY
READY_CHAT
READY_RETRIEVAL
READY_CHAT_AND_RETRIEVAL
RATE_LIMITED
DEGRADED
ERROR
```

HTTP 401/403만으로 license와 consent 원인을 임의 구분하지 않는다. Graph error code, claims challenge, response metadata를 redaction해 진단하고 사용자에게는 확인 가능한 조치만 제공한다.

## 6. API 흐름

### Chat

```text
POST /beta/copilot/conversations
  → conversation ID 저장(provider_sessions.external_session_id)

POST conversation chat endpoint (sync 또는 streamed)
  → prompt + per-turn grounding policy
  → SSE frame parser
  → TEXT_DELTA / citation-reference / USAGE-unavailable / COMPLETED
```

실제 continue/stream endpoint path와 request schema는 SDK pin 시 공식 OpenAPI/문서로 고정한다. beta response field를 domain struct에 직접 노출하지 않고 adapter DTO → normalized event로 변환한다. SSE parser는 partial UTF-8, split line, reconnect, duplicate event, terminal 누락, gateway timeout을 테스트한다.

Provider session mapping:

- Harness conversation 1개에 tenant/account별 Copilot conversation을 별도로 둔다.
- tenant/account가 바뀌면 기존 session을 resume하지 않는다.
- session ID는 opaque하게 저장하고 UI/log에 전체 노출하지 않는다.
- API가 conversation을 찾지 못하거나 만료했다고 응답하면 새 session 생성 전 사용자에게 문맥 단절을 표시한다.

### Retrieval

```text
POST /{supported-version}/copilot/retrieval
  queryString: context-rich single sentence
  dataSource: sharePoint | oneDrive-supported-path | externalItem
  optional filter/scope/resource metadata
  → security-trimmed text extracts + source metadata
```

결과는 정답이 아니라 untrusted context다. source URL/resource ID, retrieved time, data source, access label을 보존한다. 다른 Provider에 전달할 때 enterprise data transfer policy를 적용하며 full extract 대신 필요한 최소 segment를 선택한다.

## 7. Context 전달 정책

M365 결과는 기본 `ENTERPRISE` label이다.

- 사용자 질문과 tenant data는 요청한 M365 endpoint에만 전달한다.
- M365 결과를 Claude/Codex/기타 cloud Provider로 넘기는 것은 별도 data boundary 결정이다.
- Project가 `enterprise_context_transfer: deny`이면 M365는 최종 답을 별도 section으로 제공하고 coding Provider에는 전달하지 않는다.
- `allow-summary`이면 deterministic 또는 승인된 Provider로 redacted summary를 만든다.
- citation/source reference를 유지해 사용자가 근거를 검증할 수 있게 한다.
- sensitivity label/IRM content는 Microsoft가 접근을 허용했다는 사실만으로 다른 Provider egress가 허용되는 것이 아니다.

## 8. 오류/Throttle/사용량

처리 대상:

- interactive login 취소, conditional access, MFA, claims challenge
- consent denied/admin approval required
- license/preview/tenant policy unavailable
- 401 token expiry와 refresh failure
- 403 permission/tenant restriction
- 404 expired/invalid conversation
- 429 `Retry-After`
- 5xx/gateway timeout, SSE disconnect
- national cloud endpoint 차이

Chat API가 token usage를 제공하지 않으면 normalized token field는 `null`이다. request count, turn count, duration은 Harness가 사실로 측정 가능한 범위에서 기록하되 Provider token으로 표시하지 않는다. 비용 또한 license/consumption API 근거가 없으면 `null`이다.

## 9. 직접 Graph read adapter와의 구분

Outlook/Teams/meeting/file을 standard Microsoft Graph endpoint로 직접 읽는 기능은 Copilot Chat API와 다른 Provider capability다. 이를 추가할 때는:

- 각 resource에 least-privilege delegated scope를 독립 요청한다.
- pagination, delta, retention/sensitivity, HTML sanitization을 별도 구현한다.
- fetched raw content를 로컬 DB에 영구 저장할지 명시적으로 결정한다.
- “M365 Copilot이 답한 내용”과 “Harness가 Graph에서 직접 가져온 내용”의 provenance를 UI에서 구분한다.

초기 Chat adapter가 broad scope 때문에 막혔다고 direct Graph scraping으로 조용히 fallback하지 않는다.

## 10. 외부 검증 Matrix

| 축 | 최소 조합 |
|---|---|
| OS | macOS Apple Silicon, Windows 11 x64 |
| Account | 정상 work/school, personal 거부, multi-tenant guest |
| Consent | user 허용, admin 필요, tenant 차단, 부분/철회 |
| License | Copilot add-on, 미보유, Retrieval pay-as-you-go 해당/비해당 |
| Policy | Conditional Access/MFA, web grounding 금지, preview API 금지 |
| Chat | create, sync, SSE, multi-turn, timeout, expired conversation |
| Retrieval | SharePoint, OneDrive, connector, no results, unsupported file/size |
| Network | offline, proxy, TLS interception, 429, 5xx, reconnect |
| Security | token redaction, tenant switch, context transfer deny |

각 실행 증거에는 날짜, tenant 유형(식별정보 제외), license, consented scopes, Graph API version, app version, 결과/error code를 기록한다.

## 11. 출시 Gate

M365 기능을 stable로 표시하려면:

1. 사용하는 API version이 production-supported인지 Microsoft 문서로 재확인한다.
2. 법무/조직 관리자가 preview terms, permissions, data use를 승인한다.
3. 양 OS에서 system-browser PKCE와 secure-store round-trip을 검증한다.
4. 실제 tenant의 success/deny/license/rate-limit matrix를 통과한다.
5. token/enterprise content가 log, crash, 다른 Provider에 누출되지 않음을 canary test로 확인한다.
6. preview endpoint 폐기 시 앱 전체가 아닌 Provider만 graceful disable된다.

이 문서는 2026-08-29 공식 Microsoft 문서를 기준으로 작성했다. Graph beta, 권한, license, endpoint와 제한은 변경 가능하므로 릴리스마다 위 링크와 실제 tenant를 다시 검증해야 한다.
