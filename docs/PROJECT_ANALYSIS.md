# habit-cli 프로젝트 분석 (2026-02-04 기준)

## 0) 한 줄 요약
`habit` 바이너리로 동작하는 **local-first 습관/루틴 트래킹 CLI**이며, 단일 JSON DB를 읽고/수정하고, **결정론적(deterministic) 출력**을 보장하도록 설계되어 있습니다.

---

## 1) 제품/설계 원칙

### 1.1 Local-first
- 네트워크/텔레메트리 없음.
- 상태는 기본적으로 **단일 JSON 파일**에 저장.

### 1.2 Deterministic (테스트/자동화 최우선)
- 논리적 “오늘”을 고정할 수 있음:
  - `--today YYYY-MM-DD` 또는 `HABITCLI_TODAY`
- JSON 출력은:
  - 키 순서가 고정(`stable_json`)
  - 리스트/배열은 코드에서 **명시적으로 stable sort**
- append-only 성격의 이벤트(선언/예외/패널티)는 **명시적 `--ts`(RFC3339)** 입력을 요구(시스템 시계 암묵 사용 금지).

---

## 2) 현재 구현된 핵심 기능(요약)

### 2.1 F0 Core tracking
- 습관: `add/list/show/edit/archive/unarchive`
- 기록: `checkin` (add/set/delete)
- 대시보드: `status` (오늘 + 이번주)
- 통계: `stats` (streak + success rate)
- 회고: `recap` (HelloHabit-style 요약)
- 자동화용: `due` (해당 날짜에 “아직 미완료” 습관)
- 내보내기: `export` (json/csv)

### 2.2 F1~F3 (v0.1 확장)
- 선언문 게이트: `declare`
  - `needs_declaration=true`일 때 **선언 없으면 완료 인정 불가**
  - `raw_quantity` vs `counted_quantity` 분리
- 예외: `excuse`
  - quota 초과 시 deterministic하게 `denied`로 저장
  - penalty tick에서 `allowed`는 트리거를 막음
- 패널티 엔진: `penalty arm|tick|status|resolve|void`
  - `tick` idempotent(중복 debt 방지), `resolve/void` append-only action 기록

### 2.3 F4~F5 (v0.2 확장)
- 루틴 템플릿 + 세션: `routine add/list/show/archive/unarchive/step-add/start/next/skip/done/status`
  - 세션은 스텝 스냅샷 + action 로그로 재시도(idempotency)를 지원
- nag-state + prompt-plan: `nag show/config set/snooze/unsnooze/sent/plan`
  - quiet hours/snooze/cadence + due/debt 신호로 should_send/severity/next_check_at 계산

---

## 3) 저장소/데이터 모델

### 3.1 DB 경로 결정 우선순위
1) `--db <path>`
2) `HABITCLI_DB_PATH`
3) `${XDG_DATA_HOME}/habit-cli/db.json`
4) `~/.local/share/habit-cli/db.json`

### 3.2 쓰기 안전성
- `src/db.rs`에서:
  - `db.json.lock` 파일 기반 **advisory lock**으로 동시 실행 충돌 방지
  - temp 파일 write 후 rename 하는 **atomic write**

### 3.3 스키마(요약)
- `Db { version=1, meta, habits, checkins, declarations, excuses, penalty_rules, penalty_debts, penalty_actions }`
- `meta.next_*` 카운터로 ID를 안정적으로 생성(테스트/재현성 목적).

---

## 4) 코드 구조(모듈 맵)

### 4.1 엔트리포인트
- `src/main.rs`: clap 기반 CLI, DB read/update, 출력(table/json) 라우팅

### 4.2 도메인 로직
- `src/model.rs`: DB/엔티티 스키마(serde)
- `src/habits.rs`: habit 생성/선택/정렬(선택자는 “id 또는 unique name prefix”)
- `src/checkins.rs`: (habit_id,date) aggregate quantity 저장/조회
- `src/completion.rs`: `counted_quantity`/`is_declared` (선언 게이트 반영)
- `src/declarations.rs`: 선언 append-only
- `src/excuses.rs`: 예외 기록 + quota 계산
- `src/penalty.rs`: rule/debt/action, tick/resolve/void
- `src/routines.rs`: 루틴 템플릿 + 세션 상태 머신(start/next/skip/done)
- `src/nag.rs`: nag config/state + prompt-plan 계산(quiet/snooze/cadence)

### 4.3 뷰/계산
- `src/status.rs`: 오늘/이번주 대시보드 계산
- `src/stats.rs`: streak + success rate
- `src/recap.rs`: range 기반 completion %
- `src/due.rs`: automation용 due 리스트
- `src/export.rs`: csv export (디렉토리 출력)

### 4.4 공통 유틸
- `src/date.rs`: YYYY-MM-DD 파싱/ISO week 계산(chrono 미사용, 순수 구현)
- `src/ts.rs`: RFC3339 검증(chrono parse)
- `src/output.rs`: 단순 테이블 렌더링 + unicode 폭 처리
- `src/stable_json.rs`: JSON key ordering 안정화

---

## 5) 테스트 전략
- `tests/cli_integration.rs`: `assert_cmd` 기반 E2E 위주의 회귀 테스트
  - `HABITCLI_TODAY`/`--today`로 “오늘” 고정
  - JSON shape + deterministic ordering 검증

---

## 6) 다음(백로그) 작업 연결
- `bd list` 기준 P0(F0~F3)는 close 되었고, P1의 F4/F5도 구현+테스트로 반영되었습니다.
- 남은 작업(P1~P4: 운영 런북/독서로그/암호화/백업/앵커 등)은 habit-cli 단독 범위를 넘어설 수 있어 레포 경계(예: OpenClaw)부터 확정하는 것이 안전합니다.
