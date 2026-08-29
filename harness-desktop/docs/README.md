# Harness Desktop 구현 문서

이 디렉터리는 멀티에이전트 AI Harness Desktop의 제품 목표를 실제 구현으로 연결하는 기준 문서다. 문서에서 `목표`는 최종 설계, `현재`는 저장소에 존재하는 구현, `게이트`는 외부 계정·서명·하드웨어·공식 API 검증이 필요한 항목을 뜻한다.

## 문서 지도

| 문서 | 용도 |
|---|---|
| [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) | 현재 구현 범위와 즉시 확인할 수 있는 제한 사항 |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 전체 구조, 런타임 경계, 핵심 흐름, 비기능 요구사항 |
| [DECISIONS.md](DECISIONS.md) | 주요 Architecture Decision Record(ADR) |
| [PHASE_TRACEABILITY.md](PHASE_TRACEABILITY.md) | Phase 0–15 요구사항·검증·외부 게이트 추적표 |
| [PROVIDER_PROTOCOL.md](PROVIDER_PROTOCOL.md) | Provider SPI, JSON-RPC 프로토콜, manifest 규칙 |
| [SECURITY.md](SECURITY.md) | 위협 모델, 권한 정책, secret 및 공급망 통제 |
| [DATA_MODEL.md](DATA_MODEL.md) | SQLite 모델, 이벤트 저장, FTS, migration/backup 계획 |
| [M365_INTEGRATION.md](M365_INTEGRATION.md) | Entra/Graph/M365 Copilot 통합 경계와 사전 검증 항목 |
| [RELEASE_AND_UPDATES.md](RELEASE_AND_UPDATES.md) | 패키징, 서명, 업데이트, rollback 계획 |
| [DISTRIBUTED_WORKERS.md](DISTRIBUTED_WORKERS.md) | 원격 Worker 프로토콜과 신뢰 모델 |
| [TESTING_STRATEGY.md](TESTING_STRATEGY.md) | 테스트 피라미드, contract/보안/E2E/릴리스 검증 |
| [QA_EVIDENCE.md](QA_EVIDENCE.md) | 이 구현에서 실제 수행한 자동·네이티브 검증과 남은 환경 Gate |
| [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md) | 단계별 제품화 순서와 출시 판단 기준 |

## 상태 용어

- `Implemented`: 저장소 코드와 자동 테스트로 확인 가능하다.
- `Prototype`: 대표 흐름을 실행할 수 있으나 production hardening이 남았다.
- `Scaffolded`: 타입·인터페이스·UI 또는 저장 구조가 준비됐지만 실제 Provider/OS 연동은 미완료다.
- `External-gated`: Tenant, 계정, vendor CLI, 인증서, 특정 OS/GPU 등 저장소 밖 조건이 필요하다.
- `Planned`: 설계만 존재하며 구현을 완료했다고 간주하지 않는다.

## 변경 원칙

1. 구현 PR은 관련 문서와 `PHASE_TRACEABILITY.md` 상태를 함께 갱신한다.
2. 외부 서비스가 필요한 항목은 mock 통과와 실제 환경 통과를 별도 기록한다.
3. 지원하지 않는 Provider 데이터는 추정하지 않고 `null`/`unavailable`로 보존한다.
4. 보안 경계나 DB schema가 바뀌면 각각 `DECISIONS.md`, `SECURITY.md`, `DATA_MODEL.md`를 함께 검토한다.
5. 목표 아키텍처의 존재를 기능 완료의 증거로 사용하지 않는다.
