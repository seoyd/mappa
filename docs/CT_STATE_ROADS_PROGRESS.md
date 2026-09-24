# Connecticut 공식 도로 원천 확장 — 2026-09-24

## 확인한 범위

[Census 2025 TIGER/Line All Roads 공식 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)에서 주 FIPS `09`인 [9개 ZIP 목록](../data/us_tiger_2025_ct_counties.txt)을 추출했다. 2025 지리 체계에서 이 단위는 과거 8개 카운티가 아닌 [9개 계획지역](https://tigerweb.geo.census.gov/tigerwebmain/Files/acs25/tigerweb_acs25_county_ct.html)이다. 따라서 2025 파일명에 실제로 나타난 9개 FIPS를 사용했다. [출처·SHA·권리 manifest](../data/us_tiger_ct_state_roads.toml)는 각 ZIP을 별도 원천으로 기록한다. 앱은 로컬 지도 팩만 읽는다.

| 단계 | 관측 사실 |
|---|---|
| 원본 | ZIP 9개 합계 24,364,107바이트. `.shp` Polyline 헤더·`.dbf` 레코드·NAD83(EPSG:4269) `.prj`·SHA-256 검사. 원본 레코드 94,843개. |
| 원본 범위 | 헤더의 숫자 좌표 합집합 `[-73.726172, 40.988237, -71.787649, 42.050476]`. WGS84 독립 위치 정확도 값은 아니다. |
| Rust 정규화 | 허용된 차량 도로 분류 **87,833개 채택**, 규칙 밖 보행·자전거 등 **7,010개 거절**. 9개 계획지역 모두 한 개 이상 기여. [원천별 수](../artifacts/world-roads/ct-state/source-counts.csv), [압축 거절 기록](../artifacts/world-roads/ct-state/state.rejected.json.gz) 보관. |
| GeoDB | 로컬 빌드 산출물 57,753,044바이트, SHA-256 `303b627830deb01d831f420b9649b477d058a3980bed621e08ce95660238f115`; 피처별 provenance 87,833개. |
| 지도 팩 | [Connecticut PMTiles](../artifacts/world-roads/ct-state/state.pmtiles) 21,122,303바이트, SHA-256 `33e6bf4e24bc7920177d531f73766061ee1c02b383e9ffb6b24849078e2d6f5e`. z10–15의 [실제 타일 20,398개 전수 해독](../artifacts/world-roads/ct-state/tile-audit.log)에서 부재 0·실패 0. |
| 화면 | [Hartford z14 Metal 캡처](../artifacts/world-roads/ct-state/hartford-world-z14.png) 타일 실패 0, Census 출처 표기 확인. |

[Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건을 적용했다. 9개 계획지역 입력을 모두 처리했다는 뜻이며 실제 도로 누락 없음, 뉴욕·매사추세츠·로드아일랜드 주 경계 연결성, 도로의 현장 위치 정확도는 검증하지 않았다. 상세 수면·건물·역·공공기관과 iPhone 실기기 성능도 아직 확인하지 않았다.

## 재현

```bash
cargo run --release -p mappa-map-acquire -- \
  data/us_tiger_2025_ct_counties.txt data/local/ct_roads
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_ct_counties.txt data/local/ct_roads \
  data/us_tiger_ct_state_roads.toml 2026-09-24
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_ct_state_roads.toml artifacts/world-roads/ct-state/state.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_ct_state_roads.toml artifacts/world-roads/ct-state/state.mgeodb \
  artifacts/world-roads/ct-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/ct-state/state.pmtiles
```
