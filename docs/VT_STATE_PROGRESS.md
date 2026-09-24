# 미국 버몬트주 도로·수면 확장 — 2026-09-25

[US Census 2025 TIGER/Line 도로](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/) 공식 디렉터리의 주 FIPS `50` 카운티 ZIP을 각각 14개 확인했다. [도로 목록](../data/us_tiger_2025_vt_counties.txt)과 [수면 목록](../data/us_tiger_2025_vt_areawater.txt)을 고정하고 Rust 수집기로 받았다. ZIP별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_vt_state_roads.toml)와 [수면 manifest](../data/us_tiger_vt_state_areawater.toml)에 있다. 원본 ZIP은 빌드에만 쓰고 런타임은 로컬 팩을 읽는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 14개, 15,670,338바이트 | 14개, 7,868,344바이트 |
| 원본 DBF 레코드 | 52,772개 | 14,569개 |
| 채택 | 43,060개 | 14,567개 |
| 제외 | 9,712개: 현재 차량 도로 분류 밖 | 2개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 32,659,222바이트, SHA-256 `3821f4dfc84a6fff192293e8acfe801b27b222a8cf1b0606aac6eb262217c762` | 로컬 15,671,631바이트, SHA-256 `3fbe0f3124c52310486bb2890efdc5708ef6a2dea27acbc229c028262a86c228` |
| PMTiles | [도로](../artifacts/world-roads/vt-state/state.pmtiles) 16,931,945바이트, SHA-256 `3fabe8cd6d47ffcaab5af95a873aaf14c3d07506bdec51119adc1d20f5585c8b` | [수면](../artifacts/world-water/vt-state/water.pmtiles) 9,517,456바이트, SHA-256 `5f4590695440521f4352875a952f314d873e59516301a1bb08bd8b1beabac067` |
| 타일 감사 | [33,631개 전수 해독](../artifacts/world-roads/vt-state/tile-audit.log), 실패 0 | [21,740개 전수 해독](../artifacts/world-water/vt-state/tile-audit.log), 실패 0 |

도로 Shapefile 헤더의 숫자 좌표 범위 합집합은 `[-73.423258, 42.726964, -71.466387, 45.01634]`, 수면은 `[-73.437905, 42.732921, -71.465047, 45.015748]`이었다. Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 채택 피처와 원본 ID의 연결은 GeoDB에, [도로 제외](../artifacts/world-roads/vt-state/state.rejected.json.gz)와 [수면 제외](../artifacts/world-water/vt-state/water.rejected.json.gz) 사유는 압축 파일에 남겼다.

기본 세계 모드 [Burlington z14 화면](../artifacts/world-water/vt-state/burlington-road-water-z14.png)에 Champlain 호수와 도로가 함께 표시됐고 Metal 타일 실패는 0이었다. 화면에 인접한 뉴욕주 도로 팩도 선택됐지만 주 경계의 실제 도로 연결성을 검증했다는 뜻은 아니다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 정부 원천 재사용과 출처 표기를 적용했다. 실제 도로·수면의 완전성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 아직 검증하지 않았다.

## 재현

```bash
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_vt_counties.txt data/local/vt_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_vt_areawater.txt data/local/vt_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_vt_counties.txt data/local/vt_roads data/us_tiger_vt_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_vt_areawater.txt data/local/vt_areawater data/us_tiger_vt_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_vt_state_roads.toml artifacts/world-roads/vt-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_vt_state_areawater.toml artifacts/world-water/vt-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_vt_state_roads.toml artifacts/world-roads/vt-state/state.mgeodb artifacts/world-roads/vt-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_vt_state_areawater.toml artifacts/world-water/vt-state/water.mgeodb artifacts/world-water/vt-state/water.pmtiles
```
