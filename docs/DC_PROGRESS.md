# 워싱턴 DC 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 DC FIPS `11`의 `11001` 파일을 각각 1개 찾았다. [도로 목록](../data/us_tiger_2025_dc_counties.txt)과 [수면 목록](../data/us_tiger_2025_dc_areawater.txt)에 공식 디렉터리 HTML SHA-256을 고정했다. ZIP별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_dc_roads.toml)와 [수면 manifest](../data/us_tiger_dc_areawater.toml)에 있다. 원본 ZIP은 빌드 입력이며 앱은 로컬 팩만 읽는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 1개, 1,168,176바이트 | 1개, 356,209바이트 |
| 원본 DBF 레코드 | 4,335개 | 141개 |
| 채택 | 4,283개 | 141개 |
| 제외 | 52개: 현재 차량 도로 분류 밖 | 0개 |
| GeoDB | 로컬 2,994,023바이트, SHA-256 `4e82b282411fa2bca8e8ede2074d78d3006a3d2769baa9083d5bfa033b1f5528` | 로컬 592,528바이트, SHA-256 `6332526bc84b0d30a29d36c5f489c842234f1b61033f09b5163d9870b83449ba` |
| PMTiles | [도로](../artifacts/world-roads/dc/state.pmtiles) 929,146바이트, SHA-256 `7401ef344b3c1880bd005299277f3758a6530e08c13ed9aba57d5216bd55076d` | [수면](../artifacts/world-water/dc/water.pmtiles) 241,554바이트, SHA-256 `8488719e7e5c854e01049ffbbdabcf598679eab8f8ba35ac39b9c45a474e0f1f` |
| 실제 타일 전수 해독 | [328개](../artifacts/world-roads/dc/tile-audit.log), 실패 0 | [208개](../artifacts/world-water/dc/tile-audit.log), 실패 0 |

Rust 감사는 ZIP별 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 원본 헤더의 숫자 좌표 범위는 도로 `[-77.116749, 38.79241, -76.909561, 38.995251]`, 수면 `[-77.119759, 38.791645, -76.913664, 38.986266]`다. 채택 피처의 출처는 GeoDB에 남겼고, [도로 제외](../artifacts/world-roads/dc/state.rejected.json.gz) 52건은 사유와 함께 보관했다.

기본 세계 모드 [DC·메릴랜드 도로·수면 z14 화면](../artifacts/world-water/dc/dc-road-water-z14.png)에서 도로와 Potomac 수면이 함께 표시됐고 두 지역 출처가 각각 표기됐다. Metal 타일 실패는 0이었다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 이는 파일·타일·화면 검증이다. DC·메릴랜드 경계 도로 연결성, 현장 위치 정확도, NAD83↔WGS84 독립 기준점 비교, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 11 2026-09-25 data/us_tiger_2025_dc_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 11 2026-09-25 data/us_tiger_2025_dc_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_dc_counties.txt data/local/dc_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_dc_areawater.txt data/local/dc_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_dc_counties.txt data/local/dc_roads data/us_tiger_dc_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_dc_areawater.txt data/local/dc_areawater data/us_tiger_dc_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_dc_roads.toml artifacts/world-roads/dc/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_dc_areawater.toml artifacts/world-water/dc/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_dc_roads.toml artifacts/world-roads/dc/state.mgeodb artifacts/world-roads/dc/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_dc_areawater.toml artifacts/world-water/dc/water.mgeodb artifacts/world-water/dc/water.pmtiles
```
