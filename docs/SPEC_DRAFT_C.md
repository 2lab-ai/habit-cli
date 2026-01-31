# SPEC DRAFT C — “규칙/부채(Debt) 기반 + 설정 파일 중심(확장성 우선)” (v1)

## 0. 요약
- v1부터 **규칙 엔진(Policy) + 부채(미이행 누적) 모델**을 채택해 “패널티/함정”을 일반화한다.
- 습관/서브루틴/예외/패널티가 **YAML/JSON 설정으로 선언적**으로 정의되어, 이후 iPhone sync/다른 UI를 붙이기 쉽다.
- 저장은 이벤트 로그 + 스냅샷(derived state)을 분리하고, 모든 파일은 암호화한 뒤 GitHub로 백업한다.

이 초안은 “초기 구현은 무겁지만, 규칙이 늘어나도 흔들리지 않는 토대”가 목표다.

---

## MVP 기능(v1)
- policy.yaml로 습관/서브루틴/예외/부채 정책을 선언적으로 정의
- 이벤트 로그(jsonl)로 체크인/예외/정정 기록
- `habit due`가 base+debt를 합쳐 “오늘 요구량(함정)”을 반환
- `habit close-day`로 미이행 확정 및 debt 누적
- 신규 습관 첫 체크인: 선언문 강제
- age 번들 암호화 + git 백업

## 1. 목표(Goals)
1. 자연어 입력(OpenClaw/LLM)은 가능하면 밖에서 처리하되, habit-cli는 **규칙/정산/부채 계산의 단일 진실(Single Source of Truth)**이 된다.
2. “미이행 → 다음날 2배”를 단순 배수가 아니라 **부채/정산 모델**로 확장 가능하게 만든다.
3. 서브루틴을 “체크리스트”가 아니라 **목표/가중치/부분 달성**까지 표현 가능하게 한다.
4. 예외는 명시적이며 제한적(쿼터).
5. 로컬 암호화 + git 백업.

## 2. 비목표(Non-goals)
- v1에서 완벽한 자연어 처리(선택 사항)
- v1에서 모바일 동기화
- 클라우드 계정/서버

---

## 3. 핵심 개념: Debt(부채) 모델
### 3.1 직관
- “어제 2시간 해야 했는데 안 함”은 **2시간의 부채**로 기록된다.
- 오늘 해야 할 것은 **기본 의무(base obligation) + 미납 부채(debt)**.
- 따라서 자연스럽게 2h → 4h가 된다.

### 3.2 기본 알고리즘(v1)
- 매일 스케줄 발생 시:
  - `base_required = habit.target`
  - `debt_due = min(habit.debt_balance, habit.debt_policy.max_collect_per_day)`
  - `required_today = base_required + debt_due`
- 미이행 확정 시(예외 없음):
  - `debt_balance += base_required` (또는 penalty policy에 따라 가중)
- 달성 시:
  - 먼저 base를 충족, 남는 수행량은 debt 상환에 사용

> 이 모델은 “한 번만 미이행해도 다음날 정확히 2배”를 보장하면서도,
> 연속 미이행 시에는 debt가 누적되어 더 강한 함정이 된다.

---

## 4. 데이터 모델
### 4.1 파일 분리(권장)
- `policy.yaml` : 습관/규칙 정의(사람이 편집 가능)
- `events.jsonl` : 이벤트 append-only 로그(체크인/예외/정정)
- `snapshot.json` : 계산된 현재 상태 캐시(부채/연속 미이행 등)
- 최종 디스크 저장은 모두 암호화 번들로 패키징:
  - `bundle.age` = `policy.yaml + events.jsonl + snapshot.json`를 tar로 묶어 age로 암호화

### 4.2 policy.yaml 예시
```yaml
schema_version: 1
habits:
  - id: morning-coding
    title: "아침 코딩"
    schedule: everyday
    target:
      unit: minutes
      value: 120
    subroutines:
      - id: warmup
        title: "워밍업"
        target: { unit: minutes, value: 10 }
        weight: 1.0
      - id: deepwork
        title: "본코딩"
        target: { unit: minutes, value: 110 }
        weight: 1.0
    onboarding:
      require_declaration_first_checkin: true
    exceptions:
      allowed_reasons: [sick, travel, emergency]
      quota:
        per: month
        count: 2
    debt_policy:
      enabled: true
      max_collect_per_day:
        unit: minutes
        value: 240   # 하루에 부채를 최대 2시간까지 상환 요구
      missed_adds_debt_multiplier: 1.0
```

### 4.3 events.jsonl 예시
- 한 줄에 한 이벤트(JSON)
- 장점: git diff/merge에 상대적으로 유리, append-only로 손상 위험 감소

---

## 5. CLI 명령 설계(v1)
### 5.1 정책/상태
- `habit init` : policy/events/snapshot 초기화 + 암호화 설정
- `habit policy validate` : policy.yaml 정합성 검사
- `habit policy edit` : (옵션) 편집기 열기

### 5.2 조회
- `habit due [--date ...] [--format json]`
  - 출력에 `required_today`(base+debt), `debt_balance`, `exception_remaining`, `needs_declaration` 포함
- `habit ledger <habit> [--from ... --to ...]` : debt 증가/상환 내역(설명 가능한 함정)

### 5.3 기록
- `habit checkin <habit> --amount ... [--sub ...] [--evidence-text ...]`
- `habit declare <habit> --text ... [--date ...]` (신규 습관)
- `habit exception <habit> --date ... --reason ... --text ...`

### 5.4 정산(하루 마감)
- `habit close-day --date YYYY-MM-DD`
  - 해당 날짜 due 중 미이행을 확정 → debt 증가 이벤트 생성
  - OpenClaw cron이 00:05에 실행

### 5.5 백업
- `habit backup push` : bundle.age 생성 → git commit/push

---

## 6. OpenClaw/Telegram 통합
### 6.1 기본 플로우
- (리마인드) OpenClaw cron → `habit due --format json` → 메시지 렌더링
- (사용자 응답) OpenClaw가 자연어를 해석해 `habit checkin/exception/declare` 호출
- (마감) `habit close-day`로 debt 반영 후 “내일 요구량(함정)” 안내

### 6.2 대화 예시(부채 모델 설명)
OpenClaw: “오늘 러닝: 기본 4km + 어제 미이행 부채 4km = **총 8km**. 완료했으면 선언/체크인해줘.”

사용자: “오늘 8km 뛰었어.”
OpenClaw: (기존 습관이면 바로 체크인)

미이행 시:
OpenClaw: “어제 미이행으로 4km 부채가 발생했어. 내일은 기본 4km + 부채 4km가 합쳐져 8km야.”

---

## 7. 신규 습관 선언문
- Draft A와 동일: 첫 인정은 선언문 필수
- C에서는 policy에 `require_declaration_first_checkin: true`로 선언적으로 설정

---

## 8. 보안/암호화
- age로 bundle 전체 암호화
- 평문 파일(policy/events/snapshot)은 작업 중 생성될 수 있으나,
  - 기본 동작은 “메모리 로드/임시 디렉토리 사용 후 즉시 삭제”
  - 옵션: `--plain` 모드(개발자/테스트 전용)

---

## 9. cron vs CLI 책임
- CLI(habit-cli):
  - due 계산, close-day 정산, debt/예외/선언 강제
- cron(OpenClaw):
  - 시간에 맞춰 `due`/`close-day`/`backup` 트리거
  - 메시지 전송 및(선택) 자연어 해석

---

## 10. Acceptance Criteria (v1)
1. `habit due`가 base+debt를 합쳐 “오늘 요구량”을 반환한다.
2. `habit close-day`가 미이행을 이벤트로 기록하고 debt_balance를 증가시킨다.
3. 체크인이 base를 초과하면 초과분이 debt 상환에 반영된다.
4. 신규 습관은 선언문 없이는 첫 체크인이 생성되지 않는다.
5. 예외 쿼터를 초과하면 예외 생성이 거부된다.
6. bundle.age는 평문 없이 생성/백업 가능하고, git push로 private repo에 저장된다.

---

## 11. 장단점
### Pros
- 패널티/함정을 “부채”로 모델링해 설명 가능, 확장 가능
- policy.yaml로 규칙 변경이 쉬움(코드 변경 최소)
- jsonl 이벤트는 백업/감사(audit)에 유리

### Cons
- 구현 복잡도 상승(close-day, debt 정산, snapshot)
- 초기 UX가 무거울 수 있음(policy 개념 이해 필요)

---

## 12. 무엇부터 구현할지(권장 순서)
1. 정책(policy.yaml) 로드/검증 + 이벤트 로그(jsonl) + 스냅샷 계산기
2. `habit due` / `habit checkin`
3. 신규 습관 선언문(`declare`) + pending 상태
4. `habit close-day` + debt 모델 반영
5. 예외 쿼터
6. 서브루틴(부분 달성/가중치)
7. age 번들 암호화 + `backup push`
