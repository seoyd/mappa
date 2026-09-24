# 뉴욕주 공식 도로 원천 확장 — 2026-09-24

## 판정

기본 세계 모드의 뉴욕시 5개 카운티 도로 팩을 **뉴욕주 62개 카운티 도로 팩**으로 교체했다. [미국 Census 2025 TIGER/Line All Roads 배포 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)에서 해당 62개 원본을 확인하고 [파일 목록](../data/us_tiger_2025_ny_counties.txt)을 고정했다. [2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건에 따라 각 파일의 권리·출처를 [62원천 manifest](../data/us_tiger_ny_state_roads.toml)에 기록했다. ODbL 원천은 사용하지 않았다.

| 단계 | 관측 결과 |
|---|---|
| 원본 | 62개 ZIP, 로컬 압축 저장량 약 82 MiB. 모든 파일의 SHA-256·`.shp` Polyline 헤더·`.dbf` 레코드 수·NAD83 `.prj`를 Rust 도구로 검사. 원본 DBF 레코드 합계 379,647개 |
| 원본 범위 | 카운티 Shapefile 헤더 합집합 `[-79.761979, 40.497866, -71.857317, 45.011597]`. EPSG:4269의 숫자 범위로, 별도 WGS84 datum/현장 정확도 검증 결과가 아님 |
| GeoDB | 도로 피처 340,381개 채택, 도보·자전거 등 선택 규칙 밖 레코드 39,266개 거절. 62개 원천이 모두 한 개 이상 기여함. 로컬 재생성 GeoDB 206,090,324바이트, SHA-256 `8b577022a874818a6eab15232af07743d10f51e3f586c68e5b02a231c8291000`. [원천별 채택·거절 CSV](../artifacts/world-roads/ny-state/source-counts.csv), [압축 거절 기록](../artifacts/world-roads/ny-state/state.rejected.json.gz) |
| 지도 팩 | [뉴욕주 PMTiles](../artifacts/world-roads/ny-state/state.pmtiles) 89,918,995바이트, 비어 있지 않은 타일 171,167개, 빌더가 기록한 타일별 도로 피처 수 합계 2,107,853개. 전체 타일 해독 171,167개, 실패 0개. SHA-256 `ea307218ac1d001195081ab0e793bec68fdd4525734f788ea7c407b92b7ae73d` |
| 기존 뉴욕시 보존 | 기존 뉴욕시 팩의 비어 있지 않은 1,522개 타일 모두 새 팩에 존재. 1,416개는 바이트 동일, 106개는 달라졌으나 이전 도로 선형이 빠진 타일은 0개. 기존 5개 원천의 채택 피처 합계 20,882개도 이전 결과와 같음 |
| 화면 | [Buffalo z14.4](../artifacts/world-roads/ny-state/buffalo-world-z14.png)와 [Queens 네 레이어 z14.4](../artifacts/world-roads/ny-state/queens-four-layers-world.png)를 기본 세계 모드의 Mac Metal에서 캡처. 각각 타일 실패 0개 |

새 팩은 도로만 담는다. 뉴욕시 밖의 상세 건물·수면·공원·역·공공기관·주소는 추가되지 않았다. 카운티 원본의 공간적 완전성, 연결성, 같은 도로의 경계 중복, WGS84 datum 차이, 독립 기준점 정확도, iPhone 성능은 아직 검증되지 않았다. 팩의 직사각형 범위는 인접 주·캐나다 일부를 포함할 수 있지만, 실제 도로 원본이 없는 공간은 그리지 않는다. 이 결과는 미국 전체나 전 세계 도로 완성이 아니다.

대용량 원본 ZIP과 GeoDB는 Git에서 제외하고 원본 URL·해시·생성 절차를 남긴다. PMTiles는 현재 저장소에 포함한다. 전 세계 상세 팩을 저장소에 계속 추가할 때의 배포 용량·갱신 방식은 아직 확정되지 않았다.

## 재현

Rust 빌드 전용 다운로드 도구가 공식 Census 디렉터리에서 [고정 파일 목록](../data/us_tiger_2025_ny_counties.txt)의 62개 ZIP을 `data/local/ny_roads/`에 원래 파일명으로 받는다. 주소는 빌드 도구에만 있으며 앱 실행에는 네트워크가 필요하지 않다. `make_us_tiger_roads_manifest`가 다운로드 파일의 해시·헤더·좌표계를 재검증한다.

```bash
cargo run -p mappa-map-acquire -- \
  data/us_tiger_2025_ny_counties.txt data/local/ny_roads
cargo run --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_ny_counties.txt data/local/ny_roads \
  data/us_tiger_ny_state_roads.toml 2026-09-24
cargo run --offline --release -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_ny_state_roads.toml artifacts/world-roads/ny-state/state.mgeodb --gzip-rejections
cargo run --offline --release -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_ny_state_roads.toml artifacts/world-roads/ny-state/state.mgeodb \
  artifacts/world-roads/ny-state/state.pmtiles
cargo run --offline -p mappa-map-data --bin audit_canonical_sources -- \
  data/us_tiger_ny_state_roads.toml artifacts/world-roads/ny-state/state.mgeodb \
  artifacts/world-roads/ny-state/state.rejected.json.gz
cargo run --offline --release -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/ny-state/state.pmtiles
cargo run --offline --release -p mappa-map-data --bin compare_pmtiles_coverage -- \
  artifacts/world-roads/nyc-five-boroughs.pmtiles \
  artifacts/world-roads/ny-state/state.pmtiles
```
