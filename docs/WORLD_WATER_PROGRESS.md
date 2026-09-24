# 상세 수면 원천 구축 현황 — 2026-09-25

뉴욕시 5개 카운티 수면, [매사추세츠주 14개 카운티 수면](MA_STATE_WATER_PROGRESS.md), [뉴햄프셔주 10개 카운티 수면](NH_STATE_PROGRESS.md), [버몬트주 14개 카운티 수면](VT_STATE_PROGRESS.md), [프랑스 IGN 파리 D075 상세 수면](FR_PARIS_WATER_PROGRESS.md)을 기본 세계 모드에 연결했다. 나머지 세계의 상세 수면은 아직 구축하지 않았다.

## 확인된 범위

기본 세계 모드에 뉴욕시 5개 카운티의 공식 수면 폴리곤을 지역 패키지로 추가했다. [미국 Census 2025 TIGER/Line Area Hydrography 배포 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 Bronx(36005), Kings(36047), New York(36061), Queens(36081), Richmond(36085)의 ZIP을 받았다. [2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 Appendix J-1은 카운티 단위 수면 Shapefile과 `HYDROID` 필드를 정의한다. 미국 정부 원천의 재배포 허용 조건과 출처 표기를 [manifest](../data/us_tiger_nyc_areawater.toml)에 기록했다. 공유조건은 없다.

| 단계 | 관측 결과 |
|---|---|
| 원본 감사 | 5개 ZIP의 SHA-256·NAD83 `.prj`·Polygon Shapefile 헤더·DBF 레코드 수를 Rust 감사 도구로 검사. 합계 299개 레코드, 원본 헤더 범위의 합집합 `[-74.258843, 40.476578, -73.72643, 40.917705]` |
| 정규화 | 299개 `Water` 폴리곤, 출처 299개, 거절 0개. 여러 외곽 링과 안쪽 링은 포함 관계를 확인해 그룹화하고 모호하거나 잘못된 도형은 거절하도록 구현 |
| 산출물 | [GeoDB](../artifacts/world-water/nyc-five-boroughs.mgeodb) 556,882바이트, [PMTiles](../artifacts/world-water/nyc-five-boroughs.pmtiles) 523,326바이트. 비어 있지 않은 타일 1,482개, 확대 단계별 중복 포함 수면 4,020개, 전체 해독 오류 0개 |
| 화면 | [Queens 도로·건물·수면 합성](../artifacts/world-integration/queens-roads-buildings-water-world.png) Mac Metal z14.4 캡처. 타일 실패 0개 |

원본 좌표계는 **EPSG:4269 NAD83**이고 현재 숫자 좌표를 그대로 Web Mercator에 투영한다. WGS84 datum 변환, 해안·수면의 독립 위치 정확도, 카운티 경계의 연결성은 검증되지 않았다. 이 자료는 5개 카운티의 수면 표면만 담는다. 다른 지역의 상세 수면이나 공원·육지 폴리곤을 완성했다는 뜻이 아니다. 런타임은 로컬 PMTiles를 읽으며 외부 지도 API를 호출하지 않는다.

## 재현

```bash
cargo run --offline -p mappa-map-data --bin audit_us_road_source -- \
  assets/map/source/public/us_tiger_2025_36*_areawater.zip
cargo run --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_nyc_areawater.toml artifacts/world-water/nyc-five-boroughs.mgeodb
cargo run --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_nyc_areawater.toml artifacts/world-water/nyc-five-boroughs.mgeodb \
  artifacts/world-water/nyc-five-boroughs.pmtiles
cargo run --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-water/nyc-five-boroughs.pmtiles
```
