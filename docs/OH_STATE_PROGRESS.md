# 미국 오하이오주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `39`의 ZIP을 각각 88개 찾았다. 두 목록의 카운티 번호 88개는 모두 일치한다. [도로 목록](../data/us_tiger_2025_oh_counties.txt)과 [수면 목록](../data/us_tiger_2025_oh_areawater.txt)에 공식 디렉터리 HTML SHA-256을 각각 `4762a0cde6793356b040ee84b5b77fbdbf4daa37c01ab59eec0f161302948fa4`와 `31933a1e2f7aaa62430fd3f149bfa3660802401fe548793774e436e9d0bf7cd9`로 고정했다. 176개 ZIP의 개별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_oh_state_roads.toml)와 [수면 manifest](../data/us_tiger_oh_state_areawater.toml)에 기록했다. 원본 ZIP은 빌드 입력이며 앱은 로컬 지도 팩을 읽는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 88개, 107,683,479바이트 | 88개, 30,258,074바이트 |
| 원본 DBF 레코드 | 449,775개 | 52,242개 |
| 채택 | 416,747개 | 52,216개 |
| 제외 | 33,031개: 현재 차량 도로 분류 밖 | 26개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 272,771,573바이트, SHA-256 `05bc511eed44623981009b3eac050f5a9f1f61bef4345327debbd51e0049ec37` | 로컬 59,071,579바이트, SHA-256 `4e2107740a1249f04675004ee314a29ddcd600970bb1c88c4ec5da9d4d8172ea` |
| 전체 PMTiles | 로컬 95,574,526바이트, SHA-256 `932f2114e1d8cff4b044755c5b3cc01628b4b2571ee4069f5367ad1b0080b4b8` | [수면](../artifacts/world-water/oh-state/water.pmtiles) 31,412,908바이트, SHA-256 `da4adfe06861e708f7ec9d4b342d5b52cff5922bce14f58efb3028ea7bc8eae5` |
| 배포 PMTiles | [서부](../artifacts/world-roads/oh-state/roads-west.pmtiles) 40,699,084바이트, SHA-256 `6cde94e40f5a4207c86033844898e184f72b19970e316297b02fe231e688f312`; [중부](../artifacts/world-roads/oh-state/roads-east-west.pmtiles) 24,873,838바이트, SHA-256 `3a384f70e591e8815a343d09b7a5fb7cffb81ac142744ae53bd86f191eb36421`; [동부](../artifacts/world-roads/oh-state/roads-east-east.pmtiles) 29,603,428바이트, SHA-256 `3fa832186ad3b8967485495916b00a7dba9b5339eb007864def63902c57779b7` | 한 팩으로 배포 |
| 실제 타일 전수 해독 | [전체 155,314개](../artifacts/world-roads/oh-state/full-tile-audit.log), [서부 71,955개](../artifacts/world-roads/oh-state/west-tile-audit.log), [중부 40,640개](../artifacts/world-roads/oh-state/east-west-tile-audit.log), [동부 42,719개](../artifacts/world-roads/oh-state/east-east-tile-audit.log): 각각 실패 0 | [74,546개](../artifacts/world-water/oh-state/tile-audit.log): 실패 0 |

Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-84.820305, 38.404167, -80.518714, 41.976296]`, 수면 `[-84.820204, 38.404645, -80.518789, 42.327132]`이다. [도로 제외](../artifacts/world-roads/oh-state/state.rejected.json.gz) 33,031개는 모두 제외 대상 도로/경로 분류이고, [수면 제외](../artifacts/world-water/oh-state/water.rejected.json.gz) 26개는 모두 모호한 폴리곤 링이다. 채택 피처와 원본 ID의 연결은 GeoDB에 남겼다.

도로 채택 수는 Shapefile 한 행의 여러 선 파트를 각각 `LINEARID:part_index` 피처로 만드는 어댑터의 **출력 피처 수**다. [Rust 계보 감사](../artifacts/world-roads/oh-state/lineage-audit.log)에서 채택 원본 ID는 416,744개이고 추가 선 파트가 3개여서 출력 피처가 416,747개임을 확인했다. 채택 원본 ID 416,744개와 제외 원본 레코드 33,031개의 합계는 DBF 449,775행과 일치한다. 이 감사는 각 원본 ZIP의 고정 SHA-256을 다시 확인한다.

도로 원본 PMTiles는 검증용 로컬 파일로 유지했다. Rust `shard_canonical_pmtiles`로 먼저 z10 경도 `-82.96875`에서 서부·동부로, 이어 동부를 `-81.9140625`에서 중부·동부로 나눴다. [첫 서부 manifest](../data/us_tiger_oh_state_roads_west.toml), [중부 manifest](../data/us_tiger_oh_state_roads_east_west.toml), [최종 동부 manifest](../data/us_tiger_oh_state_roads_east_east.toml)는 동일한 88개 승인 원천을 보존한다. 중간 [동부 manifest](../data/us_tiger_oh_state_roads_east.toml)와 로컬 중간 PMTiles의 SHA-256 `4855baf0c3660a46033f429937ac3f78e6e1c82ca2d8368bb7600c2cb40d4ecf`도 재현에 사용한다. **원본 실제 타일 155,314개 모두가 정확히 한 최종 팩에 있고, 각 타일의 압축 바이트가 원본과 동일**하다. 타일별 반복 포함 도로 수는 major 32,990, collector 173,144, local 2,367,542이며 최종 세 팩의 합계가 원본과 같다.

기본 세계 모드 [Toledo z14](../artifacts/world-roads/oh-state/toledo-road-water-z14.png), [Mansfield z14](../artifacts/world-roads/oh-state/mansfield-road-water-z14.png), [Cleveland z14](../artifacts/world-roads/oh-state/cleveland-road-water-z14.png)에 도시 도로가 표시됐고 수면도 각각 화면 일부에 보였다. 세 Metal 캡처의 타일 실패는 각각 0이었다. [Census 2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 이는 파일·타일·화면 검증이다. 실제 지형의 누락 없음, 카운티·주 경계 연결성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 39 2026-09-25 data/us_tiger_2025_oh_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 39 2026-09-25 data/us_tiger_2025_oh_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_oh_counties.txt data/local/oh_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_oh_areawater.txt data/local/oh_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_oh_counties.txt data/local/oh_roads data/us_tiger_oh_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_oh_areawater.txt data/local/oh_areawater data/us_tiger_oh_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_oh_state_roads.toml artifacts/world-roads/oh-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_oh_state_areawater.toml artifacts/world-water/oh-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_oh_state_roads.toml artifacts/world-roads/oh-state/state.mgeodb artifacts/world-roads/oh-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_oh_state_areawater.toml artifacts/world-water/oh-state/water.mgeodb artifacts/world-water/oh-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_oh_state_roads.toml artifacts/world-roads/oh-state/state.pmtiles artifacts/world-roads/oh-state/roads
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_oh_state_roads_east.toml artifacts/world-roads/oh-state/roads-east.pmtiles artifacts/world-roads/oh-state/roads-east
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_oh_state_roads.toml artifacts/world-roads/oh-state/state.mgeodb artifacts/world-roads/oh-state/state.rejected.json.gz
```
