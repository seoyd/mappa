# Delaware 공식 도로 원천 확장 — 2026-09-24

## 관측한 결과

[미국 Census 2025 TIGER/Line All Roads 공식 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)에서 Delaware FIPS `10`인 카운티 ZIP 3개를 확인해 [고정 목록](../data/us_tiger_2025_de_counties.txt)에 기록했다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건에 따라 [출처·SHA·권리 manifest](../data/us_tiger_de_state_roads.toml)를 만들었다. 앱 실행 중에는 Census 서버나 지도 API를 호출하지 않는다.

| 단계 | 검증된 사실 |
|---|---|
| 원본 | 3개 ZIP 7,873,507바이트. `.shp` Polyline 헤더·`.dbf`·NAD83(EPSG:4269) `.prj`와 SHA-256을 검사했다. 원본 DBF 레코드 33,395개. |
| 범위 | 원본 Shapefile 헤더의 숫자 좌표 합집합 `[-75.788894, 38.451176, -75.049844, 39.839476]`. WGS84 독립 위치 정확도 측정값이 아니다. |
| Rust 변환 | 도로 종류 규칙으로 **32,518개 채택**, 보행·자전거 등 규칙 밖 **877개 거절**. 카운티 3개가 모두 기여했다. [원천별 수](../artifacts/world-roads/de-state/source-counts.csv)와 [압축 거절 기록](../artifacts/world-roads/de-state/state.rejected.json.gz)을 보관한다. |
| GeoDB | 빌드용 로컬 MappaGeoDB 20,525,602바이트, SHA-256 `77b8e027ee809ff0ed17131f32b1ebd0091b47aa9161f9dc23ee9db53ad94cba`. 원본별 provenance 32,518개. |
| 오프라인 팩 | [Delaware PMTiles](../artifacts/world-roads/de-state/state.pmtiles) 6,720,290바이트, SHA-256 `3268439deea9a598cfdfad068837ebac6026db9fefe93a4aea0202d9e855157e`. z10–15 실제 타일 7,409개를 [전수 해독](../artifacts/world-roads/de-state/tile-audit.log)해 부재 0·실패 0. |
| 화면 | [Dover z14 Metal 화면](../artifacts/world-roads/de-state/dover-world-z14.png) 타일 실패 0, Census 출처 표기 확인. |

이 팩은 공식 All Roads에서 Mappa가 승인한 차량 도로 분류의 **선형**만 담는다. 카운티 사이 도로 중복·연결성, 실제 도로 누락 여부, WGS84 datum 차이, 현장 위치 정확도, iPhone 성능은 아직 검증하지 않았다. Delaware 상세 수면·건물·역·공공기관은 구축하지 않았으며 빈 곳을 임의 도형으로 채우지 않는다.

## 재현

```bash
# 목록은 공식 Census ROADS 디렉터리의 2025 Delaware FIPS 10 ZIP에서 생성했다.
cargo run --release -p mappa-map-acquire -- \
  data/us_tiger_2025_de_counties.txt data/local/de_roads
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_de_counties.txt data/local/de_roads \
  data/us_tiger_de_state_roads.toml 2026-09-24
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_de_state_roads.toml artifacts/world-roads/de-state/state.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_de_state_roads.toml artifacts/world-roads/de-state/state.mgeodb \
  artifacts/world-roads/de-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/de-state/state.pmtiles
```
