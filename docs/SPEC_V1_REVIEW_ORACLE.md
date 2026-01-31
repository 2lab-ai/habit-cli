# Review — `SPEC_V1_DETERMINISTIC.md` (oracle)

대상: `/docs/SPEC_V1_DETERMINISTIC.md`

요청 포인트:
- 논리적 완결성/모순
- deterministic CLI contract(= CLI 내부 NLP 금지 + 동일 입력/상태 → 동일 출력/전이)
- Rust 구현 가능성
- 보안/암호화 + X anchoring 접근

---

## 1) 핵심 발견(문제/모순)

### 1.1 “100% deterministic”과 옵션 기본값/시스템 의존의 충돌
원문에는 아래가 동시에 존재했습니다.
- “habit-cli는 100% deterministic”
- `checkin`/`pending` 등에서 `--date`가 optional
- `routine`는 타이머/세션을 언급하지만 실제 시간 기준이 모호

`--date`/`--time`을 생략할 때 시스템 clock/타임존을 읽으면 “재현 가능한 결정론”이 깨집니다.

**권장:** 상태 변경(write) 명령은 날짜/시간을 **반드시 입력으로** 받도록 스펙을 강제하거나, 최소한 오케스트레이터(OpenClaw)가 항상 `--today/--ts`를 주입한다는 계약을 명시해야 합니다.

### 1.2 JSON 안정성(stable json) 요구가 불충분
원문은 “stable json”을 요구하지만,
- JSON object key ordering은 일반적으로 보장되지 않으며
- 해시/커밋에 JSON을 쓰면 canonicalization이 필수입니다.

**권장:**
- CLI 출력은 “키 집합/타입 고정 + 배열 stable sort”를 최소 보장
- 해시/커밋에는 RFC 8785(JCS) 같은 canonicalization을 스펙으로 고정

### 1.3 암호화(Random nonce)와 결정론의 충돌
원문은 age/AES-GCM 후보를 언급했지만,
- 안전한 암호화는 보통 랜덤 nonce/ephemeral을 사용 → 같은 입력을 2번 수행하면 ciphertext가 달라질 수 있음
- 스펙의 “100% deterministic”과 정면 충돌

**권장:**
- CLI 내부에서 RNG를 쓰지 않도록 설계(결정론 유지)
- nonce/salt를 (a) 호출자가 입력으로 주입하거나, (b) master_key + stable id로 **결정론적으로 파생(HMAC)**

### 1.4 X anchoring(“블록체인처럼”)이 구체적으로 정의되지 않음
원문은 “해시 체인 커밋 앵커” 아이디어만 있고,
- 무엇을 해시하는지
- prev hash 연결 규칙
- 텍스트 포맷(280자 제한)
- 추후 검증(Commit-Reveal) 방식

이 없어서 구현/테스트가 불가능했습니다.

**권장:** 해시 입력 바이트/알고리즘/SHA256 prefix string 등을 스펙으로 고정.

### 1.5 ID 스킴의 모호함
원문은 `h0001`/`d:<habit>:<date>` 등 아이디어가 섞여 있고,
- seq 규칙/정렬 기준
- 동시성(2개의 프로세스가 동시에 add 시) 처리

가 정의되지 않았습니다.

**권장:** meta에 next counter를 두는 “증가형 ID”가 테스트/디버깅/결정론에 유리합니다(파일 락 전제).

---

## 2) Rust 구현 가능성(Feasibility)

결론: 스펙을 ‘입력 명시’ + ‘canonicalization/암호화 규칙 고정’으로 정리하면 Rust로 충분히 구현 가능.

권장 크레이트:
- CLI: `clap`
- JSON: `serde`, `serde_json`
- 날짜/시간: `chrono` (또는 `time`)
- Crypto:
  - KDF: `argon2`
  - AEAD: `chacha20poly1305` (XChaCha20-Poly1305)
  - HMAC/SHA256: `hmac`, `sha2`
  - secret zeroing: `zeroize`
- file lock: `fs2` 또는 OS 락

주의점:
- “DB 전체 암호화 컨테이너”는 저장할 때마다 ciphertext 전체가 바뀌므로 git diff는 불리하지만, private repo 백업 목적에는 충분합니다.
- 결정론적 nonce 파생은 **nonce 재사용만 없으면 안전**합니다(파생 입력에 stable id를 포함).

---

## 3) 보안/암호화 + X anchoring 권장안

### 3.1 Commit-Reveal(커밋-리빌)로 설계를 단순화
- X에는 원문 대신 `commitment_sha256`만 게시
- 나중에(패널티 트리거 시) `text + commit_salt`를 공개하면 누구나 검증 가능

장점:
- X에 올릴 때 개인정보/민감정보 노출 최소화
- 공개 검증이 가능(“그때 이미 존재” 증명)

### 3.2 체인(hash chain)으로 “시간순 커밋” 강제
- 각 declaration의 commitment를 prev chain hash와 연결
- X 텍스트는 280자 내 fixed format

---

## 4) 이번 수정에서 반영한 변경(직접 편집)

`SPEC_V1_DETERMINISTIC.md`에 아래를 추가/수정했습니다.
- 결정론 정의를 “Args/Env/DB → stdout/stderr/exit/DB 전이”로 명확화
- write 명령에 `--date`/`--ts` **필수화**(시스템 clock 의존 제거)
- passphrase 주입(`--passphrase-file`/`HABITCLI_PASSPHRASE`) 및 `habit init`(kdf salt 외부 주입) 추가
- global options/exit codes/selector 규칙을 `CLI_REFERENCE.md`와 정렬
- stable JSON 규칙(배열 정렬 + hash용 JCS 권장)을 명시
- 루틴 타이머를 CLI가 돌리지 않고, 이벤트 ts만 저장하도록 명시
- 암호화/커밋/앵커를 deterministic하게 정의:
  - Argon2id + XChaCha20-Poly1305
  - nonce/salt를 HMAC로 파생
  - commitment + chain_hash + X 텍스트 포맷 고정

---

## 5) 남은 오픈 이슈(결정 필요)

1) **“선언 없으면 checkin 자체를 막을지(A) / 인정만 막을지(B)”**
- 스펙에는 A를 권장으로 적었으나, 제품 UX/데이터 보존 관점에서 B가 필요할 수 있음.

2) DB 포맷(이벤트 소싱 vs 테이블/스냅샷)
- 결정론/감사 추적에는 이벤트 소싱이 유리.
- Rust 구현 난이도/성능은 둘 다 가능.

3) X 게시 이후 “posted url”을 어떻게 기록할지
- 스펙에서는 OpenClaw 담당으로 두었고, 필요 시 `mark-posted` 같은 비네트워크 명령을 제안.

4) “GitHub backup conflict resolution”의 deterministic 정의
- last-write-wins 같은 정책은 네트워크/원격 상태에 종속.
- v1 deterministic core 범위 밖으로 분리하거나, ‘동기화는 외부에서 수행’으로 더 강하게 쪼개는 편이 안전.
