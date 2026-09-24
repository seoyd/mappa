# 미국 뉴햄프셔주 도로·수면 확장 — 2026-09-25

[US Census 2025 TIGER/Line 도로 목록](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)과 [수면 목록](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `33`의 카운티 파일 10개씩을 확인했다. [도로 파일 목록](../data/us_tiger_2025_nh_counties.txt)과 [수면 파일 목록](../data/us_tiger_2025_nh_areawater.txt)을 고정하고 Rust 수집기로 20개를 받았다. 원본 ZIP은 빌드 입력이며 앱은 로컬 PMTiles만 읽는다. 파일별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_nh_state_roads.toml)와 [수면 manifest](../data/us_tiger_nh_state_areawater.toml)에 기록했다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 10개, 19,122,890바이트 | 10개, 3,612,808바이트 |
| 원본 레코드 | 81,426개 | 5,904개 |
| 채택 | 68,617개 | 5,897개 |
| 제외 | 12,809개: 현재 차량 도로 분류 밖 | 7개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 44,416,407바이트, SHA-256 `a9455dc68b2a09c0c4ac31588aea86cb2f3cb5533e96742eccbc7c5f9c34e7e9` | 로컬 6,668,428바이트, SHA-256 `e76080e0ef3081cf5cbc98fbe8e45ecbe4c43835cd24a8fe5a83cef154fe1953` |
| PMTiles | [도로](../artifacts/world-roads/nh-state/state.pmtiles) 19,054,436바이트, SHA-256 `81d5bf592b52acf5849e5dac956b20a8d3af895a682426bc44008dbf6eb60ffc` | [수면](../artifacts/world-water/nh-state/water.pmtiles) 5,748,653바이트, SHA-256 `8c0a9cbf0b42a0efd02ce23b748fdf69e805f7661eb7525104c22ee31f79d0ac` |
| 타일 감사 | [32,213개 전수 해독](../artifacts/world-roads/nh-state/tile-audit.log), 실패 0 | [15,406개 전수 해독](../artifacts/world-water/nh-state/tile-audit.log), 실패 0 |

도로 원본 Shapefile 헤더의 숫자 좌표 범위 합집합은 `[-72.556099, 42.697316, -70.610951, 45.305378]`, 수면은 `[-72.557124, 42.697611, -70.575094, 45.298787]`이었다. Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·SHA-256을 확인했다. 채택 피처와 원본 레코드 ID의 연결은 GeoDB에 남겼다. 제외 건은 [도로](../artifacts/world-roads/nh-state/state.rejected.json.gz)와 [수면](../artifacts/world-water/nh-state/water.rejected.json.gz) 기록으로 보관했다.

기본 세계 모드의 [Concord z14 도로·수면 화면](../artifacts/world-water/nh-state/concord-road-water-z14.png)에서 강과 도로가 함께 보였고 Metal 렌더러 타일 실패는 0이었다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 정부 원천 재사용 조건과 출처 표기 규칙을 적용했다. 이 결과는 파일·타일·화면 검증이다. 실제 도로·수면의 완전성, 카운티·주 경계 연결성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release -p mappa-map-acquire -- \
  data/us_tiger_2025_nh_counties.txt data/local/nh_roads
cargo run --release -p mappa-map-acquire -- --areawater \
  data/us_tiger_2025_nh_areawater.txt data/local/nh_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_nh_counties.txt data/local/nh_roads \
  data/us_tiger_nh_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_nh_areawater.txt data/local/nh_areawater \
  data/us_tiger_nh_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_nh_state_roads.toml artifacts/world-roads/nh-state/state.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_nh_state_areawater.toml artifacts/world-water/nh-state/water.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_nh_state_roads.toml artifacts/world-roads/nh-state/state.mgeodb \
  artifacts/world-roads/nh-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_nh_state_areawater.toml artifacts/world-water/nh-state/water.mgeodb \
  artifacts/world-water/nh-state/water.pmtiles
```
