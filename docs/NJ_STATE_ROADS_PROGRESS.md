# 뉴저지주 공식 도로 원천 확장 — 2026-09-24

## 판정

기본 세계 모드에 뉴욕주 팩과 별도로 **뉴저지주 21개 카운티 도로 팩**을 추가했다. [미국 Census 2025 TIGER/Line All Roads 배포 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)의 `34`로 시작하는 파일명 21개를 [목록](../data/us_tiger_2025_nj_counties.txt)으로 고정했다. 공식 디렉터리 HTML 스냅샷의 SHA-256은 `da6d2cf6bff24f53cad8808cc19146d1433a24be7b2da5369483ebda49d186ef`다. 파일명 목록은 게시 디렉터리와의 대조 결과이며 현장 도로 완전성 판정이 아니다. [2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)에 따른 미국 정부 원천의 권리와 출처를 [manifest](../data/us_tiger_nj_state_roads.toml)에 기록했다. ODbL 원천은 사용하지 않았다.

| 단계 | 관측 결과 |
|---|---|
| 원본 | 공식 ZIP 21개, 압축 파일 합계 35,230,481바이트. 파일마다 SHA-256, `.shp` Polyline 헤더, `.dbf` 레코드 수, NAD83 `.prj` 검사. DBF 레코드 합계 184,352개 |
| 원본 수치 범위 | 카운티 Shapefile 헤더 합집합 `[-75.557455, 38.929941, -73.905957, 41.355784]`. EPSG:4269 NAD83 값으로, WGS84 datum 변환이나 독립 위치 정확도 검증 결과가 아님 |
| 정규화 | 도로 피처 177,815개 채택, 선택한 도로 종류 밖 레코드 6,537개 거절. 21개 원천 모두 한 개 이상 기여. [원천별 집계](../artifacts/world-roads/nj-state/source-counts.csv)와 [압축 거절 기록](../artifacts/world-roads/nj-state/state.rejected.json.gz) |
| GeoDB | 로컬 재생성 중간 파일 99,639,668바이트, SHA-256 `46ae3c0e91495ffb41149fb62c676a0196e9dae34920adfa1baa2ad8e5abd2f5`. Git에서 제외 |
| 지도 팩 | [뉴저지 PMTiles](../artifacts/world-roads/nj-state/state.pmtiles) 31,814,100바이트, 비어 있지 않은 타일 29,097개, 빌더의 타일별 도로 피처 수 합계 965,257개. 전체 타일 29,097개 해독, 실패 0개. SHA-256 `a3d29e8d8d5dad5fef5e954a0cd61075822ab13b0c8d0425bbe14837aaaf2d30` |
| 화면 | [Newark z14.4](../artifacts/world-roads/nj-state/newark-world-z14.png) 기본 세계 모드 Mac Metal 캡처, 타일 실패 0개. 첫 요청 lookup 80.927ms. 단일 캡처라 반복 성능 수치가 아님 |

같은 Newark 위치에서 비동기 이동 40프레임의 기본 목록 단일 실행은 초기 로딩 84.083ms, 프레임 중앙값 2.858ms, p95 3.248ms, p99 18.962ms, 미해결·실패 0이었다. 뉴저지 팩만 목록에 둔 단일 비교 실행은 초기 로딩 88.373ms, 프레임 중앙값 5.508ms, p95 7.293ms, p99 68.997ms, 미해결·실패 0이었다. 반복 측정이 아니고 실행 간 변동이 커서 다른 팩의 로딩 비용이나 최적화 효과를 판정하지 않는다.

뉴욕주와 뉴저지주 원천의 직사각형 범위가 겹칠 수 있다. 현재 렌더러는 같은 타일의 레이어를 합치지만 주 경계 도로 중복 식별·연결성 검증은 아직 하지 않는다. 뉴저지의 건물·상세 수면·공원·역·공공기관·주소는 이번 팩에 없다. 원본의 도로 누락, 도형의 위치 정확도, WGS84 datum 차이, iPhone 성능도 아직 검증하지 않았다. 미국 전체나 세계 상세 지도가 완성된 결과가 아니다.

## 재현

다운로드는 Rust 빌드 도구에서만 하며 앱 실행은 로컬 PMTiles만 읽는다. ZIP과 GeoDB는 Git에서 제외하고 [파일명 목록](../data/us_tiger_2025_nj_counties.txt), [해시·권리 manifest](../data/us_tiger_nj_state_roads.toml), 생성 절차와 지도 팩을 남긴다.

```bash
cargo run -p mappa-map-acquire -- \
  data/us_tiger_2025_nj_counties.txt data/local/nj_roads
cargo run --offline --release -p mappa-map-data --bin make_us_tiger_roads_manifest -- \
  data/us_tiger_2025_nj_counties.txt data/local/nj_roads \
  data/us_tiger_nj_state_roads.toml 2026-09-24
cargo run --offline --release -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_nj_state_roads.toml artifacts/world-roads/nj-state/state.mgeodb --gzip-rejections
cargo run --offline --release -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_nj_state_roads.toml artifacts/world-roads/nj-state/state.mgeodb \
  artifacts/world-roads/nj-state/state.pmtiles
cargo run --offline --release -p mappa-map-data --bin audit_canonical_sources -- \
  data/us_tiger_nj_state_roads.toml artifacts/world-roads/nj-state/state.mgeodb \
  artifacts/world-roads/nj-state/state.rejected.json.gz
cargo run --offline --release -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/nj-state/state.pmtiles
```
