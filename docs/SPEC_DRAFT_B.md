# SPEC DRAFT B — “habit-cli가 Bot Brain(대화 상태 + NL ingest)까지” (v1)

## 0. 요약
- OpenClaw/Telegram은 **메시지 운반(transport)** 역할을 최소화한다.
- habit-cli가 **(1) 자연어 메시지 ingest → (2) 습관 자동 등록/업데이트 → (3) 사용자에게 보낼 응답 생성**까지 담당한다.
- v1에서 LLM 호출은 필수가 아니며, **규칙 기반 한국어 파서(시간/거리/횟수 추출)**로 시작하고, 추후 “옵션 LLM”로 확장한다.

이 초안은 “한 바이너리로 끝나는 제품(자동화 포함)”을 목표로 한다.

---

## MVP 기능(v1)
- `habit ingest`(자연어 1문장 입력) → 습관 자동 등록/체크인/예외/질문 생성
- 대화 상태 저장(선언문 대기/추가 질문 대기)
- `habit tick`으로 일일 outbox(리마인드/재촉) 생성
- 패널티(기본 2배) + 예외 쿼터 강제
- 서브루틴 기록(최소: 어떤 서브루틴 수행했는지)
- DB 암호화 저장 + git 백업

## 1. 목표(Goals)
1. 사용자가 Telegram에서 자연어로 말하면, habit-cli가 이를 해석해 **습관 등록/체크인/예외/질문**을 자동 수행한다.
2. 매일 habit-cli가 생성한 “오늘 질문 목록”을 OpenClaw가 전송한다.
3. 신규 습관 첫 체크인은 선언문 요구(반드시).
4. 패널티/예외/서브루틴 추적.
5. 로컬 암호화 저장 + private GitHub 백업.

## 2. 비목표(Non-goals)
- 완벽한 한국어 의미 이해(모호함은 “질문”으로 해결)
- iPhone sync
- 음성/이미지 증빙

---

## 3. 아키텍처(책임 분리)
### 3.1 OpenClaw 역할
- Telegram에서 메시지 수신
- 수신 메시지를 `habit ingest ...`에 전달
- habit-cli가 반환한 `reply_text`를 Telegram으로 전송
- cron에서 `habit tick` 호출 후 반환된 `outbox` 전송

### 3.2 habit-cli 역할
- 대화 상태(conversation state) 저장
- 자연어 파싱(규칙 기반) → intent 결정
- 필요한 경우: 추가 질문 생성(예: 목표 단위/스케줄 모호)
- 선언문 템플릿 생성 및 “대기 상태” 관리

---

## 4. 데이터 모델
Draft A와 동일한 `habits + events`를 기반으로 하되, B에서는 **대화 상태를 DB에 포함**한다.

### 4.1 추가: conversation_state
```jsonc
{
  "conversation": {
    "channels": [
      {
        "source": "telegram",
        "chat_id": "123456",
        "state": {
          "awaiting": "declaration", // none|declaration|clarification
          "habit_id": "uuid",
          "date": "2026-01-31",
          "expected_template": "나는 ... 했습니다.",
          "retry_count": 2,
          "expires_at": "2026-02-02T00:00:00+09:00"
        }
      }
    ]
  }
}
```

### 4.2 NL 파싱 결과(내부 구조)
- `intent.kind`: `add_habit | checkin | exception | status | help | unknown`
- `entities`: `habit_title`, `amount(value, unit)`, `date`, `time_hint`, `schedule_hint`, `reason`

---

## 5. CLI 명령 설계(v1)
### 5.1 메시지 ingest(핵심)
- `habit ingest --source telegram --chat-id <id> --text "..." [--ts ...] [--today ...] --format json`
  - 입력: 사용자의 원문
  - 출력(JSON):
    - `actions`: 실행한 DB 변경(예: habit add, checkin create)
    - `reply_text`: 사용자에게 보낼 메시지
    - `needs_user_reply`: true/false
    - `next_expected`: 선언문 템플릿/질문

예)
```jsonc
{
  "actions": [
    {"type": "habit_created", "habit_id": "..."},
    {"type": "declaration_requested", "habit_id": "..."}
  ],
  "reply_text": "새 습관 ‘아침 코딩(2시간)’을 등록했어. 오늘 수행을 인정받으려면 아래 문장을 그대로 보내줘: ...",
  "needs_user_reply": true
}
```

### 5.2 daily tick(outbox 생성)
- `habit tick --date YYYY-MM-DD --format json`
  - 반환: 오늘 due 목록 + 미응답(선언문 대기/질문 대기) 재촉 메시지
  - OpenClaw는 반환된 outbox를 순서대로 발송

### 5.3 그 외 엔진 명령
- `habit init` / `habit add` / `habit checkin` / `habit exception` / `habit status` / `habit backup push` 등은 유지
- 하지만 실제 운영은 `ingest`/`tick`만으로도 가능하도록 설계

---

## 6. 신규 습관 선언문(정책)
- `ingest`가 새 habit를 생성한 경우:
  1) habit 등록
  2) `conversation.awaiting=declaration`로 전환
  3) `expected_template` 생성(오늘 날짜/시간 힌트 반영)
- 사용자의 다음 메시지가 템플릿을 충족하지 못하면:
  - reply: “선언문이 필요해. 아래 문장을 그대로 보내줘 …”
  - retry_count 증가

> 템플릿 충족 판정(v1):
- 필수 슬롯(날짜, “했습니다” 종결, habit title/slug 매칭, 수치/단위)이 모두 포함되면 OK.
- 완전 동일 문자열 매칭은 UX를 해칠 수 있어 “슬롯 매칭” 권장.

---

## 7. 패널티/예외/서브루틴
- Draft A의 정책과 동일
- B에서는 “대화형 질문”을 추가할 수 있음:
  - 예: “오늘 운동을 했어” → “운동의 어떤 서브루틴을 했어? (1)스트레칭 (2)근력 (3)유산소”

---

## 8. 보안/암호화/백업
- 암호화: Draft A와 동일하게 age 권장
- 백업: `habit backup push`를 `habit tick`이 내부적으로 제안하거나, OpenClaw cron이 별도 실행

---

## 9. cron vs CLI 책임
- cron(OpenClaw): 시간 트리거만 담당
  - `habit tick` 호출
  - outbox 전송
- habit-cli:
  - “무슨 말을 해야 하는지”를 결정(outbox)
  - “어떻게 저장/가중/예외 처리할지” 결정

---

## 10. Telegram 대화 예시
사용자: “오늘 4km 뛰었어.”
OpenClaw → `habit ingest ...`
habit-cli reply_text: “좋아. ‘러닝(4km)’ 체크인 기록할게. 신규 습관이면 아래 선언문을 보내줘: …”

사용자: “나는 2026년 1월 31일 20시경에 러닝(4km)을 했습니다.”
→ `ingest`가 선언 충족 확인 후 checkin 생성

---

## 11. Acceptance Criteria (v1)
1. OpenClaw는 “수신 메시지 → habit ingest → reply 전송”만으로 동작한다.
2. 신규 습관 생성 시 DB에 `awaiting=declaration`이 저장되고, 선언문 충족 전까지 checkin이 생성되지 않는다.
3. `habit tick`이 due/재촉 메시지를 생성한다.
4. 패널티/예외 제한이 엔진 레벨에서 강제된다.
5. DB는 암호화 저장되며, git 백업이 가능하다.

---

## 12. 장단점
### Pros
- OpenClaw 구현량 최소(운반자)
- CLI 단독 실행(로컬 터미널)도 “봇처럼” 사용 가능
- 대화 상태가 엔진에 있어 일관성 높음

### Cons
- habit-cli가 UI/대화까지 떠안아 복잡도 상승
- 규칙 기반 NL 파서는 한계가 있어, 모호한 발화에서 질문이 잦을 수 있음
- 테스트 범위가 넓어짐(파싱/대화/상태)

---

## 13. 무엇부터 구현할지(권장 순서)
1. Draft A의 엔진 기능(암호화 DB + habits/events + due/status)
2. `habit ingest` (최소 intent: add_habit / checkin / status)
3. 선언문 대기 상태(conversation_state) + 슬롯 기반 판정
4. `habit tick` (due + 재촉)
5. 패널티/예외 제한
6. 서브루틴 질의(선택)
7. git 백업 자동화
