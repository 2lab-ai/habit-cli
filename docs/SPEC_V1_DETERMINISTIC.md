# habit-cli v1 SPEC (Deterministic Core) — Draft

> 목표: **사장님(임지혁)의 습관/루틴 형성을 “강제(trap/penalty)”로 돕는 로컬‑퍼스트 시스템**.
>
> 원칙:
> - **habit-cli는 100% deterministic**(입력/상태가 같으면 출력/상태 전이가 동일). 자연어 파싱/대화 생성/설득/갈굼은 **OpenClaw(에이전트)**가 담당.
> - **빠른 반응**(대부분의 명령 p95 < 100ms 목표; 기본 플로우에서 네트워크 금지).
> - **스케줄링/푸시/알림은 cron/OpenClaw가 담당**, habit-cli는 상태/규칙/계산/로그만 제공.
> - iPhone 앱/동기화 UX는 v1 범위 밖(향후 확장).

---

## 0) 핵심 사용자 스토리

### 0.1 습관 생성(자연어)
- 유저가 텔레그램에서 “나 매일 운동한다”라고 말하면, OpenClaw가 이를 **구조화된 명령**으로 변환하여 `habit add ...`로 등록한다.

### 0.2 매일 체크(갈굼) + 선언문 강제
- OpenClaw가 매일 “오늘 운동 했어?”를 묻는다.
- 특히 **새로 시작하는 습관**은 “완료 인정”을 받기 위해 유저가 **선언문 형태**로 말해야 한다.
  - 예: `"나는 2026년 1월 31일 10시경에 운동을 했습니다."`
- 선언문을 안 하면 OpenClaw가 **유저가 응답할 때까지** 계속 유도한다.

### 0.3 트랩/패널티(자기 강제)
- 유저는 “못 하면 다음날 더 한다” 같은 트랩을 스스로 걸고, 시스템은 이를 **추적/강제**한다.
  - 예: 운동 2h 미이행 → 다음날 4h
  - 예: 달리기 4km 미이행(예외 아님) → 다음날 8km
- 예외 정책(인정되는 예외/불인정 예외)을 기록하고 강제한다.

### 0.4 루틴(습관 묶음) + 타이머 세션
- 루틴은 여러 습관/행동을 **시퀀스(단계+시간)**로 묶은 것.
- OpenClaw가 “X루틴 시작하겠습니까?”를 묻고, 유저가 “시작”이라 말하면 루틴 세션이 시작된다.
- 세션은 단계별 타이머를 진행하며, 완료/다음/스킵을 기록한다.

### 0.5 외부 공개 트랩(선언 커밋/조건부 공개)
- 선언문은 로컬에 암호화 저장.
- 동시에 “블록체인처럼” **해시 체인 커밋 앵커**를 만들어 X에 게시(또는 게시 의무를 생성).
- 미이행 시 **키 공개 / 원문 공개** 같은 더 강한 패널티로 이어질 수 있다.

---

## 1) 아키텍처(역할 분리)

### 1.1 habit-cli (Deterministic Core)
- 로컬 DB(암호화 가능) 저장/로드
- 습관/루틴/세션/선언/패널티/예외의 상태 전이
- 대시보드/통계/미이행(부채) 계산
- (옵션) X 앵커 텍스트 생성(네트워크 호출 없이)

### 1.2 OpenClaw(에이전트/텔레그램)
- 자연어 이해/파싱(유저가 말로 한 것 → deterministic CLI 명령으로 변환)
- 갈굼/유도/선언문 템플릿 강제
- cron 기반 스케줄(언제 물어볼지)
- X/Threads 실제 게시(외부 액션) — 원칙적으로 유저 승인/옵트인 정책 포함

---

## 2) 성능/반응성 요구(SLO)

- 기본 명령(pending/status/checkin/declare 등) **p95 < 100ms** 목표.
- 기본 플로우에서 네트워크 금지.
- 암호화/깃 백업/외부 게시 같은 무거운 작업은 **별도 명령**으로 분리.

---

## 3) 데이터 모델(개념)

### 3.1 엔티티
- `Habit` : 습관 정의(스케줄/타겟/서브루틴 가능)
- `Routine` : 여러 단계(step)를 묶은 루틴 템플릿(시작 질문은 OpenClaw)
- `RoutineSession` : 루틴 실행 인스턴스(단계 진행 상태)
- `Checkin` : 특정 날짜의 수행 기록(정량)
- `Declaration` : 선언문(불변/append-only) + 커밋 체인 연결
- `Penalty` : 트랩/패널티 규칙 + 실행 상태
- `Exception` : 예외(정책, quota 포함)
- `NagSession` : “새 습관 선언 강제/미이행 갈굼”을 위한 세션 상태(대화 로직은 OpenClaw)

### 3.2 결정론적 ID
- Habit: `h0001`, `h0002`...
- Routine: `r0001`...
- RoutineSession: `rs:<r_id>:<YYYY-MM-DD>:<n>` (결정론적)
- Declaration: `d:<habit_id>:<YYYY-MM-DD>` + seq
- Penalty: `p:<habit_id>:<date/week>:<n>`

---

## 4) 핵심 기능(Deterministic CLI 계약)

> 아래 명령은 “자연어 없이” 항상 동일하게 동작해야 한다.

### 4.1 습관
- `habit add ...` / `habit list` / `habit show <habit>` / `habit archive|unarchive <habit>`

### 4.2 체크인
- `habit checkin <habit> [--date] [--qty|--set|--delete]`

### 4.3 선언(강제 선언문 기록)
- `habit declare <habit> --date YYYY-MM-DD --time "10:00" --text "나는 ... 했습니다."`
  - `--text`는 원문(로컬 저장)이며, 저장 정책에 따라 암호화됨.
  - 선언은 append-only. 수정은 새 선언으로만 가능(정정은 `declare --amend-of <decl_id>` 형태로 연결).

### 4.4 미이행/부채(패널티)
- `habit pending --date YYYY-MM-DD --format json`
  - 오늘/이번주 미이행 obligations을 산출.
- `habit penalty arm ...` (트랩 규칙 등록)
- `habit penalty tick --date ...` (데드라인 지나면 트리거/부채 생성)
- `habit penalty status` (미이행 패널티 목록)

### 4.5 루틴(습관 묶음 + 타이머)
- `habit routine add "기상" --at 09:00` (스케줄은 참고용; 실제 질문은 cron)
- `habit routine step-add <routine> --name "물마시기" --minutes 5 [--quote "..."]`
- `habit routine start <routine> --date YYYY-MM-DD --time HH:MM`
- `habit routine next|skip|done <session>`
- `habit routine status <session> --format json`

### 4.6 외부 공개 트랩(커밋/해시 체인)
- `habit x-anchor build --declaration <decl_id>`
  - 출력: X에 올릴 텍스트(해시 체인 앵커). deterministic.
- (옵션) `habit x-key-release plan ...`
  - “데드라인 미이행 시 키 공개”를 위한 계획/커밋 생성(실제 게시/공개는 OpenClaw)

---

## 5) 선언문 강제 규칙(정책)

- NEW habit은 `active=false` 상태로 시작.
- 일정 기간(예: 첫 7일) 또는 “NEW 플래그” 동안:
  - checkin만으로는 인정하지 않고, **declaration이 있어야 완료로 인정**(정책으로 선택 가능).
- OpenClaw가 유저에게 요구할 선언문 템플릿:
  - `나는 YYYY년 M월 D일 HH시경에 <행동>을 했습니다.`

---

## 6) 예외 정책

- 예외는 “인정됨/거절됨”이 명확히 기록되어야 함(나중에 합리화 방지).
- 예외 quota(주간/월간) 설정 가능.
- “운동하러 못 갔다”는 예외로 인정 가능, “갔는데 하기 싫어서 안 함”은 예외로 불인정 등 규칙을 명시.

---

## 7) 암호화 + GitHub private 백업(스펙 레벨)

### 7.1 로컬 암호화
- 선언/일기/민감 노트는 **암호화 저장**.
- 구현 후보:
  - age(권장) 또는 AES‑GCM(키 관리 필요)

### 7.2 GitHub backup
- 원칙: **암호문만** private repo에 push.
- 충돌: last-write-wins + 백업 파일 보존.

---

## 8) Deadman switch(조건부 공개)

- 특정 패널티/데드라인 미이행 시:
  - (옵션) 복호화 키(또는 키 조각)를 X에 게시하여 원문 공개 가능 상태로 만들 수 있음.
- 안전 정책(기본):
  - “자동 공개”는 옵션이며 기본은 **유저 승인 1회**.

---

## 9) 테스트/검증(acceptance)

- 모든 명령은 동일 DB/동일 입력에서 동일 출력.
- `pending/status/stats`는 stable sorting + stable json.
- 루틴 세션 진행은 명령 시퀀스에 대해 재현 가능.
- penalties는 tick/arm 규칙에 따라 동일하게 트리거.

---

## 10) 범위 OUT(명시)
- iPhone 앱 구현(동기화 UX)
- 자동 자연어 파싱/모델 호출을 habit-cli 내부에 포함
- 푸시/알림을 habit-cli가 직접 수행
- 자동 외부 게시(무승인) 기본값
