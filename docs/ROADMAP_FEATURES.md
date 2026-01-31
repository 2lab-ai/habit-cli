# habit-cli Roadmap — Feature Breakdown (Dependencies + Effort)

> 목적: 사장님이 던진 아이디어들을 **피쳐 단위로 분해**하고,
> 각 피쳐의 **선행조건(dependencies)** + **구현 난이도/예상 시간**을 대략 산정해서
> **MVP / 1차 / 2차 / 3차**로 나눌 수 있게 한다.
>
> 전제:
> - habit-cli는 **deterministic & fast**.
> - 자연어/대화/갈굼/스케줄링은 OpenClaw가 담당.
> - iPhone 앱은 제외.

---

## 0) 현재 기준(진척도 스냅샷)

### Code size (scc)
- Rust: 14 files / 2,639 lines (code 2,264)
- Docs(Markdown): 17 files / 3,910 lines (code 2,959)

---

## 1) Feature List (모듈화)

### F0. Core Habit Tracking (기본)
- habits CRUD(추가/목록/조회/아카이브)
- checkin(추가/세트/삭제)
- status/stats/export
- deterministic json output, stable sorting

### F1. NEW Habit “Declaration Gate”
- NEW 습관은 선언문 없으면 ‘완료 인정’ 불가(정책)
- declare 기록(append-only) + 검증(형식/날짜/시간)
- day-close 시 미선언/미이행 판정에 포함

### F2. Exceptions (예외 정책)
- 예외 유형/정책(인정/불인정) 기록
- quota(주/월) 제한
- 예외가 penalty trigger를 막는 규칙

### F3. Penalty/Trap Engine (부채/패널티)
- penalty rule 등록(arm)
- tick(day-close)로 미이행 판정 → penalty trigger
- next-day escalation(2배, cap)
- resolve/void(불변 로그 남김)

### F4. Routine Templates + Routine Sessions (타이머 루틴)
- routine template 정의(steps with minutes + optional quote)
- routine session start/next/skip/done
- session state + timeline(remaining minutes, current step)

### F5. Nag Session State (AI 갈굼을 위한 상태)
- nag_state(quiet hours, cadence, snooze, max prompts)
- prompt-plan(should_send, severity, next_check_at)

### F6. Crypto Local Storage (암호화)
- 선언문/노트 등 민감 데이터 암호화 저장
- key management(로컬)

### F7. GitHub Private Backup (암호문만 push)
- encrypt bundle → git commit/push
- pull/decrypt
- 충돌 정책(backup + LWW)

### F8. X Anchor / Hash Chain Commitments (공개 커밋)
- declaration → deterministic anchor text 생성
- hash chain(prev_hash linking)

### F9. Deadman / Gradual Reveal (조건부 공개)
- 미이행 때마다 key shard 공개 계획
- threshold/shamir 개념

### F10. Payment / Marketplace (외부 서비스)
- $1000 deposit/escrow
- claim code를 암호화해 공개
- 제3자 참여(구매/관찰)

---

## 2) Dependency Graph (요약)

- F0 is base.
- F1 depends on F0.
- F2 depends on F0 (+ day-close 판단).
- F3 depends on F0 + (F1 optional) + F2.
- F4 depends on F0 (routine steps can reference habits optionally).
- F5 depends on F0 (+ F1/F3 triggers as inputs) but can be separate.
- F6 depends on F0/F1 (what to encrypt), and impacts F7/F8/F9.
- F7 depends on F6.
- F8 depends on F1 (+ optional F6).
- F9 depends on F8 + F6.
- F10 depends on F6/F8/F9 + external infra/legal.

---

## 3) Effort Estimate (난이도 + 시간)

> 단위: **엔지니어링 순수 구현 시간** 대략치(테스트/문서 포함).
> (OpenClaw 대화/cron 구현은 별도 트랙)

| Feature | Depends | Difficulty | Est. time |
|---|---|---:|---:|
| F0 Core tracking (Rust) | — | M | 이미 있음(유지/정합 0.5~1d) |
| F1 Declaration gate + append-only | F0 | M | 1~2d |
| F2 Exceptions + quota | F0 | M | 1~2d |
| F3 Penalty engine (arm/tick/resolve, 2x cap) | F0,F2,(F1) | H | 3~5d |
| F4 Routine sessions + timer state | F0 | H | 3~5d |
| F5 Nag state + prompt-plan (deterministic) | F0,(F1/F3) | M | 1~2d |
| F6 Local encryption (age or AES-GCM) | F0/F1 | H | 3~7d |
| F7 GitHub backup (encrypted only) | F6 | H | 3~7d |
| F8 X anchor (hash chain) | F1,(F6) | M | 1~3d |
| F9 Gradual reveal (shards, schedule) | F6,F8 | VH | 1~2w |
| F10 Payment/marketplace | F6,F8,F9 + external | VH | 별도 프로젝트(수주~수개월) |

---

## 4) Release Slices (MVP / 1차 / 2차 / 3차)

### MVP (v0.1) — “강제 습관 형성의 최소 뼈대” (약 1~2주)
- 포함:
  - F0 Core tracking
  - F1 Declaration gate (NEW 습관 강제 선언)
  - F2 Exceptions(간단 quota)
  - F3 Penalty engine(기본 2배 + cap, day-close tick)
  - F5 Nag state(should_send/next_check_at) **or** OpenClaw 단독(선택)
- 제외:
  - F4 Routine timer
  - F6~F10(crypto/X/payment)

### 1차 (v0.2) — “루틴 타이머 + 운영감” (추가 1~2주)
- 포함:
  - F4 Routine templates + sessions
  - F5 Nag state 강화(quiet hours/snooze)
- 목표:
  - ‘기상 루틴’ 같은 강제 세션이 돌아감

### 2차 (v0.3) — “프라이버시/백업 기반” (추가 1~3주)
- 포함:
  - F6 Local encryption
  - F7 GitHub private backup
- 목표:
  - 개인 데이터(선언/일기 확장)를 안전하게 저장/백업

### 3차 (v0.4+) — “공개 커밋/조건부 공개” (연구/옵트인)
- 포함(옵션):
  - F8 X anchor(hash chain)
  - F9 gradual reveal/deadman
- 주의:
  - 정책/사회적 리스크 크므로 옵션/가드레일 필수

### (별도) 서비스화
- F10 payment/marketplace는 habit-cli 범위를 넘어서는 **별도 제품**으로 분리.

---

## 5) CEO에게 필요한 결정(지금 2개만)

1) **MVP(v0.1)에 Routine(F4) 넣을까?**
- 넣으면 MVP 기간이 1~2주 → 2~4주로 늘 가능성.

2) **Declaration gate(F1)의 적용 범위**
- NEW 습관만 강제 vs 모든 습관 강제(강제력↑, 피로도↑)
