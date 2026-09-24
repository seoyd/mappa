# 미국 노스캐롤라이나주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `37`의 ZIP을 각각 **100개** 찾았다. 두 목록의 카운티 번호는 전부 일치한다. [도로 목록](../data/us_tiger_2025_nc_counties.txt)과 [수면 목록](../data/us_tiger_2025_nc_areawater.txt)은 공식 디렉터리 HTML SHA-256 `955c64c82791c7682aa614c8462bb313518a42ea84c5ecc327a8fd3130ff615b`, `3b7c48618943427dd0d31657b48be6861a0e20a8db804a4bf955a8689e26e1be`를 고정한다. ZIP별 출처·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_nc_state_roads.toml)와 [수면 manifest](../data/us_tiger_nc_state_areawater.toml)에 있다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 100개, 146,648,198바이트 | 100개, 31,203,048바이트 |
| 원본 DBF 행 | 510,111개 | 41,168개 |
| 채택 | 460,586개 | 41,148개 |
| 제외 | 49,525개: 현재 차량 도로 표시 분류 밖 | 20개: 내부 링 짝이 모호함 |
| GeoDB | 로컬 338,127,114바이트, SHA-256 `f8cbbb802bfd7f3bbe767941a6d9d56a9a22f8da7c385e085e2991d84c4701bf` | 로컬 57,246,694바이트, SHA-256 `a18ff346376cf3051e54ce72619f83b95b65a5348ad63d84c913fa70414f87d8` |
| 전체 PMTiles | 로컬 115,555,950바이트, SHA-256 `1cb2e7d57059b286bdb32ebdbc49081a646c93cee05138909600d4d3e8647aa4` | [수면](../artifacts/world-water/nc-state/water.pmtiles) 33,220,831바이트, SHA-256 `04130dce4e9f8727059ec63d60b24014a2a527ed946487f47ecf97cc9d48f43d` |
| 최종 도로 팩 | [서부](../artifacts/world-roads/nc-state/roads-west-west.pmtiles) 18,466,664바이트, SHA `e2223e8082ab8b7f473e162af6c55210a60075c96c95f6836085a7120de5dc86`; [중서부](../artifacts/world-roads/nc-state/roads-west-east.pmtiles) 42,245,567바이트, SHA `89c8111a335c89fc38ac424d691b294d76659d760821f8bfb73fd4f190a2e1e2`; [중동부](../artifacts/world-roads/nc-state/roads-east-west.pmtiles) 40,186,594바이트, SHA `031408bddfbcff0e945deb2f0a1f4e8bfb8f0c8706cefab6462a8979edf98dd5`; [동부](../artifacts/world-roads/nc-state/roads-east-east.pmtiles) 14,271,257바이트, SHA `ea11eb2d4a19ff7d9e6673e2b39c37bd5198613022f4318dbcb52b47c98808d5` | 해당 없음 |
| 전체 타일 해독 | [원본 도로 152,590개](../artifacts/world-roads/nc-state/full-tile-audit.log), [서부 14,641개](../artifacts/world-roads/nc-state/west-west-tile-audit.log), [중서부 46,420개](../artifacts/world-roads/nc-state/west-east-tile-audit.log), [중동부 59,857개](../artifacts/world-roads/nc-state/east-west-tile-audit.log), [동부 31,672개](../artifacts/world-roads/nc-state/east-east-tile-audit.log): 모두 부재·실패 0 | [수면 70,852개](../artifacts/world-water/nc-state/tile-audit.log): 부재·실패 0 |

각 ZIP의 Shapefile 구조·DBF 행 수·NAD83 `.prj`·SHA-256을 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-84.32151, 33.844418, -75.461684, 36.587746]`, 수면 `[-84.298109, 33.752878, -75.400119, 36.580437]`이다. [도로 제외 기록](../artifacts/world-roads/nc-state/state.rejected.json.gz) 49,525개 중 원천 분류 `S1740`이 31,439개, `S1750`이 12,659개, `S1500`이 3,630개였다. [Census 기술 문서의 MTFCC 정의](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수면 제외 기록](../artifacts/world-water/nc-state/water.rejected.json.gz) 20개는 모두 모호한 폴리곤 링이다. [Rust 도로 계보 감사](../artifacts/world-roads/nc-state/lineage-audit.log)에서 **510,111개 원본 행이 채택 460,586개와 제외 49,525개로 모두 설명**됐다.

전체 도로 PMTiles는 검증용 로컬 파일이다. Rust `shard_canonical_pmtiles`로 먼저 z10 경도 `-79.8046875`, 이어 서부 `-82.265625`와 동부 `-77.6953125`에서 분할했다. [1차](../artifacts/world-roads/nc-state/shard-audit.log)·[서부](../artifacts/world-roads/nc-state/west-shard-audit.log)·[동부](../artifacts/world-roads/nc-state/east-shard-audit.log) 감사에서 **원본의 실제 타일 152,590개 모두가 정확히 한 최종 팩에 있고 압축 타일 바이트가 동일**했다. 중간 PMTiles는 로컬에 유지한다. 최종 네 [출처 manifest](../data/us_tiger_nc_state_roads_west_west.toml)는 동일한 100개 승인 원천의 각 공간 경계를 보존한다.

기본 세계 모드의 [Asheville](../artifacts/world-roads/nc-state/asheville-road-water-z14.png), [Charlotte](../artifacts/world-roads/nc-state/charlotte-road-water-z14.png), [Raleigh](../artifacts/world-roads/nc-state/raleigh-road-water-z14.png), [Wilmington](../artifacts/world-roads/nc-state/wilmington-road-water-z14.png) z14 Metal 캡처에서 도로가 표시되고 타일 실패는 각각 0개였다. Wilmington 화면의 해안 쪽 넓은 흰 영역은 **해양면 고배율 데이터가 아직 없다는 표시 한계**다. 파일·타일·화면 검증은 실제 지형의 누락 없음, 카운티·주 경계 도로 연결성, NAD83↔WGS84 독립 위치 정확도, 건물·역·공공기관, iPhone 성능을 입증하지 않는다.

[VA–NC 경계 중복 감사](../artifacts/world-roads/nc-state/va-overlap-audit.log)에서 두 주의 공통 비어 있지 않은 타일 777개와 완전히 동일한 도로 좌표열 108개를 관측했다. 세계 모드의 합성기는 같은 분류·같은 좌표열의 도로를 역방향 순서까지 비교해 한 번만 그리도록 수정했다. [첫 중복 타일에서 유도한 경계 z11 화면](../artifacts/world-roads/nc-state/va-nc-border-world-z11.png)은 타일 실패 0이었고, 변경 뒤 Raleigh PNG는 변경 전과 바이트 단위로 같았다. 이 중복 제거는 부분 중첩 선형이나 실제 도로 연결성을 검증·수정하지 않는다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 37 2026-09-25 data/us_tiger_2025_nc_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 37 2026-09-25 data/us_tiger_2025_nc_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_nc_counties.txt data/local/nc_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_nc_areawater.txt data/local/nc_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_nc_counties.txt data/local/nc_roads data/us_tiger_nc_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_nc_areawater.txt data/local/nc_areawater data/us_tiger_nc_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_nc_state_roads.toml artifacts/world-roads/nc-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_nc_state_areawater.toml artifacts/world-water/nc-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_nc_state_roads.toml artifacts/world-roads/nc-state/state.mgeodb artifacts/world-roads/nc-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_nc_state_areawater.toml artifacts/world-water/nc-state/water.mgeodb artifacts/world-water/nc-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_nc_state_roads.toml artifacts/world-roads/nc-state/state.pmtiles artifacts/world-roads/nc-state/roads
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_nc_state_roads_west.toml artifacts/world-roads/nc-state/roads-west.pmtiles artifacts/world-roads/nc-state/roads-west
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_nc_state_roads_east.toml artifacts/world-roads/nc-state/roads-east.pmtiles artifacts/world-roads/nc-state/roads-east
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_nc_state_roads.toml artifacts/world-roads/nc-state/state.mgeodb artifacts/world-roads/nc-state/state.rejected.json.gz
```
