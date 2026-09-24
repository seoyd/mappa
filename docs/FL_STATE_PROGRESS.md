# Florida 2025 TIGER/Line 도로·수역 실증 — 2026-09-25

## 원천과 계보

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수역 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 Florida 주 FIPS `12`의 ZIP을 각각 **67개** 확인했다. [도로 목록](../data/us_tiger_2025_fl_counties.txt)과 [수역 목록](../data/us_tiger_2025_fl_areawater.txt)의 카운티 번호는 일치한다. 공식 디렉터리 HTML SHA-256은 각각 `5c92a15a9e28ac03e4a8b2b5dc393d885339f052a5f8deb3fcbd1589d1fe07e4`, `a231fce830a71305ed5a9ff5b32774817f90fcc28a63ff424dcca883a861e163`이다. ZIP별 공식 URL·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_fl_state_roads.toml)와 [수역 manifest](../data/us_tiger_fl_state_areawater.toml)에 고정했다. 원천 ZIP 다운로드는 도로 118,674,104바이트, 수역 62,115,409바이트였다.

| 검사 | 도로 | 수역 |
| --- | --- | --- |
| 원본 DBF 행 | 684,796 | 93,700 |
| 채택·제외 | 647,644·37,152 | 93,646·54 |
| canonical GeoDB | 350,895,365바이트 | 118,098,052바이트 |
| 전체 PMTiles | 132,265개 타일·110,963,319바이트 | 114,110개 타일·63,554,756바이트 |
| 최종 팩 | [서부](../artifacts/world-roads/fl-state/roads-west.pmtiles) 17,773,620바이트, [중서부](../artifacts/world-roads/fl-state/roads-east-west.pmtiles) 38,704,278바이트, [중동부](../artifacts/world-roads/fl-state/roads-east-east-west.pmtiles) 35,273,123바이트, [동부](../artifacts/world-roads/fl-state/roads-east-east-east.pmtiles) 18,888,005바이트 | [서부](../artifacts/world-water/fl-state/water-west.pmtiles) 9,960,815바이트, [중부](../artifacts/world-water/fl-state/water-east-west.pmtiles) 22,169,681바이트, [동부](../artifacts/world-water/fl-state/water-east-east.pmtiles) 31,212,491바이트 |

각 ZIP의 Shapefile·DBF·NAD83 `.prj`·SHA-256을 검사했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-87.63196, 24.54418, -80.032124, 31.000934]`, 수역 `[-87.634896, 24.396308, -79.974306, 31.000896]`이다. [도로 계보 감사](../artifacts/world-roads/fl-state/lineage-audit.log)는 원본 684,796행이 채택 647,644개와 [제외 기록](../artifacts/world-roads/fl-state/state.rejected.json.gz) 37,152개로 설명된다고 확인했다. 주요 제외 분류는 `S1740` 15,048개, `S1500` 11,867개, `S1750` 5,527개, `S1780` 3,335개다. 분류 뜻은 [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수역 제외 기록](../artifacts/world-water/fl-state/water.rejected.json.gz) 54개는 모호한 폴리곤 링이다.

## 타일 연결과 표시

[전체 도로 감사](../artifacts/world-roads/fl-state/full-tile-audit.log)는 z10–z15 **132,265개 타일**에서 확정 가능한 타일 내부 도로 경계 미일치 **0개**를 보고한다. 네 타일 꼭짓점 근처 **7건**과 정수화된 얕은 선분의 추정 교차점 **1건**은 별도 모호 사례로 보고하며 실제 연결 완료로 단정하지 않는다. [도로 1차](../artifacts/world-roads/fl-state/shard-audit.log)·[2차](../artifacts/world-roads/fl-state/east-shard-audit.log)·[3차](../artifacts/world-roads/fl-state/east-east-shard-audit.log), [수역 1차](../artifacts/world-water/fl-state/shard-audit.log)·[2차](../artifacts/world-water/fl-state/east-shard-audit.log) 분할 감사는 모든 원본 타일이 최종 도로 네 팩과 수역 세 팩에 한 번씩 들어가며 압축 바이트가 같음을 확인했다. 각 최종 팩은 50MiB 미만이다.

[도로 서부](../artifacts/world-roads/fl-state/west-tile-audit.log)·[중서부](../artifacts/world-roads/fl-state/east-west-tile-audit.log)·[중동부](../artifacts/world-roads/fl-state/east-east-west-tile-audit.log)·[동부](../artifacts/world-roads/fl-state/east-east-east-tile-audit.log), [수역 서부](../artifacts/world-water/fl-state/west-tile-audit.log)·[중부](../artifacts/world-water/fl-state/east-west-tile-audit.log)·[동부](../artifacts/world-water/fl-state/east-east-tile-audit.log) 최종 팩의 전수 해독 결과는 각각 도로 30,614·47,355·39,793·14,503개, 수역 24,057·37,611·52,442개이며 부재·해독 실패는 모두 0개다. 최종 팩 SHA-256은 도로 서부 `b542d83cfc448ed80ec7ef6dc8a1e5a8f38397aea2c648b2c9964447826fbc76`, 중서부 `6af3d8e18f75850c93d6a06feabc313a389e653b1a5ab8c40130926a97728092`, 중동부 `69278672b7ed2beaa1ac79f0ed638dcb28d1eec716072c16bf1d42d7aa916db2`, 동부 `e4b15abcc1fa07a13395194af779719f22530767084c197bf1359e1b599a4c55`, 수역 서부 `b9f338ddc37f0e041955a96a606f284593fcb29e9d90b65abecafc48064ff126`, 중부 `9af28a4d9ed2a511cebe812e06c05978c6e54b8ba5a4ad507fd605434e22e09a`, 동부 `2f64de4a21a7396cc684a16d519986d377458dce36b397deeae01b4978eb44e8`이다.

기본 세계 모드의 [Miami](../artifacts/world-roads/fl-state/miami-road-water-z14.png)·[Orlando](../artifacts/world-roads/fl-state/orlando-road-water-z14.png)·[Tampa](../artifacts/world-roads/fl-state/tampa-road-water-z14.png)·[Jacksonville](../artifacts/world-roads/fl-state/jacksonville-road-water-z14.png) z14 화면과 [GA–FL 경계](../artifacts/world-roads/fl-state/ga-fl-border-world-z11.png) z11 화면의 타일 실패는 각각 0개였다. [GA](../artifacts/world-roads/fl-state/ga-overlap-audit.log)·[AL](../artifacts/world-roads/fl-state/al-overlap-audit.log)과 공통 비어 있지 않은 타일은 498·437개, 완전히 동일한 도로 좌표열은 116·320개였다. 부분 중복·주 경계 실제 연결성·원천 완전성·NAD83↔WGS84 독립 위치 정확도·건물·역·공공기관·iPhone 성능은 아직 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 12 2026-09-25 data/us_tiger_2025_fl_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 12 2026-09-25 data/us_tiger_2025_fl_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_fl_counties.txt data/local/fl_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_fl_areawater.txt data/local/fl_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_fl_counties.txt data/local/fl_roads data/us_tiger_fl_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_fl_areawater.txt data/local/fl_areawater data/us_tiger_fl_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_fl_state_roads.toml artifacts/world-roads/fl-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_fl_state_areawater.toml artifacts/world-water/fl-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_fl_state_roads.toml artifacts/world-roads/fl-state/state.mgeodb artifacts/world-roads/fl-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_fl_state_areawater.toml artifacts/world-water/fl-state/water.mgeodb artifacts/world-water/fl-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- data/us_tiger_fl_state_roads.toml artifacts/world-roads/fl-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_fl_state_roads.toml artifacts/world-roads/fl-state/state.pmtiles artifacts/world-roads/fl-state/roads
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_fl_state_roads_east.toml artifacts/world-roads/fl-state/roads-east.pmtiles artifacts/world-roads/fl-state/roads-east
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_fl_state_roads_east_east.toml artifacts/world-roads/fl-state/roads-east-east.pmtiles artifacts/world-roads/fl-state/roads-east-east
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_fl_state_areawater.toml artifacts/world-water/fl-state/water.pmtiles artifacts/world-water/fl-state/water
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_fl_state_areawater_east.toml artifacts/world-water/fl-state/water-east.pmtiles artifacts/world-water/fl-state/water-east
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_fl_state_roads.toml artifacts/world-roads/fl-state/state.mgeodb artifacts/world-roads/fl-state/state.rejected.json.gz
```
