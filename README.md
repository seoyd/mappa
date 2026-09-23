# Mappa v0.2.1 (실기기 검증 진행 중)

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

프로토콜은 [PROTOCOL.md](docs/PROTOCOL.md), 현재 검증 상태는 [GRAPH.md](docs/GRAPH.md)에 기록합니다. v0.2.1에서는 로컬 Axum/PostGIS를 무료 Quick Tunnel의 HTTPS 주소로 노출해 Mac의 fake 위치에서 생성·재조회했습니다. 외장 Xcode 27.0으로 Simulator와 기기용 Rust 실행 파일 및 `.app` 번들을 빌드했고 Simulator UI 실행도 확인했습니다. iPhone 13이 유선으로 연결되고 Developer Mode가 활성화됐으며 Apple Development 인증서도 생성됐습니다. 하지만 Personal Team의 iPhone 등록 한도 초과로 개발 프로파일을 만들 수 없어 설치 가능한 빌드는 실패했습니다. 서명 없는 Simulator의 Keychain 접근은 `-34018`로 실패했습니다. 따라서 실제 GPS 게시와 identity 재시작 gate는 미검증입니다.

## v0.2 로컬 검증

PostgreSQL/PostGIS 서버를 시작하고 `DATABASE_URL`을 설정한 뒤 실행합니다. v0.2 harness는 고유한 테스트 글을 실제 DB에 저장하며, fake 위치 입력만 사용합니다.

```sh
DATABASE_URL='postgres://...' cargo run -p mappa-harness --bin v02
```

기존 v0.1 harness는 `cargo run -p mappa-harness`로 유지됩니다. 두 harness 모두 0002 migration이 적용된 DB에서 실행할 수 있습니다.

## iOS 준비와 실행

전체 Xcode와 iOS SDK, Xcodegen, Rust의 `aarch64-apple-ios` 및 `aarch64-apple-ios-sim` 타깃이 필요합니다. 앱의 Rust UI는 `mobile/mappa-ios/src/main.rs`, Xcodegen 설정은 `mobile/mappa-ios/project.yml`에 있습니다. 앱 시작 시 Keychain 익명 ID를 준비하고, 서버 연결 뒤와 사용자가 갱신/게시를 누를 때 one-shot 위치를 요청합니다. UI는 When In Use 위치 권한만 요청합니다. 정확도 100m 초과 또는 30초보다 오래된 위치에서는 게시하지 않습니다.

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo check -p mappa-apple --target aarch64-apple-ios
IPHONEOS_DEPLOYMENT_TARGET=16.0 cargo build -p mappa-ios --bin mappa-ios --target aarch64-apple-ios
cd mobile/mappa-ios && xcodegen generate
```

생성된 Xcode 프로젝트를 열어 Apple 개발 팀/서명을 선택한 뒤 iPhone에 배포합니다. 서명 없는 Simulator 빌드는 UI를 실행할 수 있지만 Keychain의 `errSecMissingEntitlement`로 게시 준비가 완료되지 않습니다. 앱 화면의 개발 서버 주소에 현재 Quick Tunnel의 HTTPS URL을 입력하고 연결합니다. 재빌드 없이 주소를 바꿀 수 있으며, `MAPPA_API_BASE_URL` 빌드 변수는 입력란의 선택적 초기값일 뿐입니다. 앱은 HTTPS Mappa 서버만 허용합니다. `http://localhost`/`127.0.0.1`은 로컬 harness 전용으로 허용하지만 물리 iPhone의 localhost는 Mac 개발 서버가 아닙니다. 앱은 전역 ATS 예외를 설정하지 않습니다. 일회성 개발 배포의 실제 명령과 주의사항은 [DEPLOYMENT_DEV.md](docs/DEPLOYMENT_DEV.md)에 있습니다.

실기기 검증 절차: 기기의 Developer Mode와 Xcode 개발 팀 설정 → 권한 승인 후 화면의 Location/Accuracy/Cell 확인 → 실제 문장 입력 및 게시 → 서버 DB count/revision 확인 → 앱의 같은 cell 조회 결과 확인 → 앱 종료/재실행 후 Keychain ActorId 유지 확인. 아직 기기 실행 이후의 절차는 수행되지 않았습니다. ActorId는 익명 로컬 식별자이며 인증 계정이나 위치 위조 방지 수단이 아닙니다.
