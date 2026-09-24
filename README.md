# Mappa (Rust 오프라인 세계지도 / 실기기 검증 진행 중)

## 자체 GeoDB 지역 실증 (v0.3C)

나주 10km 안팎의 도로 중심선·도로면, 공식 SGIS 읍면동 지명 14개, ESA의 2021 영구수면·수목 피복을 [출처·라이선스 manifest](data/sources.toml) → Rust 어댑터 → MappaGeoDB → 자체 z10–z15 PMTiles → 기존 Rust 렌더러로 연결했다. 이 경로는 OSM/Natural Earth를 읽지 않고 운영 중 지도 API 비용이 없다. **지역 실증은 PARTIAL이며 S16은 NO_GO**다. 건물·철도·법정 공원 경계와 독립 위치 정확도 검증이 없어 자체 세계 상세지도가 완성됐다고 판단하지 않는다. [방향](docs/MAP_PHILOSOPHY.md), [원천](docs/MAP_SOURCES.md), [품질 판정](docs/MAP_V0_3C_GATE_AUDIT.md), [측정](docs/BENCHMARK_MAP_V0_3C.md)을 함께 확인한다.

[나주 도로와 송월동 지명 시안](artifacts/map-v0.3c/naju-district-z14.png)

```bash
cargo run -p mappa-map-data --bin build_canonical_proof -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb
cargo run -p mappa-map-data --bin build_canonical_tiles -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb artifacts/map-v0.3c/naju-roads.pmtiles
MAPPA_DATASET=canonical-proof cargo run -p mappa-map-demo -- --street-demo 126.715 35.025 15.2
```

기본 세계지도의 저배율은 Natural Earth 기반이다. 고배율에는 기존 지역 패키지 8개, 영국 Ordnance Survey Open Roads의 GB 격자 패키지 52개, 캐나다 NRN 13개 지역 도로 팩 15개를 필요할 때 연다. GB 팩은 일반화된 도로 선형이며 북아일랜드를 포함하지 않는다. 나주 자체 GeoDB 시안은 S16 `NO_GO` 상태이므로 `canonical-proof`를 명시해야 연다.

MVT/PMTiles 교체 가능성은 [MSP 실험 결과](docs/MAP_SPATIAL_PACK_EXPERIMENT.md)로 별도 검증 중이다. 같은 나주 GeoDB에서 geometry 1회 저장·공간 셀 참조·정수 delta 좌표를 구현해 Metal 화면까지 확인했다. 현재 MSP 파일은 기존 PMTiles의 1.499배여서 기본 지도 규격은 바꾸지 않았다.

## 세계지도 (기본 모드)

기본 지도는 Mappa의 Rust 타일 생성기·렌더러가 로컬 파일을 직접 읽어 그린다. 세계 z0–z4는 Natural Earth 1:110m, 전 세계 z5–z7은 1:10m 개략 원본을 쓴다. z6부터 주요 강·하천 중심선과 선별된 도시·주요 도로를 표시한다. 지형·국경·수계·도시·일부 도로의 좌표 원본은 [퍼블릭 도메인 Natural Earth](https://www.naturalearthdata.com/about/terms-of-use/)이고, 외부 지도 서버/API는 호출하지 않는다. 고배율에서는 [기존 지역 목록](assets/map/regional_packs.toml)의 모나코·Queens 건물, 뉴욕주·뉴저지주·델라웨어주·코네티컷주·로드아일랜드주·매사추세츠주·뉴햄프셔주·버몬트주·메인주·메릴랜드주·펜실베이니아주·워싱턴 DC 246개 카운티 상당 단위 도로와 프랑스 IGN 파리 D075 도로·수면·`FICTIF=Non` 여객역, 매사추세츠주·뉴햄프셔주·버몬트주·메인주·메릴랜드주·펜실베이니아주·워싱턴 DC·뉴욕시 수면과 뉴욕시 공원 경계와 [GB 도로 목록](assets/map/gb_regional_packs.toml)의 52개 지역 PMTiles 및 [캐나다 13개 지역 목록](assets/map/ca_regional_packs.toml)의 15개 팩을 읽는다. 미수집 지역은 빈 중립색으로 표시한다. 전 세계 건물·역·정밀 도로망은 여전히 없다. [건물 현황](docs/WORLD_BUILDINGS_PROGRESS.md), [뉴욕주 도로 확장](docs/NY_STATE_ROADS_PROGRESS.md), [뉴저지주 도로 확장](docs/NJ_STATE_ROADS_PROGRESS.md), [델라웨어주 도로 확장](docs/DE_STATE_ROADS_PROGRESS.md), [코네티컷주 도로 확장](docs/CT_STATE_ROADS_PROGRESS.md), [로드아일랜드주 도로 확장](docs/RI_STATE_ROADS_PROGRESS.md), [매사추세츠주 도로](docs/MA_STATE_ROADS_PROGRESS.md)·[수면](docs/MA_STATE_WATER_PROGRESS.md) 확장, [뉴햄프셔주 도로·수면](docs/NH_STATE_PROGRESS.md)·[버몬트주 도로·수면](docs/VT_STATE_PROGRESS.md)·[메인주 도로·수면](docs/ME_STATE_PROGRESS.md)·[메릴랜드주 도로·수면](docs/MD_STATE_PROGRESS.md)·[펜실베이니아주 도로·수면](docs/PA_STATE_PROGRESS.md)·[워싱턴 DC 도로·수면](docs/DC_PROGRESS.md) 확장, [프랑스 파리 도로](docs/FR_PARIS_ROADS_PROGRESS.md)·[수면](docs/FR_PARIS_WATER_PROGRESS.md)·[여객역](docs/FR_PARIS_STATIONS_PROGRESS.md) 실증, [GB 도로 확장](docs/GB_ROADS_PROGRESS.md), [캐나다 도로 확장](docs/CA_ROADS_PROGRESS.md), [국가별 도로 현황](docs/WORLD_ROADS_PROGRESS.md), [수면 현황](docs/WORLD_WATER_PROGRESS.md), [공원 현황](docs/WORLD_PARKS_PROGRESS.md), [지도 레이어·선택 구조](docs/MAP_LAYER_ARCHITECTURE.md), [세계지도 검증 결과](docs/WORLD_MAP.md)에 현재 상태를 구분해 기록했다.

```bash
cargo run -p mappa-map-demo
cargo run -p mappa-map-demo -- --capture 0 15 1.3 artifacts/world-map/world.png
cargo run -p mappa-map-data --bin audit_fixture -- assets/map/world_10m.pmtiles
```

## 직접 기록만 보는 모드

현장 조사 좌표만 확인하려면 `MAPPA_DATASET=first-party`를 명시한다. 현재 직접 기록은 0건이므로 이 모드는 빈 화면이다. 입력·검증 절차는 [자체 기록 지도 문서](docs/FIRST_PARTY_MAP.md)에 있다. 세계지도와 직접 기록 데이터는 현재 별도 모드이며 합쳐서 표시하지 않는다. iPhone의 GPS 직접 수집은 개발자 서명·실기기 검증이 진행되지 않아 아직 연결되지 않았다.

```bash
cargo run -p mappa-map-data --bin first_party_map -- inspect assets/map/first_party/survey.geojson
cargo run -p mappa-map-data --bin first_party_map -- build assets/map/first_party/survey.geojson assets/map/first_party.pmtiles
MAPPA_DATASET=first-party cargo run -p mappa-map-demo -- --street-demo
```

## 이전 비교 시안

과거 비교용 `MAPPA_DATASET=legacy-osm`과 `MAPPA_DATASET=public-naju`는 각각 [OSM 시안](docs/MAP_DATA.md), [공공 도로 시안](docs/PUBLIC_ROADS_PILOT.md)의 외부 원본을 쓴다. 명시적으로 선택해야만 연다. 지도 렌더러의 조작과 성능 기록은 [지도 구조](docs/MAP_ARCHITECTURE.md), [화면 갱신 측정](docs/BENCHMARK_MAP_ASYNC.md)을 참고한다.

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
