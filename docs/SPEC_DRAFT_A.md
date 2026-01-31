# SPEC DRAFT A — “엔진( habit-cli ) + 오케스트레이터(OpenClaw/Telegram)” (v1)

## 0. 요약
- **habit-cli는 ‘결정론적(Deterministic) 습관 엔진’**으로만 동작한다. (로컬 DB, 규칙/벌칙/예외/상태 계산)
- **자연어 이해/대화(LLM)는 OpenClaw가 담당**한다. (Telegram에서 받은 메시지를 해석 → habit-cli에 “구조화된 명령”으로 반영)
- 신규 습관(New habit) 첫 체크인은 **반드시 “명시적 선언문(Declaration)”**을 요구하고, 사용자 응답이 올 때까지 OpenClaw가 계속 리마인드한다.

이 초안은 “제품을 빨리 만들고, 추후 iPhone sync/고급 AI는 외부로 확장”을 목표로 한다.

---

## MVP 기능(v1)
- 습관 등록/조회/체크인(로컬 DB)
- 신규 습관 첫 체크인: 선언문(Declaration) 강제 + 대기 상태
- 일일 due/status/penalty 계산(미이행 시 다음날 목표 가중)
- 예외 기록(사유 제한 + 월별 쿼터)
- 서브루틴 정의 및 체크인 시 서브루틴별 기록
- DB 암호화 저장 + private GitHub repo 백업(git push)
- OpenClaw/Telegram 연동을 위한 JSON 출력(`--format json`)

## 1. 목표(Goals)
1. 사용자가 Telegram에서 일상 행동을 자연어로 말하면(OpenClaw가 해석) **습관이 자동 등록**된다.
2. 매일 정해진 시간에(OpenClaw cron) “오늘 할 일/미완료”를 묻고, 사용자의 응답을 통해 **체크인 기록**한다.
3. **신규 습관의 첫 체크인**은 선언문 없이는 인정하지 않는다. (OpenClaw가 “정확한 문장 템플릿”을 제시하고, 사용자가 그대로 보내야 완료)
4. **벌칙/함정(패널티)**: 일정 미달(미체크인)이면 다음 날 보상 규칙이 자동 강화된다. 예: 2h → 4h, 4km → 8km.
5. 습관 하나 안에 **서브루틴(sub-routine)**을 정의/추적한다. (예: “운동” = 스트레칭 + 근력 + 유산소)
6. 데이터는 로컬에 저장, **암호화**, 그리고 **개인 GitHub private repo로 백업**한다.

## 2. 비목표(Non-goals)
- iPhone 동기화(추후)
- 다자 사용자/팀 공유
- 습관 추천, 코칭 등 고급 기능
- 완전한 자연어 파서(습관 엔진 내부에 LLM 포함) — v1에서는 OpenClaw 쪽에서 해결

---

## 3. 핵심 UX 개념
### 3.1 용어
- **Habit**: 추적 대상(예: “아침 코딩 2시간”)
- **Routine/SubRoutine**: Habit 내부 구성 요소(예: “워밍업 10분”, “알고리즘 30분”)
- **Checkin**: 특정 날짜/시간에 수행 완료를 기록하는 이벤트
- **Declaration**: 신규 습관 첫 체크인을 인정하기 위한 “정형 문장”
- **Exception**: 예외 인정(질병/출장 등). 반드시 명시적으로 기록되며, 사용 가능 횟수 제한.
- **Penalty**: 미이행 시 다음 날 목표가 가중되는 규칙

### 3.2 “신규 습관” 선언문 정책
- “신규 습관”은 **등록 후 첫 체크인**(최초 인정)에만 선언문을 요구한다.
- 선언문은 날짜/시간/행위가 포함된 단일 문장. 예:
  - `나는 2026년 1월 31일 10시경에 아침 코딩(2시간)을 했습니다.`
- OpenClaw는 템플릿을 제공하고, 사용자가 **그대로(또는 필수 슬롯 충족)** 답할 때까지 반복 프롬프트.

### 3.3 패널티(벌칙) 정책
- Habit마다 “기본 목표(base target)”와 “가중 규칙”을 둔다.
- 미이행(그리고 예외 없음) 시 다음 스케줄 발생일에 목표가 증가한다.
  - 기본: **2배(×2)**
  - 연속 미이행이면 **지수 증가(×2^n)** 또는 “최대 배수 cap” 적용(예: 8배 제한) — v1은 단순화를 위해 cap 도입 권장.

---

## 4. 데이터 모델(로컬 DB; JSON; 암호화 저장)
> v1 기본은 단일 DB 파일(암호화된 blob) + 메타 파일(버전/키 식별자).

### 4.1 파일 구조(권장)
- `${XDG_DATA_HOME}/habit-cli/db.age` (암호화 DB)
- `${XDG_DATA_HOME}/habit-cli/db.meta.json` (암호화 아님: schema_version, created_at, key_id 등)

### 4.2 상위 스키마(개략)
```jsonc
{
  "schema_version": 1,
  "habits": [
    {
      "id": "uuid",
      "slug": "morning-coding",
      "title": "아침 코딩",
      "schedule": {"kind": "everyday"},
      "target": {"unit": "minutes", "value": 120},
      "subroutines": [
        {"id": "uuid", "title": "워밍업", "target": {"unit": "minutes", "value": 10}},
        {"id": "uuid", "title": "본코딩", "target": {"unit": "minutes", "value": 110}}
      ],
      "onboarding": {
        "requires_declaration_for_first_checkin": true,
        "pending_declaration": {
          "date": "2026-01-31",
          "template": "나는 2026년 1월 31일 10시경에 아침 코딩(2시간)을 했습니다.",
          "status": "waiting" // waiting|satisfied|cancelled
        }
      },
      "penalty": {
        "kind": "multiplier",
        "base_multiplier": 2,
        "max_multiplier": 8,
        "apply_on": "next_due_date"
      },
      "exceptions_policy": {
        "allowed_reasons": ["sick", "travel", "emergency"],
        "limit": {"per": "month", "count": 2}
      },
      "created_at": "...",
      "archived_at": null
    }
  ],
  "events": [
    {
      "id": "uuid",
      "ts": "2026-01-31T10:05:00+09:00",
      "type": "checkin",
      "habit_id": "uuid",
      "date": "2026-01-31",
      "amount": {"unit": "minutes", "value": 120},
      "subroutine_amounts": [
        {"subroutine_id": "uuid", "amount": {"unit": "minutes", "value": 10}}
      ],
      "note": "",
      "evidence": {
        "kind": "declaration",
        "text": "나는 2026년 1월 31일 10시경에 ..."
      }
    },
    {
      "id": "uuid",
      "ts": "2026-02-01T09:00:00+09:00",
      "type": "exception",
      "habit_id": "uuid",
      "date": "2026-01-31",
      "reason": "sick",
      "text": "나는 2026년 1월 31일 감기 증상으로 아침 코딩을 예외 처리합니다."
    }
  ]
}
```

### 4.3 이벤트 소싱(event log) 채택 이유
- 패널티/예외/정정(undo) 등 “규칙이 진화”해도 과거 데이터를 보존.
- 출력(상태/통계)은 **events → 계산**으로 일관성 있게 파생.

---

## 5. CLI 명령 설계(v1)
> 원칙: habit-cli는 **대화가 아니라 상태 전이**를 제공한다. OpenClaw가 대화/자연어를 담당.

### 5.1 핵심 명령
- `habit init` : DB 암호화 초기 설정(키/패스프레이즈)
- `habit add <title> [--schedule ...] [--target ...]` : 습관 등록(기본값 포함)
- `habit add-sub <habit> <sub_title> [--target ...]` : 서브루틴 추가
- `habit due [--date YYYY-MM-DD] [--format json]` : 해당 날짜에 해야 할 습관 목록(필요 target/패널티 반영)
- `habit status [--date ...]` : 달성/미달성/예외/보류(선언문 대기) 요약
- `habit checkin <habit> [--date ...] [--amount ...] [--sub ...] [--evidence-text ...]` : 체크인 기록
- `habit declare <habit> --text "..." [--date ...]` : **신규 습관 선언문 제출**(이게 만족되면 checkin 허용 또는 즉시 checkin 생성)
- `habit exception <habit> --date ... --reason <sick|travel|...> --text "..."` : 예외 기록
- `habit penalties [--date ...]` : 미이행으로 인해 내일 가중되는 목록
- `habit backup init --repo <git_url>` : 백업 리포 연결(로컬에 remote 등록)
- `habit backup push [--message ...]` : 암호화 DB를 git commit+push

### 5.2 OpenClaw 연동을 위한 “기계 친화 출력”
- 대부분의 read-only 명령은 `--format json` 지원(안정적인 키 순서/정렬)
- `habit due --format json` 응답에 아래 포함:
  - `required_amount` (패널티 반영)
  - `onboarding.pending_declaration` 여부
  - `exception_remaining` (월별 남은 횟수)

---

## 6. OpenClaw/Telegram 통합 설계
### 6.1 책임 분리
- **OpenClaw**
  - Telegram 수신/발신
  - 자연어 → 구조화된 의도(intent)로 변환
  - 신규 습관일 때 선언문 템플릿 생성 및 “응답 올 때까지” 재촉
  - 매일 리마인드(cron)
- **habit-cli**
  - DB(암호화/복호화)
  - 습관/서브루틴/이벤트 저장
  - due/status/penalty 계산

### 6.2 OpenClaw cron 시나리오(예)
- 매일 21:00: `habit due --date today --format json` 호출 → Telegram으로 “오늘 남은 것” 전송
- 매일 23:30: 미완료 + 선언문 대기 항목 재촉
- 매일 00:05: 전날 미이행 확정 → `habit penalties --date yesterday`로 내일 가중 안내
- 매일 00:10: `habit backup push` (네트워크 가능할 때만)

### 6.3 Telegram 대화 예시
#### (1) 자연어로 습관 자동 등록
사용자: “요즘 매일 아침에 2시간 코딩해.”
OpenClaw(LLM): (새 습관 후보 생성) `habit add "아침 코딩" --schedule everyday --target 120m`
OpenClaw: “좋아. 새 습관 ‘아침 코딩(2시간)’을 등록했어. **오늘 수행을 인정받으려면 아래 문장을 그대로 보내줘.**
- 나는 2026년 1월 31일 10시경에 아침 코딩(2시간)을 했습니다.”

사용자: “나는 2026년 1월 31일 10시경에 아침 코딩(2시간)을 했습니다.”
OpenClaw: `habit declare morning-coding --date 2026-01-31 --text "..."`
OpenClaw: `habit checkin morning-coding --date 2026-01-31 --amount 120m --evidence-text "..."`
OpenClaw: “기록 완료.”

#### (2) 미이행 → 다음날 2배
OpenClaw(21:00): “오늘 ‘러닝 4km’ 아직 미완료. 했으면 ‘체크인’이라고 말해줘.”
사용자: (무응답)
OpenClaw(00:05): `habit penalties --date 2026-02-01` → 러닝 내일 8km
OpenClaw: “어제 러닝이 미이행이라 **오늘 목표가 8km로 가중**됐어. 예외면 ‘예외: 감기’처럼 명시해줘.”

#### (3) 예외(명시 + 제한)
사용자: “예외: 감기라서 어제 러닝 못했어.”
OpenClaw: `habit exception run --date 2026-02-01 --reason sick --text "나는 ... 예외 처리합니다."` (또는 템플릿 제공 후 선언 요구)

---

## 7. 보안/암호화/백업
### 7.1 암호화 접근(권장)
- **age(https://age-encryption.org)** 기반 파일 암호화
  - 장점: 단순한 UX(패스프레이즈/키 파일), 구현 난이도 낮음, “git에 올려도 안전”
  - Rust: `age` 크레이트 검토
- 키 관리(초안):
  - v1: 패스프레이즈(환경변수 또는 인터랙티브 입력)
  - 추후: OS keychain(선택)

### 7.2 GitHub 백업
- 원칙: **암호화된 db.age만 백업** (db.meta.json은 민감정보 최소화)
- `habit backup push`는:
  1) 작업 트리 확인(필요 시 repo init)
  2) db.age stage
  3) commit message 자동 생성(예: `backup: 2026-01-31`)
  4) push

---

## 8. Acceptance Criteria (v1)
1. Telegram에서 자연어로 습관을 말하면(OpenClaw) **habit add가 호출되어** DB에 등록된다.
2. 신규 습관의 첫 체크인은 **선언문 없이 기록되지 않는다.**
3. 선언문이 제출되면 해당 날짜 checkin이 생성되고, onboarding 상태가 `satisfied`로 바뀐다.
4. 미이행(예외 없음) 시 다음 스케줄의 `required_amount`가 2배로 증가한다(최대 cap 적용).
5. 예외는 reason+텍스트로만 생성되며, 월별 제한을 초과하면 거부된다.
6. 서브루틴이 있는 습관은 체크인 시 서브루틴별 수행량을 저장/조회할 수 있다.
7. DB는 디스크에 **평문 JSON으로 남지 않는다**(기본 경로 기준).
8. `habit backup push`로 private GitHub repo에 **암호화 DB**가 푸시된다.

---

## 9. 장단점
### Pros
- 습관 엔진이 단순/테스트 가능/로컬-퍼스트 유지
- LLM/대화 로직을 OpenClaw로 분리하여 변경 비용이 낮음
- 암호화+git 백업과 궁합이 좋음

### Cons
- OpenClaw 쪽 구현량이 증가(대화 상태/재촉 로직)
- “자연어 입력 → 습관 생성” 품질이 OpenClaw 프롬프트에 좌우

---

## 10. 무엇부터 구현할지(권장 순서)
1. **DB 암호화 저장(age) + 이벤트 로그(events) + 마이그레이션 틀**
2. `habit add`, `habit due`, `habit checkin`, `habit status`
3. **onboarding/pending_declaration 상태 + `habit declare`**
4. 패널티 계산(×2, cap)
5. 예외 정책(허용 reason + 월 제한)
6. 서브루틴 저장/체크인
7. `habit backup init/push` (git 연동)
8. OpenClaw cron 시나리오 연결(메시지 템플릿/재촉 루프)
