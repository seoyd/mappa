# Rhode Island 공식 도로 원천 확장 — 2026-09-24

## 확인한 범위

[Census 2025 TIGER/Line All Roads 공식 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)에서 주 FIPS `44`인 [5개 ZIP 목록](../data/us_tiger_2025_ri_counties.txt)을 추출했다. [출처·SHA·권리 manifest](../data/us_tiger_ri_state_roads.toml)는 각 ZIP을 별도 원천으로 기록한다. 앱은 로컬 지도 팩만 읽는다.

| 단계 | 관측 사실 |
|---|---|
| 원본 | ZIP 5개 합계 5,840,495바이트. `.shp` Polyline 헤더·`.dbf` 레코드·NAD83(EPSG:4269) `.prj`·SHA-256 검사. 원본 레코드 30,683개. |
| 원본 범위 | 헤더의 숫자 좌표 합집합 `[-71.861023, 41.147566, -71.121203, 42.018791]`. WGS84 독립 위치 정확도 값은 아니다. |
| Rust 정규화 | 허용된 차량 도로 분류 **29,931개 채택**, 규칙 밖 보행·자전거 등 **752개 거절**. 5개 카운티 모두 한 개 이상 기여. [원천별 수](../artifacts/world-roads/ri-state/source-counts.csv), [압축 거절 기록](../artifacts/world-roads/ri-state/state.rejected.json.gz) 보관. |
| GeoDB | 로컬 빌드 산출물 17,319,573바이트, SHA-256 `e333e4a77b0c1cd26f79b043ba9c75e8944d15d0cf839646121bd006d1897f8f`; 피처별 provenance 29,931개. |
| 지도 팩 | [Rhode Island PMTiles](../artifacts/world-roads/ri-state/state.pmtiles) 5,631,726바이트, SHA-256 `a2c8bbde19d5a3d5d803a11969441ff854b700cbefb2e6666729a6506f8c25e2`. z10–15의 [실제 타일 4,867개 전수 해독](../artifacts/world-roads/ri-state/tile-audit.log)에서 부재 0·실패 0. |
| 화면 | [Providence z14 Metal 캡처](../artifacts/world-roads/ri-state/providence-world-z14.png) 타일 실패 0, Census 출처 표기 확인. |

[Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건을 적용했다. 5개 카운티 입력을 모두 처리했다는 뜻이며 실제 도로 누락 없음, 코네티컷·매사추세츠 주 경계 연결성, 도로의 현장 위치 정확도는 검증하지 않았다. 상세 수면·건물·역·공공기관과 iPhone 실기기 성능도 아직 확인하지 않았다.

## 재현

```bash
cargo run --release -p mappa-map-acquire -- \
  data/us_tiger_2025_ri_counties.txt data/local/ri_roads
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_ri_counties.txt data/local/ri_roads \
  data/us_tiger_ri_state_roads.toml 2026-09-24
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_ri_state_roads.toml artifacts/world-roads/ri-state/state.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_ri_state_roads.toml artifacts/world-roads/ri-state/state.mgeodb \
  artifacts/world-roads/ri-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/ri-state/state.pmtiles
```
