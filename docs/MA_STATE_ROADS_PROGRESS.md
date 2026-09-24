# 미국 매사추세츠주 공식 도로 확장 — 2026-09-25

## 확인한 범위

[Census 2025 TIGER/Line All Roads 공식 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)에서 주 FIPS `25`인 [14개 카운티 ZIP 목록](../data/us_tiger_2025_ma_counties.txt)을 확인하고 전부 확보했다. [원천·SHA·권리 manifest](../data/us_tiger_ma_state_roads.toml)는 ZIP마다 별도 출처를 기록한다. 원본은 빌드에만 사용하며 앱은 로컬 지도 팩을 읽는다.

| 단계 | 관측 사실 |
|---|---|
| 원본 | ZIP 14개 합계 40,196,437바이트. Rust 감사가 각 파일의 Shapefile 헤더·DBF 레코드 수·NAD83 `.prj`·SHA-256을 확인했다. 원본 도로 레코드 216,775개. 헤더 숫자 좌표 범위의 합집합 `[-73.497277, 41.239513, -69.929137, 42.886777]`. |
| Rust 정규화 | 기존 Census 차량 도로 분류 규칙으로 **210,165개 채택**, 보행·자전거 등 규칙 밖 6,610개 거절. [카운티별 원천·채택·거절 수](../artifacts/world-roads/ma-state/source-counts.csv)에서 14개 모두 채택 피처가 있다. [압축 거절 기록](../artifacts/world-roads/ma-state/state.rejected.json.gz)을 보관했다. |
| GeoDB | 피처·출처 각 210,165개, 로컬 115,447,663바이트, SHA-256 `53c45c20cfa79e1d5190ff9d1a581f31ede832cbddbc8fc20d46f0397a12506e`. |
| 지도 팩 | [Massachusetts PMTiles](../artifacts/world-roads/ma-state/state.pmtiles) 37,066,020바이트, SHA-256 `971b8f303fa5a73d707859b300b8a389412f39507491bbcde4434e553cf78e8a`. z10–15 실제 [33,938개 타일 전수 해독](../artifacts/world-roads/ma-state/tile-audit.log), 부재·실패 0개. 타일별 중복 포함 도로선 1,096,232개. |
| 화면 | 기본 세계 모드 [Boston z14 Metal 캡처](../artifacts/world-roads/ma-state/boston-world-z14.png), 타일 실패 0, Census 출처 표기 확인. |

[Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)에 따른 미국 정부 원천 재사용·출처 표기 조건을 적용했다. 14개 카운티 입력을 전부 처리했다는 뜻이며 실제 도로 누락 없음, 인접 주 경계 연결성, NAD83에서 WGS84로의 독립 위치 정확도는 검증하지 않았다. [매사추세츠 상세 수면](MA_STATE_WATER_PROGRESS.md)은 별도 팩으로 합성한다. 건물·역·공공기관과 iPhone 실기기 성능은 미검증이다.

## 재현

```bash
cargo run --release -p mappa-map-acquire -- \
  data/us_tiger_2025_ma_counties.txt data/local/ma_roads
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_ma_counties.txt data/local/ma_roads \
  data/us_tiger_ma_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_ma_state_roads.toml artifacts/world-roads/ma-state/state.mgeodb \
  --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_ma_state_roads.toml artifacts/world-roads/ma-state/state.mgeodb \
  artifacts/world-roads/ma-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/ma-state/state.pmtiles
```
