# F5 nag-state + prompt-plan 모델 — 스펙 (v0.2 제안)

> 목적: “메시징은 OpenClaw가 담당”하되, **언제/얼마나 강하게 물어볼지**(should_send/severity/next_check_at)를 habit-cli가 **결정론적으로 계산**할 수 있게 한다.

---

## 1) Goals / Non-goals

### Goals
- quiet hours / snooze / cadence를 반영한 `prompt-plan` 계산
- plan 계산은 **DB 상태 + 입력(now_ts/date)**만으로 재현 가능
- JSON 출력 안정(키/정렬/값)

### Non-goals
- 실제 메시지 발송/스케줄링/리마인드 루프(=OpenClaw/cron)
- 고급 ML/NL 정책

---

## 2) 데이터 모델(저장)

### 2.1 NagConfig (전역)
- `quiet_start`: `HH:MM` (default: `23:00`)
- `quiet_end`: `HH:MM` (default: `08:00`)
- `cadence_minutes`: 최소 재프롬프트 간격(기본 180 권장)

### 2.2 NagState (전역 상태)
- `snoozed_until`: RFC3339 or null
- `snooze_reason`: optional
- `last_sent_ts`: RFC3339 or null (OpenClaw가 “보냈다” 기록을 남길 때 업데이트)

---

## 3) CLI 계약(제안)

### 3.1 `habit nag show`
```bash
habit nag show [--format table|json]
```
- 현재 config/state를 출력

### 3.2 `habit nag config set`
```bash
habit nag config set --quiet-start HH:MM --quiet-end HH:MM [--cadence-minutes N]
```

### 3.3 `habit nag snooze`
```bash
habit nag snooze --until RFC3339 [--reason <text>]
habit nag unsnooze
```

### 3.4 `habit nag sent`
```bash
habit nag sent --ts RFC3339
```
- OpenClaw가 실제 전송 후 “마지막 전송 시각” 기록용

### 3.5 `habit nag plan`
```bash
habit nag plan --date YYYY-MM-DD --now-ts RFC3339 [--include-archived] [--format table|json]
```

계산 입력:
- `date`: 평가 대상 날짜(습관 due/패널티 debt 기준)
- `now_ts`: 현재 시각(quiet hours/cadence/snooze 기준)

계산 출력(최소 JSON):
- `date`, `now_ts`
- `quiet_hours` (start/end)
- `snoozed_until`, `last_sent_ts`, `cadence_minutes`
- `due_count` (=`habit due` 결과의 counts.due)
- `debts_due_count` (=`penalty` outstanding debts as-of)
- `severity`: 0..3
  - 0: 아무 것도 due/debt 없음
  - 1: due 존재
  - 2: debt 존재
  - 3: due + debt 모두 존재
- `should_send`: bool
- `next_check_at`: RFC3339
  - quiet hours면 quiet_end
  - snooze면 snoozed_until
  - cadence면 last_sent_ts + cadence
  - 그 외(should_send=true)이면 now_ts(즉시)
  - 그 외(should_send=false)이면 now_ts + cadence (재평가)

---

## 4) Acceptance Criteria (bd habit-lq0.14용)
- quiet hours/snooze/cadence 조합에서 should_send/next_check_at이 기대대로 계산된다.
- due/debt 유무에 따라 severity가 deterministic하게 계산된다.
- `cargo test`에 e2e 시나리오가 포함된다.

