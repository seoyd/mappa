# Implementation graph

Dependency: `G0 → G1 → {G2,G3} → G4 → G5 → G6 → G7 → G8`. Status는 해당 gate 결과가 확인된 뒤 변경한다.

v0.2 경로: `G8 → G9 → {G10 → G12 → G13, G11} → G14 → G15 → G16 → G17`. `PASS_CODE`는 코드/가짜 어댑터/로컬 DB 검증만 완료했다는 뜻이다. 실제 기기 실행은 별도 gate다.

| Node | Input | Output | Dependency | Invariant | Test / Gate | Status |
|---|---|---|---|---|---|---|
| G0 Workspace | empty repository | Cargo workspace | none | stable Rust, all members build | fmt/check/clippy/test | PASS |
| G1 Domain | E7 coordinates, text | typed Post | G0 | WGS84 bounds, original text | unit tests | PASS |
| G2 Spatial | Coordinate | z14 CellId, neighbors | G1 | reversible ID, x wrap, y bound | unit tests | PASS |
| G3 Protocol | typed messages | v1 binary frames | G1,G2 | strict decode, no panic | roundtrip/corrupt tests | PASS |
| G4 Storage | create/query | PostGIS rows, revision | G2,G3 | insert + revision atomic | migration and DB harness | PASS |
| G5 Axum | binary HTTP | create/query response | G4 | input bound, binary errors | process harness | PASS |
| G6 Client core | camera idle, cache | one batch request | G5 contract | moving = zero request | scheduler tests | PASS |
| G7 E2E harness | local DB/server | A create, B query | G5,G6 | same post, persistence | run harness | PASS |
| G8 Benchmark | running harness | baseline numbers | G7 | measured, not estimated | run and record | PASS |

2026-09-23: `cargo fmt --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace` 통과. 로컬 PostgreSQL 17.11/PostGIS 3.6.4에서 `cargo run -p mappa-harness` 통과. 별도 `mappa-server` 프로세스의 `/health` HTTP 200 확인. 상세 수치는 [BENCHMARK_V0_1.md](BENCHMARK_V0_1.md)에 기록.

## v0.2 실행 기록 (2026-09-23)

| Node | Input | Output | Dependency | Invariant | Test / Gate | Status |
|---|---|---|---|---|---|---|
| G9 Idempotent post | actor, client_post_id, coordinate, kind, body | canonical post와 원래 response | G4,G5 | unique actor/client_post_id; replay revision 불변; conflict 명시 | v0.2 real DB harness: first/replay/conflict/20 concurrent | PASS |
| G10 Device contract | raw degree, fix, store | E7, posting gate, actor | G1,G2 | invalid/stale/inaccurate 위치 차단; 저장 손상 명시 | `mappa-device` unit tests | PASS |
| G11 HTTP transport | binary frame, configured URL | bounded binary response | G3 | HTTPS, localhost HTTP만 예외; timeout; MIME/status/size 검사 | unit tests + real HTTP harness | PASS |
| G12 Actor identity | Keychain raw 16 bytes | device-local ActorId | G10 | 재실행 동일 ID, 손상 시 error, sync off | fake store 100회 및 독립 store; iOS adapter target check | PASS_CODE / DEVICE_NOT_AVAILABLE |
| G13 Apple location | one-shot CoreLocation fix | LocationStatus | G10,G12 | When In Use, foreground, timestamp/accuracy gate | `cargo check -p mappa-apple --target aarch64-apple-ios` 및 `aarch64-apple-ios-sim`; 실기기 미검증 | PASS_CODE / DEVICE_NOT_AVAILABLE |
| G14 Client runtime | identity/location/transport/core | idempotent post와 cache invalidate | G9,G10,G11,G13 contract | 같은 logical post에 같은 ID; posting 실패 시 retry | fake runtime tests + real HTTP/DB harness | PASS_CODE |
| G15 Rust mobile shell | Slint + runtime + Apple adapters | posting UI and query-back | G14 | UI main thread 유지; Rust app code | host workspace check; iOS app target blocked by absent iPhoneOS SDK | PARTIAL |
| G16 Device harness | fake location + real Axum/DB | first/replay/conflict/query-back evidence | G15 contract | fake와 actual device 명확히 분리 | `cargo run -p mappa-harness --bin v02`; device test unavailable | PASS_CODE / DEVICE_NOT_AVAILABLE |
| G17 v0.2 benchmark | measured harness | recorded baseline | G16 | 측정과 추정 분리 | [BENCHMARK_V0_2.md](BENCHMARK_V0_2.md) | PASS_CODE / DEVICE_NOT_AVAILABLE |

2026-09-23 당시 전체 iOS 앱 빌드와 실제 iPhone posting gate는 Mac에 전체 Xcode/iPhoneOS SDK 및 연결된 기기가 없어 미검증이었다. 당시 앱 target `cargo check`는 `aws-lc-sys`의 `xcrun --sdk iphoneos`와 Skia 바이너리 획득 단계에서 중단됐다. Apple adapter 자체의 target check는 통과했다.

## v0.2.1 실행 기록 (2026-09-23~24)

의존 경로: `G17 → G18 → G19 → G20 → G21 → G22 → G23 → G24 → G25`. 환경이 막은 노드는 성공으로 올리지 않는다. G20의 Mac fake-location 경로는 G19와 별도로 실행하여 네트워크와 DB만 확인했다.

| Node | Input | Output | Dependency | Invariant | Test | Gate | Status |
|---|---|---|---|---|---|---|---|
| G18 Apple toolchain | macOS, Rust targets, Xcodegen | iOS 빌드 환경 목록 | G17 | Command Line Tools와 full Xcode 구분 | `xcode-select`, `xcodebuild`, `xcrun`, `rustup`, `xcodegen generate` | full Xcode와 SDK 존재 | PASS |
| G19 Full iOS build | Slint 앱, Apple adapter, transport | simulator/device 앱 바이너리와 launch | G18 | host check는 iOS build가 아님 | 양쪽 Rust/Xcode `.app` build; Simulator install/launch/Keychain; device signing 시도 | build + launch + ActorStore 초기화 | PARTIAL / ACCOUNT_DEVICE_LIMIT |
| G20 HTTPS dev deployment | 로컬 Axum/PostGIS, 임시 URL, fake location | HTTPS create/replay/query와 DB 증거 | G19의 device 경로; G11/G14의 Mac 경로 | 임시 URL을 소스에 저장하지 않음; 최초 insert만 revision 증가 | `/health` 200/TLS 검증; `v021_tunnel`; 직접 DB 확인 | 외부 HTTPS 경로 및 실기기 연결 | PASS_MAC_FAKE / ACCOUNT_DEVICE_LIMIT |
| G21 Physical device location | iPhone, When In Use, CoreLocation | fresh E7 fix/거절 상태 | G19,G20 | one-shot, 30초/100m, 백그라운드 추적 없음 | 실기기 권한/거절/idle 확인 필요 | 실제 위치와 posting gate | BLOCKED_ACCOUNT_DEVICE_LIMIT |
| G22 Physical device post E2E | 실제 fix, Keychain ID, HTTPS | 게시와 동일 cell 재조회 | G21 | 동일 logical post 한 건, revision 한 번만 증가 | 실기기 게시/중복 탭/재시도/DB 대조 필요 | 실제 iPhone → DB → 앱 | BLOCKED_ACCOUNT_DEVICE_LIMIT |
| G23 Restart / identity | 설치된 앱과 Keychain | 재실행 전후 같은 ActorId | G22 | 값 자체를 로그에 노출하지 않음 | 실제 기기 종료/재실행 필요 | ActorId 일치 | BLOCKED_ACCOUNT_DEVICE_LIMIT |
| G24 Mobile resource baseline | 실기기 실행과 계측 | 설치 크기/메모리/CPU/idle/네트워크 수치 | G22,G23 | Simulator 수치를 모바일 수치로 표기하지 않음 | Mac 터널 frame 크기와 Simulator 앱 크기 기록; 기기 계측 필요 | idle GPS/API/DB 0 및 기기 baseline | PARTIAL / ACCOUNT_DEVICE_LIMIT |
| G25 v0.2 closure | G0~G24 증거 | 검증된 iOS vertical slice | G18~G24 | 미실행 항목을 PASS로 표시하지 않음 | workspace/DB 회귀 및 full iOS build/simulator launch | 모두 PASS, 기기 존재 시 G21~G23도 PASS | BLOCKED_ACCOUNT_DEVICE_LIMIT |

2026-09-23 도구 감사 당시 `xcode-select -p`는 `/Library/Developer/CommandLineTools`; full Xcode/iPhoneOS/Simulator SDK는 없었다. Rust 두 iOS target과 Xcodegen 2.46.0은 설치됐고 프로젝트 생성/Info.plist 검사는 통과했다. 이후 외장 Xcode가 설치되어 아래 결과로 갱신했다.

2026-09-24 재개: `xcode-select -p`는 `/Volumes/iStorage/Applications/Xcode.app/Contents/Developer`, Xcode 27.0 및 두 iOS SDK 27.0 확인. `cargo check`/`cargo build`가 Simulator 앱 전체에서 통과했다. 기기용 `cargo build`는 `IPHONEOS_DEPLOYMENT_TARGET=16.0`을 지정해야 통과했고, 두 Xcode `.app` 패키징도 통과했다. iPhone 18 Pro Simulator에 설치·launch하고 Slint UI와 생존 프로세스를 확인했다. Xcodegen의 Rust build script는 `basedOnDependencyAnalysis: false`로 수정하여 소스 변경 후 번들 바이너리가 갱신됨을 시간으로 확인했다. 앱 시작 Keychain 호출은 OSStatus `-34018`로 실패했다. `security find-identity -v -p codesigning`은 유효한 서명 신원 0개, `devicectl list devices`는 연결 기기 0개였다. 임의 entitlements 서명은 Simulator가 실행을 거부해 채택하지 않았다.

2026-09-24 회귀: `cargo fmt --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, v0.1 DB harness, v0.2 DB/idempotency harness가 통과했다. 새 위치 fix가 요청 시작 시각보다 늦게 도착하면 거부되던 시점 오류를 수정했고, fix 수신 뒤의 시각으로 검증하는 unit test가 통과했다. Apple adapter는 두 iOS Rust target에서 check가 통과했다.

2026-09-24 기기 연결 이후: iPhone 13(iOS 27.0)이 유선으로 페어링되었다. 처음에는 `connected (no DDI)`, `developerModeStatus=disabled`였고 이후 `connected`, `developerModeStatus=enabled`가 확인됐다. `devicectl device info ddiServices`는 `contentIsCompatible=true`, `isUsable=true`를 반환했다. Xcode에서 Personal Team을 선택해 Apple Development 인증서가 생성됐고 유효 신원 1개를 확인했다. 그러나 실기기 대상 `xcodebuild -allowProvisioningUpdates build`는 `Your development team has reached the maximum number of registered iPhone devices` 및 `No profiles for 'com.seoyd.Mappa' were found`로 실패했다. 설치·GPS·Keychain 실기기 검증은 이 계정 제한으로 대기 중이다.
