# F4 루틴 템플릿 + 루틴 세션 (timer) — 스펙 (v0.2 제안)

> 목적: habit-cli 안에 “루틴”을 **결정론적 상태 머신**으로 넣는다.
> - 스케줄링/메시징/알림/대화는 OpenClaw/cron이 담당
> - habit-cli는 **상태 저장 + 전이 + 계산 + 안정적 출력**만 제공

---

## 1) Goals / Non-goals

### Goals
- 루틴 템플릿 정의(단계/분/선택 quote)
- 루틴 세션 시작 + 단계 진행(`next`) + 건너뛰기(`skip`) + 종료(`done`)
- table/json 출력 모두 deterministic
- 재시도(같은 `--ts`)에 안전(최소한의 idempotency)

### Non-goals
- 실제 타이머/실시간 카운트다운 실행
- OS 알림/cron 내장
- 자연어 파싱(“기상 루틴 시작해”) 등

---

## 2) 데이터 모델(저장)

### 2.1 Routine (템플릿)
- `id`: `r0001`, `r0002`, …
- `name`: 루틴 이름(사용자 입력)
- `at`: `HH:MM` (optional, “참고용”)
- `steps`: 순서가 의미를 가지는 배열(append-only에 가깝게 운용)
- `archived`, `created_date`, `archived_date`

### 2.2 RoutineStep
- `index`: 1부터 시작하는 순번(고정)
- `name`: 단계 이름
- `minutes`: 정수 ≥ 1
- `quote`: optional

### 2.3 RoutineSession (실행 인스턴스)
- `id`: `rs:<routine_id>:<YYYY-MM-DD>:<n>`
  - 같은 루틴/같은 날짜에 여러 세션을 허용(`n`은 1부터 증가)
- `routine_id`, `routine_name`, `date`, `started_ts`
- `state`: `active|done`
- `steps`: 시작 시점에 템플릿 steps를 **스냅샷**(나중에 템플릿이 바뀌어도 세션은 흔들리지 않게)
- `actions`: append-only 로그(재시도 idempotency/감사용)

### 2.4 RoutineAction (세션 이벤트)
- `kind`: `next|skip|done`
- `ts`: RFC3339(필수)
- `step_index`: `next|skip`에서만 존재(당시 “current step”)
- `reason`: `skip`에서만 optional(기록)

---

## 3) CLI 계약(제안)

### 3.1 `habit routine add`
```bash
habit routine add <name> [--at HH:MM] [--format table|json]
```

### 3.2 `habit routine list`
```bash
habit routine list [--all] [--format table|json]
```

### 3.3 `habit routine show`
```bash
habit routine show <routine> [--format table|json]
```

### 3.4 `habit routine archive|unarchive`
```bash
habit routine archive <routine>
habit routine unarchive <routine>
```

### 3.5 `habit routine step-add`
```bash
habit routine step-add <routine> --name <text> --minutes <N> [--quote <text>] [--format table|json]
```

### 3.6 `habit routine start`
```bash
habit routine start <routine> --date YYYY-MM-DD --ts RFC3339 [--format table|json]
```

정책:
- `--date`와 `--ts`는 필수(시스템 시계 암묵 사용 금지)
- 동일 루틴/동일 날짜에서 `started_ts`가 같은 세션이 이미 있으면 **그 세션을 그대로 반환**(idempotent)

### 3.7 `habit routine next|skip|done`
```bash
habit routine next <session> --ts RFC3339
habit routine skip <session> --ts RFC3339 [--reason <text>]
habit routine done <session> --ts RFC3339
```

정책:
- `--ts` 필수
- 동일 세션에서 동일 `kind+ts` action이 이미 있으면 no-op으로 처리(재시도 안전)
- `next|skip`은 “현재 pending step(가장 앞)”에만 적용
- `done`은 pending step이 0개일 때만 성공(그 외는 usage error)

### 3.8 `habit routine status`
```bash
habit routine status <session> [--format table|json]
```

JSON 출력(최소):
- session id/state/date/started_ts
- current step(있다면)
- steps(각 step의 status + action_ts + reason)

---

## 4) 선택자/에러 규칙
- `<routine>`은:
  - 정확한 id(`r0003`) 또는
  - unique name prefix(대소문자 무시)
- `<session>`은 세션 id를 권장(필요 시 prefix 허용 가능)
- exit code는 기존 규칙을 따른다:
  - 2: usage/validation
  - 3: not found
  - 4: ambiguous
  - 5: IO/DB corruption

---

## 5) Acceptance Criteria (bd habit-lq0.15용)
- 템플릿 생성/조회/단계 추가가 동작한다.
- 세션 start → next/skip → done 시나리오가 e2e로 검증된다.
- `--format json` 출력이 stable ordering을 만족한다.
- 같은 `--ts`로 재시도 시 중복 action이 생기지 않는다.

