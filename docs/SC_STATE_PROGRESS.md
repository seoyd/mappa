# 미국 사우스캐롤라이나주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `45`의 ZIP을 각각 **46개** 찾았다. 두 목록의 카운티 번호는 전부 일치한다. [도로 목록](../data/us_tiger_2025_sc_counties.txt)과 [수면 목록](../data/us_tiger_2025_sc_areawater.txt)은 공식 디렉터리 HTML SHA-256 `f5cb489037d736dee3bfb87236ff2299e422a85540d2e738cc2212f7cb8de917`, `3f86b2945a539ddfc0df15650552f92cda9b38712b673d9d2a36677866ed3585`를 고정한다. ZIP별 출처·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_sc_state_roads.toml)와 [수면 manifest](../data/us_tiger_sc_state_areawater.toml)에 있다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 46개, 79,372,829바이트 | 46개, 13,911,741바이트 |
| 원본 DBF 행 | 284,287개 | 14,883개 |
| 채택 | 265,485개 | 14,875개 |
| 제외 | 18,802개: 현재 차량 도로 표시 분류 밖 | 8개: 내부 링 짝이 모호함 |
| GeoDB | 로컬 188,998,525바이트, SHA-256 `f679f0de0ca0a0632006b0767f08ee105ddd392bac247971e86c4b6ed571ba3e` | 로컬 24,286,378바이트, SHA-256 `d67c5416fe1ebb98a9df4e51da05e666696a85dfcbdc5bfb31f733f74739459e` |
| 전체 PMTiles | 로컬 68,130,067바이트, SHA-256 `ffa32adeeb50df08e26212f4f21c06b085dc2e378220eaae9708f15cc0dc7d73` | [수면](../artifacts/world-water/sc-state/water.pmtiles) 17,987,173바이트, SHA-256 `e061498cfa42fc2dd887397af1d58ab20ea11ff9e02deed21c5902fbb705860c` |
| 최종 도로 팩 | [서부](../artifacts/world-roads/sc-state/roads-west.pmtiles) 28,472,530바이트, SHA `2a239edb1bd187ee92acd5812ea338f96625fa68f35f4ee6aebd5c3629d7fc0d`; [동부](../artifacts/world-roads/sc-state/roads-east.pmtiles) 39,414,324바이트, SHA `aa7ca8913473dace8f0bd5feaa6b609fea56454d079734a2db1e1111faf056c0` | 해당 없음 |
| 전체 타일 해독 | [원본 도로 93,006개](../artifacts/world-roads/sc-state/full-tile-audit.log), [서부 36,981개](../artifacts/world-roads/sc-state/west-tile-audit.log), [동부 56,025개](../artifacts/world-roads/sc-state/east-tile-audit.log): 부재·실패 0 | [수면 38,194개](../artifacts/world-water/sc-state/tile-audit.log): 부재·실패 0 |

각 ZIP의 Shapefile 구조·DBF 행 수·NAD83 `.prj`·SHA-256을 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-83.346361, 32.082652, -78.575567, 35.215226]`, 수면 `[-83.353928, 31.995954, -78.499301, 35.196231]`이다. [도로 제외 기록](../artifacts/world-roads/sc-state/state.rejected.json.gz) 18,802개 중 원천 분류 `S1740`이 13,730개, `S1750`이 2,316개, `S1500`이 1,623개다. [Census 기술 문서의 MTFCC 정의](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수면 제외 기록](../artifacts/world-water/sc-state/water.rejected.json.gz) 8개는 모두 모호한 폴리곤 링이다. [Rust 도로 계보 감사](../artifacts/world-roads/sc-state/lineage-audit.log)에서 원본 **284,287행이 채택 265,485개와 제외 18,802개로 모두 설명**됐다.

도로 원본 PMTiles는 검증용 로컬 파일이다. Rust `shard_canonical_pmtiles`로 z10 경도 `-81.2109375`에서 나눴다. [분할 감사](../artifacts/world-roads/sc-state/shard-audit.log)에서 원본 93,006개 실제 타일이 정확히 한 최종 팩에 있고 압축 바이트가 동일했다. 최종 [서부 manifest](../data/us_tiger_sc_state_roads_west.toml)와 [동부 manifest](../data/us_tiger_sc_state_roads_east.toml)는 동일한 46개 승인 원천과 각 공간 경계를 보존한다.

기본 세계 모드의 [Greenville](../artifacts/world-roads/sc-state/greenville-road-water-z14.png), [Charleston](../artifacts/world-roads/sc-state/charleston-road-water-z14.png) z14 Metal 화면에서 도로·수면이 보이고 타일 실패는 각각 0개였다. [NC–SC 경계 중복 감사](../artifacts/world-roads/sc-state/nc-overlap-audit.log)는 공통 비어 있지 않은 타일 848개, 완전히 동일한 도로 좌표열 427개를 관측했다. 세계 모드의 좌표열 일치 도로 중복 표시 제거가 이 구간에도 적용된다. [첫 중복 타일에서 유도한 경계 z11 화면](../artifacts/world-roads/sc-state/nc-sc-border-world-z11.png)의 타일 실패는 0개다. 부분 중복과 실제 도로 연결성, NAD83↔WGS84 독립 위치 정확도, 원본 완전성, 건물·역·공공기관, iPhone 성능은 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 45 2026-09-25 data/us_tiger_2025_sc_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 45 2026-09-25 data/us_tiger_2025_sc_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_sc_counties.txt data/local/sc_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_sc_areawater.txt data/local/sc_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_sc_counties.txt data/local/sc_roads data/us_tiger_sc_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_sc_areawater.txt data/local/sc_areawater data/us_tiger_sc_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_sc_state_roads.toml artifacts/world-roads/sc-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_sc_state_areawater.toml artifacts/world-water/sc-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_sc_state_roads.toml artifacts/world-roads/sc-state/state.mgeodb artifacts/world-roads/sc-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_sc_state_areawater.toml artifacts/world-water/sc-state/water.mgeodb artifacts/world-water/sc-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_sc_state_roads.toml artifacts/world-roads/sc-state/state.pmtiles artifacts/world-roads/sc-state/roads
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_sc_state_roads.toml artifacts/world-roads/sc-state/state.mgeodb artifacts/world-roads/sc-state/state.rejected.json.gz
```
