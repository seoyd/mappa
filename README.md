# Mappa v0.1

Rust 단일 서버와 PostgreSQL/PostGIS로 위치 글 생성·조회 vertical slice를 검증합니다. 실행 경로에 외부 지도·라우팅·AI·번역 API 호출은 없습니다.

## 구현 언어

Mappa가 작성하는 서버, 클라이언트, UI, 지도 처리, 향후 AI/라우팅 코드는 Rust로 구현합니다. Web 클라이언트가 필요해지면 Rust/WASM을 사용하며 JavaScript/TypeScript 애플리케이션 코드를 추가하지 않습니다. PostgreSQL/PostGIS의 스키마 정의는 SQL migration으로 유지하고 Rust 서버가 실행합니다. 지도 데이터와 외부 서비스 API는 별개이며 외부 지도 API는 사용하지 않습니다.

## 요구 환경

- Rust stable
- PostgreSQL과 PostGIS extension을 설치할 수 있는 DB 사용자
- `DATABASE_URL` (예: 로컬 개발 DB의 PostgreSQL URL)
- `BIND_ADDR` (예: `127.0.0.1:3000`)

## 실행

```sh
cargo fmt --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
DATABASE_URL='postgres://...' BIND_ADDR='127.0.0.1:3000' cargo run -p mappa-server
DATABASE_URL='postgres://...' cargo run -p mappa-harness
```

서버는 시작할 때 version-controlled 초기 migration을 실행합니다. Harness는 임의 포트로 실제 Axum 서버를 띄우고 binary HTTP → DB → 다른 클라이언트 조회와 새 DB 연결에서의 영속성을 검증합니다. 개발용 DB에서만 실행하세요. Harness가 고유한 테스트 게시글을 저장합니다.

프로토콜은 [PROTOCOL.md](docs/PROTOCOL.md), 현재 검증 상태는 [GRAPH.md](docs/GRAPH.md)에 기록합니다. 아직 실제 지도 UI와 기기 위치 권한 연결은 없습니다. 클라이언트 코어는 플랫폼이 제공한 위경도를 입력받습니다.
