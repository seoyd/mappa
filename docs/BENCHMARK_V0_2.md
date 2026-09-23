# Benchmark v0.2 baseline

측정일: 2026-09-23. macOS 27.0 arm64, Rust 1.98.1, Cargo dev profile, 로컬 PostgreSQL 17.11/PostGIS 3.6.4, loopback HTTP/DB. `DATABASE_URL='postgres://seo@127.0.0.1:55432/mappa_test' cargo run -p mappa-harness --bin v02` 단일 실행 결과다. 성능 합격선은 정하지 않았다. 이전 실행 데이터가 남아 있는 개발 테스트 DB이므로 초기 count/revision은 0이 아니다.

| 항목 | 반복 | 측정 |
|---|---:|---:|
| ActorId 첫 생성 + fake store 저장 | 1 | 2 µs |
| ActorId fake store 재로드 | 1 | 1 µs |
| degree → E7 | 1 | 14 µs |
| v2 create frame encode | 10,000 | 5,052 µs total |
| v2 create frame decode | 10,000 | 9,653 µs total |
| 최초 create HTTP 왕복 + DB insert | 1 | 12,417 µs |
| 동일 요청 replay HTTP 왕복 | 1 | 619 µs |
| ClientRuntime post + fake location + 실제 HTTP/DB | 1 | 1,339 µs |

요청 frame 100 bytes, 최초/재전송 응답 frame 각각 56 bytes. 같은 `(actor_id, client_post_id)`의 첫 요청과 재전송 후 DB revision/count는 25/25 → 26/26이다. 이어서 독립 연결 20개의 동시 재전송을 수행한 뒤 27/27이 되었으므로 동시 요청도 한 번만 증가했다. 서로 다른 본문의 replay는 HTTP 409였다. 그 뒤 fake 위치를 사용하는 ClientRuntime이 별도 글을 실제 Axum/DB에 저장하고 같은 cell 조회에서 확인했다. 표의 HTTP 지연은 localhost와 DB 상태의 영향을 받는 wall-clock 값이며 실제 모바일 네트워크나 GPS 지연을 나타내지 않는다.

실제 iPhone cold start, CoreLocation acquisition, 모바일 왕복 지연, idle 배터리/네트워크 사용량은 기기 부재로 측정하지 않았다. Apple adapter는 one-shot 요청을 구현하며 background location은 설정하지 않았다.

## v0.2.1 HTTPS 개발 경로 별도 측정

2026-09-23에 **Mac fake location** → Cloudflare Quick Tunnel HTTPS → 로컬 Axum → 일회성 PostGIS DB에서 `v021_tunnel`을 한 번 실행했다. DB 초기 count/revision은 0/0, create와 동일 binary request 재전송·동일 cell 조회 후 1/1이었다. DB의 actor ID, client post ID, 본문, cell과 조회 결과를 대조했다. 터널과 DB는 측정 후 종료·삭제했다. 이 수치는 iPhone, 실제 GPS 또는 모바일 회선 결과가 아니다.

| 측정 | 단일 실행 값 |
|---|---:|
| HTTPS `/health` | HTTP 200, TLS 검증 성공, 0.966 s |
| HTTPS create 왕복 | 28 ms |
| 동일 binary request replay 왕복 | 75 ms |
| 동일 cell query 왕복 | 28 ms |
| create 요청 frame | 109 bytes |
| replay 응답 frame | 56 bytes |
| query 요청 frame | 161 bytes |
| query 응답 frame | 290 bytes |

위 시간은 특정 순간의 외부 터널 왕복 값이다. 앞 표의 localhost Axum 지연과 직접 비교해 서버 자체 성능이나 모바일 성능으로 해석하지 않는다. 2026-09-24 Simulator Debug `.app`의 디스크 사용량은 약 76 MiB, 실행 파일은 79,567,912 bytes였다. 이는 실제 iPhone 설치 크기가 아니다. iPhone cold launch, idle/posting 메모리, idle CPU/GPS/API/DB 활동은 기기 부재로 측정하지 못했다.

2026-09-24 로컬 회귀 재실행: v0.1 DB harness PASS, v0.2 DB harness PASS. 마지막 v0.2 실행에서 첫 insert 16,130 µs, replay 751 µs, fake 위치 runtime post 1,451 µs였다. 이 실행의 count/revision은 34/34 → 36/36(서로 다른 두 logical post)이며 동일 logical post에 대한 20개 동시 replay는 추가 증가를 만들지 않았다. 앞의 HTTPS 터널 수치와는 별도 실행이다.
