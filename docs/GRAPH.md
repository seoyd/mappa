# Implementation graph

## v0.3C 자체 GeoDB 지역 실증 (2026-09-24)

`S0 → S1 → S2 → S3 → S4 → S5 → S8 → S9 → S11 → S12 → S13 → S14 → S15 → S16`; 다원천의 `S6 → S7`과 수정 이력은 입력이 확보된 뒤 진행한다. 기존 구현 graph는 아래에 유지한다.

| Node | Input → Output | Dependency | Invariant | Test / Gate | Status |
|---|---|---|---|---|---|
| S0 Source discovery | 공식 후보 → 목록 | 없음 | 상용 지도 역추출 금지 | [원천 조사](MAP_SOURCES.md) | PASS_REGION |
| S1 License audit | 후보 조건 → 승인/보류 | S0 | 미확인 권리 입력 금지 | [matrix](MAP_LICENSES.md) | PASS_ROADS_SGIS_ESA / BUILDING_PENDING |
| S2 Manifest | 승인 5개 입력 → SHA·버전 잠금 | S1 | 입력 재현 가능 | `data/sources.toml`; ESA crop·polygonized 파일 재현 | PASS_ROADS_SGIS_ESA |
| S3 Adapter | 나주 도로·SGIS 경계·ESA 수면/수목 → canonical 후보 | S2 | raw 필드가 builder로 새지 않음 | source checksum·형상 검사 | PASS_ROADS_SGIS_ESA |
| S4 Schema | 후보 → WGS84 f64 feature | S3 | Mappa ID ≠ 원본 gid/ADM_CD | GeoDB roundtrip | PASS_ROADS_SGIS_ESA |
| S5 Validation | geometry → 유효 feature | S4 | NaN·퇴화 선·비정상 polygon 거부 | unit tests + [17건 거부 목록](../artifacts/map-v0.3c/naju-roads.rejected.json) | PARTIAL: 수리 정책 없음 |
| S6 Matching | 다원천 후보 → 동일 객체 | S5 | 불확실한 병합 금지 | 두 번째 승인 원천 필요 | NOT_STARTED |
| S7 Fusion | 매칭 → 단일 feature | S6 | deterministic conflict 기록 | 두 번째 승인 원천 필요 | NOT_STARTED |
| S8 GeoDB | feature → versioned binary/index | S5 | checksum·bounds·query | 11,006건·corruption test | PASS_ROADS_SGIS_ESA |
| S9 LOD | GeoDB → 줌별 도로 선택 | S8 | 저줌에 지역 밖 지형 추정 금지 | z10–z15 생성 | PARTIAL: 일반화 없음 |
| S10 Provenance | feature → lineage | S8 | runtime 타일에서 제외 | 11,006건 추적·decode | PARTIAL: revision 체인 없음 |
| S11 Tiles | GeoDB → PMTiles | S8,S9 | raw/OSM/NE 미입력 | 149 tiles, 모든 줌 decode | PASS_ROADS_SGIS_ESA |
| S12 Renderer | PMTiles → Mac 화면 | S11 | 로컬 한 파일만 열기 | [수계·수목 z14 Metal](../artifacts/map-v0.3c/naju-water-tree-river-z14.png) | PASS_MAC_5_LAYERS |
| S13 Fidelity | 화면/원본 → 오차 | S12 | 독립 기준점으로 판정 | [품질 보고](MAP_FIDELITY_V0_3C.md); 도로선 타일 경계 z10–z15 미일치 0; 도로 100개 처리 경로 측정 | PARTIAL / CRS_UNVERIFIED / NO_INDEPENDENT_GROUND_TRUTH |
| S14 Buildings | 건물 원천 → 경계·완성도 | S13 | 실제 geometry만 허용 | 승인 파일 미확보 | NO_GO_SOURCE_COVERAGE |
| S15 Size/speed | 산출물 → 수치 | S11,S12 | Mac/모바일 구분 | [측정](BENCHMARK_MAP_V0_3C.md) | PARTIAL / NO_BUILDINGS_OR_DEVICE |
| S16 Regional gate | S0–S15 → 확장 판정 | 전부 | 모든 필수 레이어·정확도 통과 | 독립 A/B 미완료 | NO_GO_SOURCE_COVERAGE |


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

## v0.3A 독립 지도 branch (2026-09-24)

G25의 `BLOCKED_ACCOUNT_DEVICE_LIMIT`는 그대로 유지한다. 지도 의존 경로는 `M0 → M1 → {M2,M3} → M4 → M5 → M6 → M7 → M8 → M9 → M10 → M11`이다. 지도 노드의 PASS는 iPhone 설치를 뜻하지 않는다.

| Node | Input | Output | Dependency | Invariant | Test | Gate | Status |
|---|---|---|---|---|---|---|---|
| M0 Architecture | 기존 workspace와 계정 blocker | 네 crate/분리된 파이프라인 | G25와 병렬 | server/DB에 지도 의존 없음 | workspace 의존성 확인, 문서 | 소유 경계 명시 | PASS |
| M1 Camera/projection | lon/lat, 물리 픽셀 | Web Mercator, 화면/역변환, tile placement, 중심 m/px | M0 | f64, x wrap, y clamp, z0~16 소수 줌 | 투영/왕복/경계/resize/축척 unit tests | 좌표 정합 | PASS_CODE |
| M2 Local PMTiles | 불변 파일, TileKey | mmap 기반 local gzip MVT bytes | M1 | tile URL 없음, missing 명시 | 실제 fixture 열기/읽기 test | local-only | PASS |
| M3 Decoder/format | MVT bytes, MLT v1 평가 bytes | feature geometry; 크기/해독 비교 | M0,M2 | MLT v2 비활성, 잘못된 MVT 오류 | real tile + malformed inputs + 3 타일 MLT CLI | 포맷 근거 | PASS_EVAL |
| M4 Geometry | polygon/holes, line | 재사용 가능한 triangle/line mesh | M3 | tile마다 준비, frame마다 tessellation 금지 | 실제 world mesh와 PNG 확인 | 지형/호수/국경 | PASS_MAC |
| M5 GPU | prepared tile, typed style | wgpu buffers/WGSL/Metal draw | M4 | 물리 해상도 유지, layer 순서 | Apple M4 Metal adapter + PNG/창 | 실제 세계 GPU 출력 | PASS_MAC |
| M6 Scheduler | viewport tile set | 타일 우선순위 작업 스레드, 프레임당 완료 타일 1개 업로드 | M5 | 중복 요청 억제, local-only | 40프레임 이동과 빈 타일/실패 카운터 | 빠른 이동 중 화면 스레드 부하 제한 | PASS_MAC; 진행 중 작업 취소는 없음 |
| M7 Pan/zoom | mouse/scroll, resize | 연속 camera update | M6 | zoom anchor 고정, antimeridian x 반복 | camera unit + window run + zoom 로그 | 수동 이동/확대 확인 | PARTIAL: drag 수동 증거 부족 |
| M8 World demo | Natural Earth 110m | 4 스타일 + 6 시점 PNG | M7 | 실제 세계 형태 | GPU PNG 시각 검사 | 대륙/국경/수면 정상 | PASS_MAC |
| M9 Cache/memory | decoded/GPU tile | CPU 64MiB 추정, GPU 128MiB LRU | M8 | 무제한 보관 금지 | cache counters와 화면 내 타일 누락 검사 | 메모리 상한/퇴출 관찰 | PARTIAL: 장시간 이동과 현재 프로세스 peak 메모리 미측정 |
| M10 Benchmark | 실행 중 Metal demo | open/decode/prep/upload/frame/format 측정 | M9 | offscreen과 화면 FPS 구분 | `--benchmark`, `compare_formats` | pan/zoom/peak 메모리 포함 기록 | PARTIAL |
| M11 Gate | M0~M10 증거 | v0.3A 판정 | M10 | 빠진 gate를 PASS로 표시하지 않음 | workspace 회귀 + 문서 | 전체 필수 조건 | PARTIAL |

이 표는 시안 구현 현황이며 최신 성능 수치와 열린 한계는 [MAP_INTERIM_REPORT_2026-09-24.md](MAP_INTERIM_REPORT_2026-09-24.md)에 정리했다. 초기 측정은 [BENCHMARK_MAP_V0_3A.md](BENCHMARK_MAP_V0_3A.md)에 남겨 두었다.

2026-09-24 색상 시안 재작업: 동일한 로컬 지형과 카메라로 Sunny/Coral/Mint/Cobalt를 Apple M4 Metal에서 렌더링해 `artifacts/map-v0.3a/contact-sheet-v2.png`와 개별 PNG를 생성했다. 앞선 비교판 `contact-sheet.png`는 보존했다. `cargo fmt --check`, 지도 renderer/demo 대상 Clippy, `git diff --check`가 통과했다. 두 crate의 `cargo test`도 성공했지만 실행된 테스트는 0개이므로 시각 검증 근거는 실제 Metal PNG다. 최종 색상 선택은 아직 하지 않았다.

2026-09-24 참고 이미지 반영: 실제 land ring에서 해안선을 만들고 tile 절단선은 빼며 굵은 둥근 해안선/네 팔레트(Arcade/Lagoon/Candy/Sunset)를 같은 Metal 경로로 렌더링했다. 세계·동아시아 4안씩 `contact-sheet-v3.png`, `contact-sheet-v3-region.png`에 저장했다. renderer의 tile 절단선 unit test 1개, 지도 renderer/demo 대상 Clippy, 실제 Metal 벤치마크가 통과했다. 도로·공원·게임 노드는 fixture에 없어 표시하지 않았다. 최종 시안 선택과 M11 판정은 미완료다.

별도 text R&D: `glyphon 0.12.0` + `cosmic-text 0.19.0` 선택 기능으로 Metal 타깃에 Latin/한국어/일본어/중국어/키릴/아랍어 글리프를 실제 렌더링했다. 전체 지도 라벨/충돌 처리는 M11 PASS 근거가 아니다.

## 확대 수준별 오프라인 지도 시안 (2026-09-24)

| Node | 확인된 출력 | 검증 | 상태 |
|---|---|---|---|
| L0 1:10m 지역 파일 | 동아시아 z5–z7 PMTiles, 589 타일, 1,174,286 B | 고정 원본 hash와 실제 Seoul/도로 타일 해독 | PASS_MAC |
| L1 데이터 선택 | z4 세계 파일, z5–z7 지역 파일; 범위 밖 세계 파일 복귀 | viewport 판정 unit test와 Metal 네 장 출력 | PASS_MAC |
| L2 확대 레이어 | z5 상세 해안선; z6 도로·도시 점/이름 | 실제 Natural Earth road/place와 화면 라벨 출력 | PASS_MAC |
| L3 한국 OSM 중간 단계 | z8–z9 173개 타일, 도시·간선도로 | 실제 타일 해독, Metal 두 화면, 누락 0 | PASS_MAC |
| L4 서울 OSM 상세 단계 | z10–z12 821개 타일, 한글 도시·역·공공기관·도로 | 실제 타일 해독, Metal 네 화면, 누락 0 | PASS_MAC |
| L5 출처·라이선스 | 고정 OSM PBF hash, ODbL 데이터 고지, 화면 기여자 표시 | 데이터/화면 확인 | PASS_MAC |
| L6 확대 정확도·성능 | 서울 범위의 실제 OSM 데이터, z12 이후 확대 | [실측](BENCHMARK_ZOOM_LOD.md); 건물·전국 상세 없음, 실제 제스처 p95 미측정 | PARTIAL |
| L7 모바일 연동 | 없음 | Apple 계정의 기기 등록 제한 유지 | BLOCKED_ACCOUNT_DEVICE_LIMIT |

이 작업은 macOS 지도 커널의 확대 시안이다. M11과 G25의 기존 상태를 PASS로 변경하지 않는다.

## 기본 오프라인 세계지도 (2026-09-24)

| Node | 확인된 출력 | 검증 | 상태 |
|---|---|---|---|
| W0 세계 개략 | Natural Earth 1:110m z0–z4, 나라 이름 | 실제 Metal 세계 PNG, 타일 오류 0 | PASS_MAC |
| W1 전 세계 중간 상세 | 퍼블릭 도메인 1:50m 원본을 Rust로 z5–z7 타일화; 10,690개 타일, 1,608,226 B | 10,690개 전부 해독, 오류 0 | PASS_MAC |
| W2 지역 선택 | 동아시아 내부 1:10m, 유럽 등 밖 1:50m | 타일 선택 테스트, 양 지역 Metal PNG, 오류 0 | PASS_MAC |
| W3 길·시설 | 동아시아 기존 주요 도로 일부; 세계 도로·역·기관 미포함 | 출력 원본/화면 확인 | PARTIAL |
| W4 직접 측량 합성 | 현장 기록 0건, 세계지도와 합성 미구현 | 직접 기록 파일 검사 | NOT_STARTED |
| W5 실기기 | iPhone 지도 프레임·GPS 조사 미측정 | 기존 계정 기기 등록 제한 유지 | BLOCKED_ACCOUNT_DEVICE_LIMIT |

원본·화면·한계의 상세 기록은 [WORLD_MAP.md](WORLD_MAP.md). 이 노드들은 기존 M11/G25 상태를 변경하지 않는다.
