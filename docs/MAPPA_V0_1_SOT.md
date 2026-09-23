# MAPPA v0.1 — Graph-Driven Vertical Slice

## 0. 역할

너는 `Mappa` 프로젝트의 선임 Rust 시스템 엔지니어이자 구현 책임자다.

이번 작업의 목적은 설계 문서를 만드는 것이 아니라 **실제로 빌드되고 테스트되고 실행 가능한 Mappa v0.1 Vertical Slice를 완성하는 것**이다.

현재 저장소에 기존 코드가 있다면 먼저 전체 구조와 관련 코드를 충분히 읽고 기존 동작을 보존하라.

이 문서를 이번 작업의 **Source of Truth**로 사용한다.

문서와 기존 구현이 충돌할 경우:

1. 기존 데이터 호환성
2. 기존 정상 동작
3. 이 문서의 명시적 요구사항

순으로 검토하고, 의미 있는 충돌이 있으면 임의로 대규모 재작성하지 말고 최소 변경으로 해결한다.

단순한 TODO, mock, pseudocode, 인터페이스만 추가하고 IMPLEMENTED라고 보고하지 마라.

---

# 1. 프로젝트 개요

프로젝트명:

`Mappa`

Mappa는 **현실 위치를 기반으로 글·사건·모임·정보가 생성되고 지도 위에 존재하는 위치 기반 세계 커뮤니티**다.

핵심 철학:

* 세계지도 자체는 장기적으로 클라이언트에 내장한다.
* 사용자가 지도를 움직인다고 서버 요청이 계속 발생해서는 안 된다.
* 실제 지역 콘텐츠는 가까이 확대했을 때만 로드한다.
* 게시물은 실제 위치를 기반으로 생성한다.
* 외부 지도 API를 사용하지 않는다.
* 외부 Routing API를 사용하지 않는다.
* 외부 번역 API를 사용하지 않는다.
* 외부 AI API를 사용하지 않는다.
* 장기적으로 자체 Tiny AI/TR++, 번역 AI, Manager AI, Routing Engine을 추가한다.
* 서버/클라이언트 간 데이터는 가능한 작고 빠르게 전달한다.
* 핵심 시스템은 Rust 중심으로 작성한다.
* PostgreSQL + PostGIS를 canonical server database로 사용한다.
* 초기에는 단일 서버 구조를 유지한다.
* microservice를 만들지 않는다.

Mappa는 카카오톡이나 범용 SNS를 만드는 프로젝트가 아니다.

핵심은:

`현실 공간 → 지역 정보 → 사람 → 순간적 상호작용`

이다.

---

# 2. 이번 작업의 절대 범위

이번 작업은 **Mappa v0.1 Vertical Slice**만 구현한다.

완료 조건은 다음 흐름이 실제로 동작하는 것이다.

```text
Client
  ↓
현재 위치 결정
  ↓
Mappa spatial cell 계산
  ↓
현재 위치에서 게시글 생성
  ↓
Mappa binary protocol
  ↓
Axum
  ↓
PostgreSQL + PostGIS
  ↓
게시글 저장
  ↓
다른 client가 해당 cell 조회
  ↓
binary response
  ↓
게시글/pin 복원
```

이번 단계에서는 다음 기능을 구현하지 않는다.

* 실제 세계 전체 지도 빌드
* 80GB+ OSM Planet 다운로드
* 세계 PMTiles 생성
* TR++
* 번역 AI
* Manager AI
* Routing
* 개인 DM
* 임시 채팅방
* 사진 업로드
* Push notification
* OAuth
* Google/Kakao/Naver login
* Redis
* Kafka
* RabbitMQ
* Kubernetes
* GraphQL
* microservice
* CDN
* 결제
* 추천 시스템

향후 연결할 수 있는 경계만 명확하게 설계한다.

---

# 3. 개발 방식 — Graph Engineering + Harness Gate

이 프로젝트는 단순 TODO 순서가 아니라 **의존성 그래프**로 관리한다.

각 구현 단위를 `Node`라고 한다.

각 Node는 반드시 다음을 가진다.

```text
INPUT
OUTPUT
DEPENDENCY
INVARIANT
TEST
GATE
STATUS
```

Node 상태는 다음 중 하나다.

```text
NOT_STARTED
IMPLEMENTING
PASS
PARTIAL
BLOCKED
```

`BLOCKED`는 다음 경우에만 사용한다.

* 컴파일 불가
* 실행 불가
* 데이터 손상 가능
* binary protocol 호환성 파괴
* DB migration 파괴
* 핵심 correctness 실패
* 심각한 보안/입력검증 문제

사소한 warning이나 미래 최적화는 BLOCKED 사유가 아니다.

---

# 4. 구현 그래프

초기 그래프는 다음과 같다.

```text
G0 Workspace
       │
       ▼
G1 Domain Model
       │
       ├───────────┐
       ▼           ▼
G2 Spatial      G3 Binary Protocol
       │           │
       └─────┬─────┘
             ▼
         G4 Storage
             │
             ▼
          G5 Axum
             │
             ▼
      G6 Client State/Core
             │
             ▼
        G7 E2E Harness
             │
             ▼
        G8 Benchmark
```

각 Node가 독립적으로 검증 가능해야 한다.

---

# 5. Repository 구조

가능하면 Cargo Workspace 하나로 구성한다.

권장 구조:

```text
mappa/
├─ Cargo.toml
├─ README.md
├─ rust-toolchain.toml
│
├─ crates/
│  ├─ mappa-domain/
│  ├─ mappa-spatial/
│  ├─ mappa-protocol/
│  ├─ mappa-storage/
│  ├─ mappa-client-core/
│  └─ mappa-harness/
│
├─ server/
│  └─ mappa-server/
│
├─ migrations/
│
├─ docs/
│  ├─ MAPPA_V0_1_SOT.md
│  ├─ GRAPH.md
│  ├─ PROTOCOL.md
│  ├─ STORAGE.md
│  └─ DECISIONS.md
│
└─ tools/
```

현재 저장소의 기존 구조가 충분히 합리적이면 이를 억지로 재배치하지 않는다.

불필요하게 crate를 더 쪼개지 않는다.

---

# 6. G0 — Workspace

## 목적

Mappa를 하나의 Rust Workspace로 빌드 가능하게 한다.

## 요구사항

Rust stable을 기준으로 한다.

전체 workspace가 다음을 통과해야 한다.

```bash
cargo fmt --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

프로덕션 경로에서:

* 불필요한 `unwrap()`
* 불필요한 `expect()`
* panic 의존

을 피한다.

테스트 코드에서는 의미가 명확할 경우 허용 가능하다.

에러 처리는 typed error를 사용한다.

권장:

```text
thiserror
```

logging/tracing:

```text
tracing
tracing-subscriber
```

를 사용한다.

---

# 7. G1 — Domain Model

Mappa 내부 도메인 타입은 HTTP나 DB 타입에 종속되지 않게 한다.

최소 타입:

```text
PostId
ActorId
CellId
Post
Coordinate
Timestamp
PostKind
```

## Coordinate

위경도를 네트워크와 저장에서 가능한 안정적으로 표현한다.

floating point를 canonical representation으로 사용하지 않는다.

다음 fixed-point를 기본으로 한다.

```text
lat_e7: i32
lon_e7: i32
```

즉:

```text
37.1234567
→ 371234567
```

범위 validation:

```text
latitude:
-90.0000000 ~ +90.0000000

longitude:
-180.0000000 ~ +180.0000000
```

잘못된 좌표는 서버에서 반드시 거부한다.

---

# 8. Actor ID

v0.1에서 완전한 인증 시스템은 만들지 않는다.

클라이언트 최초 실행 시 random UUID를 하나 생성하고 로컬에 보관한다고 가정한다.

이를:

```text
ActorId
```

로 사용한다.

중요:

ActorId는 인증된 신원으로 취급하지 않는다.

향후 authentication system으로 교체 가능해야 한다.

---

# 9. Post

최소 Post 구조:

```text
Post {
    id
    actor_id
    coordinate
    cell_id
    kind
    body
    created_at
    expires_at
}
```

PostKind v0.1:

```text
General
```

만 있어도 된다.

향후:

```text
Event
Meetup
Alert
Trade
Question
```

등을 추가할 수 있도록 version-friendly하게 설계한다.

---

# 10. 게시글 본문 규칙

v0.1 게시글은 모바일 위치 글을 가정한다.

최대 UTF-8 payload:

```text
8192 bytes
```

초과 시 거부한다.

다음은 거부한다.

* invalid UTF-8
* NUL 포함
* 빈 문자열
* whitespace만 존재

사용자 문장의 의미나 은어를 임의 정규화하지 않는다.

원문은 그대로 보존한다.

---

# 11. G2 — Spatial System

Mappa의 중요한 원칙:

**지도 이동 자체는 server transaction/network request를 발생시키지 않는다.**

이를 위해 콘텐츠 조회 단위를 spatial cell로 만든다.

v0.1에서는 Web Mercator / slippy-map tile 좌표와 호환되는 고정 zoom cell을 사용한다.

기본:

```text
CONTENT_CELL_ZOOM = 14
```

사용:

```text
lat/lon
→ Web Mercator
→ z14 x/y
→ CellId
```

CellId는 `u64` 하나로 표현한다.

packing scheme는 명시적으로 문서화하고 영구적으로 versioning 가능하게 한다.

예:

```text
bits:
zoom
x
y
```

구체 packing은 구현자가 안전한 방식을 선택하되:

* reversible
* deterministic
* global
* collision-free

여야 한다.

Web Mercator latitude limit:

```text
±85.05112878°
```

그 이상은 명시적으로 처리한다.

---

# 12. Cell Neighbor Query

현재 cell 주변을 기본적으로:

```text
3 x 3
```

으로 조회할 수 있어야 한다.

```text
┌───┬───┬───┐
│ A │ B │ C │
├───┼───┼───┤
│ D │ X │ E │
├───┼───┼───┤
│ F │ G │ H │
└───┴───┴───┘
```

필수 함수:

```text
coordinate_to_cell()
cell_to_xy()
xy_to_cell()
neighbors_3x3()
```

경계:

* antimeridian
* x wrapping
* Web Mercator y bounds

를 테스트한다.

---

# 13. Local Detail Threshold

향후 UI는 개별 게시물을 지도 화면 폭 약:

```text
6 km 이하
```

에서만 표시하는 것을 기본 목표로 한다.

하지만 서버가 이를 강제할 필요는 없다.

Client Core에 정책 상수로 둔다.

```text
LOCAL_DETAIL_MAX_VIEWPORT_METERS = 6000
```

지도 이동 중에는 network request를 만들지 않는다.

Camera idle 이후에만 판단한다.

---

# 14. Client request 정책

다음 state machine의 골격을 구현한다.

```text
CAMERA_MOVING
    ↓
network = NONE

CAMERA_IDLE
    ↓
viewport > 6 km
    ↓
network = NONE

CAMERA_IDLE
    ↓
viewport <= 6 km
    ↓
required cells 계산
    ↓
cache 확인
    ↓
missing/stale cells 존재
    ↓
1개의 batch request
```

이 정책은 `mappa-client-core`에 UI 독립적으로 구현한다.

실제 MapLibre UI 연결은 이후 단계다.

---

# 15. G3 — Binary Protocol

프로덕션 API에서 JSON을 기본 wire format으로 사용하지 않는다.

Mappa v0.1 binary protocol을 구현한다.

모든 frame은 versioned여야 한다.

권장 Header:

```text
magic        4 bytes
version      u8
kind         u8
flags        u16
request_id   u32
payload_len  u32
```

magic:

```text
MPPA
```

모든 integer endian은 하나로 고정한다.

권장:

```text
little-endian
```

가변 길이 정수는 필요할 경우 unsigned LEB128/varint를 사용할 수 있다.

문서에 정확한 binary layout을 기록한다.

---

# 16. Protocol message 종류

최소 다음을 지원한다.

```text
CreatePostRequest
CreatePostResponse

QueryCellsRequest
QueryCellsResponse

ErrorResponse
```

향후 message 종류를 추가할 수 있도록 `kind` 공간을 남긴다.

---

# 17. CreatePostRequest

최소 payload:

```text
actor_id
lat_e7
lon_e7
kind
body_len
body
```

중요:

클라이언트가 `cell_id`를 보내더라도 서버는 신뢰하지 않는다.

서버가 좌표로부터 직접 CellId를 재계산한다.

---

# 18. CreatePostResponse

최소:

```text
post_id
cell_id
cell_revision
created_at
```

---

# 19. QueryCellsRequest

한 번에 여러 cell을 batch query한다.

최대:

```text
9~16 cells
```

정도로 제한한다.

각 cell에 대해 클라이언트가 알고 있는 revision을 같이 보낼 수 있게 한다.

예:

```text
cell_id
known_revision
```

---

# 20. Cell Revision

Mappa는 지도 이동 시 전체 콘텐츠를 계속 다시 받지 않는다.

각 cell에 revision number를 둔다.

예:

```text
cell 1001
revision 8
```

클라이언트 cache:

```text
cell 1001
revision 8
```

서버도 revision 8이면:

```text
UNCHANGED
```

만 응답하거나 해당 cell payload를 생략한다.

서버가 revision 9이면 새 snapshot/delta를 보낸다.

v0.1에서는 복잡한 delta protocol보다:

```text
unchanged
or
complete cell snapshot
```

정도로 구현해도 된다.

정확성과 단순성을 우선한다.

---

# 21. Protocol 방어

decoder는 반드시 다음을 방어한다.

* truncated frame
* malformed varint
* payload length overflow
* excessive payload
* invalid enum
* invalid UTF-8
* invalid coordinate
* excessive cell count
* duplicate malformed fields

악성 packet이 panic을 발생시켜서는 안 된다.

---

# 22. G4 — PostgreSQL + PostGIS

canonical server database:

```text
PostgreSQL
+
PostGIS
```

SQLite를 canonical server DB로 사용하지 않는다.

PostGIS extension migration을 포함한다.

---

# 23. Database schema

최소:

## posts

```text
id
actor_id
cell_id
lat_e7
lon_e7
geom
kind
body
created_at
expires_at
```

`geom`:

```text
geography(Point, 4326)
```

또는 프로젝트 목적에 더 적절한 PostGIS Point type을 사용하되 이유를 문서화한다.

---

## cell_state

```text
cell_id PRIMARY KEY
revision BIGINT
updated_at
```

게시물 생성/삭제/활성 상태 변경 시 해당 cell revision을 증가시킨다.

---

# 24. Index

최소:

```text
posts(cell_id)
posts(cell_id, created_at)
cell_state(cell_id)
```

그리고 spatial 정밀 연산을 위한:

```text
GIST(geom)
```

index를 추가한다.

일반적인 주변 콘텐츠 fetch는 가능한:

```text
cell_id
```

index 경로를 사용한다.

PostGIS 거리 연산은 정밀 query에만 사용한다.

---

# 25. Transaction correctness

게시글 생성 시:

```text
INSERT post
+
cell revision increment
```

은 하나의 DB transaction 안에서 처리한다.

중간 실패로:

```text
post 생성됨
cell revision 안 올라감
```

같은 상태가 생기면 안 된다.

---

# 26. Migration

migration은 version controlled한다.

초기 schema를 수동 DB 설정에 의존시키지 않는다.

새로운 개발 환경에서:

```text
database 생성
→ migration 실행
→ server 실행
```

만으로 동작 가능하게 한다.

---

# 27. G5 — Axum Server

서버:

```text
Rust
Tokio
Axum
```

단일 프로세스.

microservice 금지.

초기에는:

```text
Axum
  ├ health
  ├ create post
  └ query cells
```

만 구현한다.

---

# 28. Endpoint

예:

```text
GET  /health
POST /v1/posts
POST /v1/cells/query
```

binary endpoint Content-Type:

```text
application/x-mappa
```

를 사용할 수 있다.

---

# 29. Server input limits

HTTP body 전체 최대 크기를 제한한다.

예:

```text
64 KiB
```

v0.1 request는 이보다 훨씬 작아야 한다.

무제한 body를 허용하지 않는다.

---

# 30. Error protocol

HTTP status와 Mappa ErrorResponse를 같이 사용한다.

예:

```text
400 invalid request
404 unknown resource
413 too large
429 rate limited
500 internal
```

internal DB error나 stack trace를 클라이언트에 노출하지 않는다.

---

# 31. Observability

tracing을 사용해 최소 다음을 기록한다.

```text
request_id
message_kind
latency
DB latency
response bytes
error category
```

게시글 본문 전체를 로그에 출력하지 않는다.

정확한 개인 위치를 일반 로그에 남기지 않는 것을 기본으로 한다.

---

# 32. G6 — Client Core

실제 UI 프레임워크와 분리된 Rust client core를 만든다.

역할:

```text
coordinate
→ cell

viewport
→ local mode 판단

cell cache
→ missing/stale 판단

request 생성
→ protocol encode

response
→ protocol decode

local state update
```

UI 자체는 아직 최소화한다.

---

# 33. Cell Cache

메모리 기반 cache부터 시작한다.

각 cell:

```text
CellCacheEntry {
    cell_id
    revision
    posts
    last_updated
}
```

중요:

같은 cell을 계속 보고 있다고 매 camera movement마다 query하지 않는다.

---

# 34. Request scheduler

요구 동작:

```text
camera moved
→ 아무것도 안 함

camera idle
→ debounce

viewport 검사

> 6km
→ query 없음

<= 6km
→ 필요한 cells 계산

all cached
→ query 없음

일부 missing/stale
→ missing/stale만 batch request
```

debounce 초기값:

```text
약 300~500ms
```

정확한 값은 상수화한다.

---

# 35. 중복 요청 방지

동일한 cell set에 대해 request가 이미 진행 중이면 중복 request를 만들지 않는다.

필요:

```text
in_flight set
```

또는 동등한 구조.

---

# 36. G7 — End-to-End Harness

`mappa-harness`를 만든다.

목적은 사람이 앱을 눌러보지 않아도 핵심 vertical slice를 자동 검증하는 것이다.

Harness는 최소 다음 scenario를 실행한다.

---

## Scenario 1 — Spatial

```text
서울 좌표 A
→ Cell X

동일 위치
→ 항상 Cell X

인접 좌표
→ 예상 cell 또는 neighbor
```

---

## Scenario 2 — Protocol roundtrip

```text
CreatePostRequest
encode
decode
==
original
```

모든 message type 수행.

---

## Scenario 3 — Corrupt protocol

다음 입력에서 panic 금지:

```text
0 byte
1 byte
truncated header
잘못된 magic
unsupported version
payload length overflow
malformed varint
invalid UTF-8
```

---

## Scenario 4 — DB write

게시글 생성:

```text
post count +1
cell revision +1
```

둘 모두 transactionally 성공해야 한다.

---

## Scenario 5 — Cell query

cell에 3개 게시글 생성.

다른 cell에도 게시글 생성.

해당 cell query 결과에는 올바른 게시글만 존재해야 한다.

---

## Scenario 6 — Revision cache

클라이언트:

```text
cell revision = 5
```

서버:

```text
cell revision = 5
```

이면 게시글 전체 payload를 다시 보내지 않는다.

게시글 추가 후 revision = 6이면 업데이트가 전달된다.

---

## Scenario 7 — No-pan request

viewport가:

```text
50 km
```

이면 query 생성:

```text
0
```

viewport가:

```text
5 km
```

이고 cache miss면:

```text
1 batch request
```

같은 화면에서 camera 이동을 50회 발생시켜도 camera moving 상태에서는:

```text
network request = 0
```

이어야 한다.

---

## Scenario 8 — 3x3 cache

초기:

```text
9 cells missing
→ 1 request
```

동일 영역:

```text
0 request
```

옆으로 한 cell 이동:

기존 중복 cell은 재조회하지 말고 새로 필요한 cell만 요청한다.

---

# 37. 테스트 전략

각 Node마다 unit test.

Node 간 edge마다 integration test.

전체 graph에는 E2E harness.

테스트를 위해 production 구조를 왜곡하지 않는다.

필요한 경우 test fixture를 사용한다.

---

# 38. Property testing

특히 다음은 property/fuzz 성격 테스트가 유용하다.

```text
lat/lon → cell
cell pack/unpack
binary encode/decode
malformed packet decoder
```

가능하면 `proptest` 같은 Rust library를 dev dependency로 사용할 수 있다.

외부 서비스는 사용하지 않는다.

---

# 39. G8 — Benchmark

성능 측정 harness도 추가한다.

초기 목표는 승패 gate가 아니라 baseline 확보다.

최소 측정:

```text
coordinate → cell
100k iterations

protocol encode
protocol decode

100 posts response size

1000 posts response size

cell cache lookup

DB cell query
```

결과를:

```text
docs/BENCHMARK_V0_1.md
```

에 기록한다.

측정 환경도 기록한다.

---

# 40. Binary size 비교

production에서는 JSON을 쓰지 않지만 benchmark에서 비교 목적으로만 JSON baseline을 생성해도 된다.

예:

```text
100 post binary bytes
vs
100 post JSON bytes
```

이를 통해 실제 절감률을 측정한다.

단:

JSON library가 production runtime dependency가 되지 않게 한다.

---

# 41. 성능 철학

무조건 unsafe를 사용해 빠르게 만들지 않는다.

우선순위:

```text
correctness
↓
architecture simplicity
↓
measurement
↓
optimization
```

측정 없이 최적화하지 않는다.

---

# 42. 메모리

불필요한 clone을 피한다.

하지만 zero-copy 때문에 코드를 지나치게 복잡하게 만들지 않는다.

실제 profile 결과가 나온 후 개선한다.

---

# 43. 금지 사항

이번 작업에서 절대 하지 않는다.

```text
외부 지도 API
외부 routing API
외부 AI API
외부 translation API

Google Maps
Mapbox API
Kakao Maps API
Naver Maps API

Redis
Kafka
Kubernetes
microservice

대규모 generic framework 도입
```

MapLibre, OSM, Natural Earth 등 **오픈소스 코드/원시 데이터**는 향후 사용 가능하지만 외부 서비스 API 의존성으로 만들지 않는다.

---

# 44. 지도 관련 이번 단계

세계지도 전체를 지금 만들지 않는다.

전체 OSM Planet 파일을 다운로드하지 않는다.

다만 향후:

```text
OSM/Natural Earth
→ custom map builder
→ MLT/MVT
→ PMTiles
→ client local
```

로 연결할 수 있도록 architecture boundary만 문서화한다.

---

# 45. AI Architecture boundary

아직 AI를 구현하지 않는다.

향후 추가 예정:

```text
Tiny TR++
Translator
Slang Memory
Manager AI
```

이들은 core domain/storage/network를 침범해서는 안 된다.

게시글 canonical data는 AI가 없어도 정상 동작해야 한다.

---

# 46. Translation 원칙

향후 번역을 추가할 때도 서버 canonical post는 원문이다.

```text
Original post
= canonical

translation
= derived
```

번역 실패가 원문 데이터를 변경해서는 안 된다.

v0.1에서는 구현하지 않는다.

---

# 47. Manager AI 원칙

향후 Manager AI는:

```text
AI 판단
→ Policy Engine
→ Action
```

구조로 만든다.

AI 자체가 DB를 직접 삭제하거나 계정을 직접 정지하지 않는다.

v0.1에서는 구현하지 않는다.

---

# 48. Chat/Event boundary

향후:

```text
일반 지역 글
→ comments

시간/모임 게시글
→ temporary room
```

구조를 추가할 예정이다.

v0.1에서는 구현하지 않는다.

---

# 49. 데이터 삭제/버전

향후 moderation, correction, event history가 필요하므로 중요한 운영 기록을 단순 overwrite하지 않는 방향을 유지한다.

그러나 v0.1에서 복잡한 event sourcing은 구현하지 않는다.

과도한 미래 설계를 하지 않는다.

---

# 50. Web experiment boundary

Mappa는 이후 초고속 Web Client 실험에도 사용한다.

Core logic:

```text
spatial
binary protocol
cache
```

는 Rust crate로 UI와 분리하여 나중에 WASM에서도 재사용 가능하게 한다.

현재 v0.1에서는 이 재사용이 불가능해지는 의존성을 만들지 않는다.

---

# 51. Dependency 원칙

새 dependency를 추가할 때마다 확인:

```text
왜 필요한가?
표준 library로 충분하지 않은가?
binary size 영향은?
runtime dependency인가?
dev dependency인가?
유지보수되는가?
```

목적 없는 dependency 추가 금지.

---

# 52. 문서

최소 다음 문서를 최신 상태로 유지한다.

## docs/GRAPH.md

각 Node:

```text
G0 PASS
G1 PASS
G2 PASS
...
```

및 dependency를 기록.

---

## docs/PROTOCOL.md

byte-level layout 기록.

예:

```text
offset
size
field
endianness
validation
```

---

## docs/STORAGE.md

PostgreSQL/PostGIS schema와 index 설명.

---

## docs/DECISIONS.md

중요 architecture decision만 기록.

예:

```text
ADR-001
왜 z14 cell인가

ADR-002
왜 lat/lon fixed point인가

ADR-003
왜 cell query가 primary이고 PostGIS가 secondary인가
```

장황한 회의록을 만들지 않는다.

---

# 53. 구현 진행 원칙

코드를 먼저 이해한다.

그리고 가장 작은 dependency order로 구현한다.

권장 진행:

```text
G0
↓
G1
↓
G2
↓
G3
↓
G4
↓
G5
↓
G6
↓
G7
↓
G8
```

Node 하나가 완료될 때마다 관련 test를 즉시 실행한다.

마지막에만 한 번 몰아서 테스트하지 않는다.

---

# 54. 기존 코드 보호

기존 기능이 존재한다면 반드시 회귀 테스트한다.

새 설계를 이유로 정상 코드를 무조건 갈아엎지 않는다.

대규모 rewrite는 실제 blocker가 있을 때만 허용한다.

---

# 55. 완료 Gate

최종적으로 반드시 실행:

```bash
cargo fmt --check

cargo check --workspace

cargo clippy --workspace \
  --all-targets \
  --all-features \
  -- -D warnings

cargo test --workspace
```

PostgreSQL integration test도 실행한다.

가능하면 실제 Axum process를 띄우고 E2E harness를 실행한다.

---

# 56. 최종 E2E 성공 조건

반드시 실제로 다음을 검증한다.

```text
Client A
→ coordinate
→ CreatePost binary
→ HTTP
→ Axum
→ PostgreSQL/PostGIS
→ 저장

Client B
→ same spatial region
→ QueryCells binary
→ Axum
→ PostgreSQL
→ binary response
→ decode
→ 동일 post 확인
```

이것이 성공하지 않으면 v0.1을 DONE이라고 하지 않는다.

---

# 57. 이번 단계에서 품질 기준

v0.1의 성공 기준:

```text
정확하게 작동
+
재시작 후 데이터 유지
+
binary protocol 안정
+
cell cache 동작
+
map movement ≠ network request
```

UI 완성도는 성공 기준이 아니다.

AI 성능도 성공 기준이 아니다.

세계지도 완성도도 성공 기준이 아니다.

---

# 58. 진행 중 발견된 문제

문제를 발견했다고 작업을 중단하지 않는다.

가능하면 현재 범위에서 직접 수정한다.

범위 밖 문제는:

```text
DEFERRED
```

로 기록한다.

실제 진행이 불가능한 경우에만:

```text
BLOCKED
```

를 사용한다.

---

# 59. 최종 보고 형식

작업 완료 후 반드시 다음 형식으로 보고한다.

```text
RESULT:
PASS / PARTIAL / BLOCKED

GRAPH:
G0 ...
G1 ...
G2 ...
G3 ...
G4 ...
G5 ...
G6 ...
G7 ...
G8 ...

IMPLEMENTED:
- ...

TESTS:
- command
- result

E2E:
- Client A create
- DB persistence
- Client B query
- result

PROTOCOL:
- request bytes
- response bytes
- JSON baseline과 비교 가능하면 기록

BENCHMARK:
- coordinate→cell
- encode
- decode
- DB query
- response size

CHANGED FILES:
- ...

DEFERRED:
- ...

BLOCKERS:
- none 또는 실제 blocker

NEXT NODE:
- 다음에 가장 먼저 구현할 정확한 작업
```

"대체로 잘 됩니다" 같은 추상적 보고를 하지 않는다.

수치와 PASS/FAIL을 명확하게 기록한다.

---

# 60. 최종 구현 원칙

Mappa v0.1의 목적은 거대한 시스템을 만드는 것이 아니다.

다음 하나를 **아주 작고 정확하게 증명하는 것**이다.

> 현실의 한 위치에서 작성된 글이
> Mappa의 spatial cell에 저장되고
> 작은 binary protocol을 통해
> 다른 클라이언트의 같은 공간에 나타난다.

이 한 줄이 실제 코드와 실제 테스트로 증명되면 이번 작업은 성공이다.

구조를 미래 기능 때문에 복잡하게 만들지 말고,
미래 기능 때문에 현재 확장 가능성을 막지도 마라.

**작게 만들되 막다른 구조로 만들지 마라.**

이제 저장소를 처음부터 충분히 확인한 뒤 G0부터 실제 구현을 시작하라.
