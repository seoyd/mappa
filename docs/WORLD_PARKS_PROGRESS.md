# 뉴욕시 주변 공원 경계 실증 — 2026-09-24

## 판정

[미국 Census 2025 TIGER/Line 뉴욕주 Area Landmark ZIP](https://www2.census.gov/geo/tiger/TIGER2025/AREALM/tl_2025_36_arealm.zip)을 읽고, [2025 MTFCC 정의](https://www2.census.gov/geo/pdfs/reference/mtfccs2025.pdf)의 공원·숲·휴양 구역 `K2180`–`K2190`만 선택했다. 이들은 행정·랜드마크 경계이며 실제 수목 피복이나 모든 공원의 완전한 목록이 아니다. 미국 정부 원천의 재배포 허용 조건과 출처를 [manifest](../data/us_tiger_nyc_parks.toml)에 기록했다.

| 단계 | 관측 결과 |
|---|---|
| 입력 | 뉴욕주 ZIP SHA-256 `2c40daa05cec76eafb0bc1f5dc4e15f6077aa94031453a876638107e116912d8`, EPSG:4269 NAD83 Polygon Shapefile, DBF 5,547개 레코드 |
| 선택 | 공원 코드 외 4,578개 제외, 링 관계가 모호한 5개 거절. 뉴욕시 도로·수면 원본 헤더 범위와 겹치는 공원 도형 178개 채택, provenance 178개. [전체 제외·거절 기록](../artifacts/world-parks/nyc-source-extent.rejected.json) |
| 산출물 | [GeoDB](../artifacts/world-parks/nyc-source-extent.mgeodb) 215,014바이트, [PMTiles](../artifacts/world-parks/nyc-source-extent.pmtiles) 252,906바이트. 비어 있지 않은 타일 963개, 확대 단계별 중복 포함 공원 1,722개, 전체 해독 오류 0개 |
| 화면 | [Queens의 도로·건물·수면·공원](../artifacts/world-integration/queens-four-layers-world.png) Mac Metal z14.4 캡처, 타일 실패 0개 |

공원은 GeoDB의 `Park` 종류로 보존하고 화면 타일의 `green` 레이어로 표시한다. 따라서 초록 면적은 자연 식생 조사 결과를 뜻하지 않는다. z12부터 보이며, 중첩된 수면은 화면 순서상 공원 위에 그린다. 원본 좌표의 WGS84 datum 변환·독립 위치 정확도·공원 목록의 완전성은 검증되지 않았다. 선택 범위는 뉴욕시 원본 도로·수면 헤더의 합집합 사각형으로, 인접 지역이 포함될 수 있다.

## 재현

```bash
cargo run --offline -p mappa-map-data --bin audit_us_road_source -- \
  assets/map/source/public/us_tiger_2025_36_arealm.zip
cargo run --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_nyc_parks.toml artifacts/world-parks/nyc-source-extent.mgeodb
cargo run --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_nyc_parks.toml artifacts/world-parks/nyc-source-extent.mgeodb \
  artifacts/world-parks/nyc-source-extent.pmtiles
cargo run --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-parks/nyc-source-extent.pmtiles
```
