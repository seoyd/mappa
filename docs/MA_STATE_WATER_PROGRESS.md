# 미국 매사추세츠주 공식 상세 수면 확장 — 2026-09-25

## 확인한 범위

[Census 2025 TIGER/Line Area Hydrography 공식 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `25`의 [14개 카운티 ZIP 목록](../data/us_tiger_2025_ma_areawater.txt)을 확인하고 전부 확보했다. [원천·SHA·권리 manifest](../data/us_tiger_ma_state_areawater.toml)는 카운티별 파일을 기록한다. 원본은 빌드 입력이며 기본 지도는 로컬 타일만 읽는다.

| 단계 | 관측 사실 |
|---|---|
| 원본 | ZIP 14개 합계 10,511,553바이트. Rust 감사가 파일별 Polygon Shapefile 헤더·DBF 레코드 수·NAD83 `.prj`·SHA-256을 확인했다. 원본 폴리곤 23,651개, 헤더 숫자 좌표 범위 합집합 `[-73.494264, 41.187053, -69.858861, 42.886151]`. |
| 정규화 | **23,633개 채택**, 내부 링의 짝이 모호한 18개 거절. [카운티별 수](../artifacts/world-water/ma-state/source-counts.csv)에서 14개 카운티 모두 수면 형상이 기여했고, [압축 거절 기록](../artifacts/world-water/ma-state/water.rejected.json.gz)을 보관했다. |
| GeoDB | 피처·출처 각 23,633개, 로컬 21,835,957바이트, SHA-256 `9580a8a115298118823cf36207ca414c837bfea42dc7fcd458394c64ff30c29d`. |
| 지도 팩 | [Massachusetts 수면 PMTiles](../artifacts/world-water/ma-state/water.pmtiles) 13,248,797바이트, SHA-256 `870408364999c8defa7668c4d1de90a41b9e38d6055dfcf53e42577e889ba64e`. z10–15 실제 [31,357개 타일 전수 해독](../artifacts/world-water/ma-state/tile-audit.log), 부재·실패 0개. 타일별 중복 포함 수면 폴리곤 150,156개. |
| 화면 | 기본 세계 모드 [Boston 도로·수면 z14 Metal 캡처](../artifacts/world-water/ma-state/boston-road-water-z14.png), 타일 실패 0, Census 출처 표기 확인. |

[Census 2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 14개 파일의 채택 가능한 폴리곤을 표시한 결과이며, 거절된 18개 형상을 임의로 고쳐 채우지 않았다. 실제 수면 누락 없음, 인접 카운티 경계 연결성, NAD83에서 WGS84로의 독립 위치 정확도, iPhone 성능은 검증하지 않았다. 건물·역·공공기관은 아직 없다.

## 재현

```bash
cargo run --release -p mappa-map-acquire -- --areawater \
  data/us_tiger_2025_ma_areawater.txt data/local/ma_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_ma_areawater.txt data/local/ma_areawater \
  data/us_tiger_ma_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_ma_state_areawater.toml artifacts/world-water/ma-state/water.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_ma_state_areawater.toml artifacts/world-water/ma-state/water.mgeodb \
  artifacts/world-water/ma-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-water/ma-state/water.pmtiles
```
