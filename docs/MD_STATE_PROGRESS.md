# 미국 메릴랜드주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `24`의 파일을 각각 24개 찾았다. 두 목록의 카운티 상당 단위 번호 24개가 모두 일치하며 Baltimore 독립시 `24510`이 포함된다. [도로 목록](../data/us_tiger_2025_md_counties.txt)과 [수면 목록](../data/us_tiger_2025_md_areawater.txt)에 공식 디렉터리 HTML SHA-256을 고정했다. 48개 ZIP의 개별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_md_state_roads.toml)와 [수면 manifest](../data/us_tiger_md_state_areawater.toml)에 있다. 원본 ZIP은 빌드 입력이며 앱은 로컬 지도 팩을 읽는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 24개, 43,534,419바이트 | 24개, 13,682,075바이트 |
| 원본 DBF 레코드 | 224,002개 | 14,905개 |
| 채택 | 212,347개 | 14,888개 |
| 제외 | 11,655개: 현재 차량 도로 분류 밖 | 17개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 120,283,277바이트, SHA-256 `9b79a4692b9d65cd8ba95dd8fc0b91a1a20787d2511e28dcb1a1608927c8cbd5` | 로컬 25,418,301바이트, SHA-256 `3d0709262a4ef8dfdf7d2f08dcf3a4967e61167e68458f6b0522f33b944b0b18` |
| PMTiles | [도로](../artifacts/world-roads/md-state/state.pmtiles) 39,275,060바이트, SHA-256 `80c7f214266cdb1655cd8dddacb044aa100fa5f5269e8c9462d21282fdd6e55a` | [수면](../artifacts/world-water/md-state/water.pmtiles) 15,374,131바이트, SHA-256 `bfb42b0af9c8c4e290ffec0ad7086c13eb19b2d9e2810878814f1bb157ca3a01` |
| 실제 타일 전수 해독 | [38,627개](../artifacts/world-roads/md-state/tile-audit.log), 실패 0 | [29,258개](../artifacts/world-water/md-state/tile-audit.log), 실패 0 |

Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-79.48765, 37.956046, -75.050439, 39.723012]`, 수면 `[-79.468594, 37.886605, -74.986282, 39.723017]`이다. 채택 피처와 원본 ID의 연결은 GeoDB에, [도로 제외](../artifacts/world-roads/md-state/state.rejected.json.gz)와 [수면 제외](../artifacts/world-water/md-state/water.rejected.json.gz) 사유는 별도 파일에 남겼다.

기본 세계 모드 [Baltimore z14 도로·수면 화면](../artifacts/world-water/md-state/baltimore-road-water-z14.png)에 시가지 도로와 항만 수면이 함께 표시됐고 Metal 타일 실패는 0이었다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 이 결과는 파일·타일·화면 검증이다. 실제 지형의 누락 없음, 카운티·주 경계 연결성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 24 2026-09-25 data/us_tiger_2025_md_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 24 2026-09-25 data/us_tiger_2025_md_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_md_counties.txt data/local/md_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_md_areawater.txt data/local/md_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_md_counties.txt data/local/md_roads data/us_tiger_md_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_md_areawater.txt data/local/md_areawater data/us_tiger_md_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_md_state_roads.toml artifacts/world-roads/md-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_md_state_areawater.toml artifacts/world-water/md-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_md_state_roads.toml artifacts/world-roads/md-state/state.mgeodb artifacts/world-roads/md-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_md_state_areawater.toml artifacts/world-water/md-state/water.mgeodb artifacts/world-water/md-state/water.pmtiles
```
