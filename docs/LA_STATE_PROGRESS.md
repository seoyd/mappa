# Louisiana 2025 TIGER/Line 도로·수역 실증 — 2026-09-25

## 원천과 계보

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수역 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 Louisiana 주 FIPS `22`의 ZIP을 각각 **64개** 확인했다. [도로 목록](../data/us_tiger_2025_la_counties.txt)과 [수역 목록](../data/us_tiger_2025_la_areawater.txt)의 교구 번호는 일치한다. 공식 디렉터리 HTML SHA-256은 각각 `19e0d611f3c1e03a570e214e5d1bc8d61328d743d45f0df2b773b7c389c8dcbd`, `31826f2076aa11b0626ac7637ac7deb1435f7a07085461063ac5663ad23505fc`다. ZIP별 공식 URL·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_la_state_roads.toml)와 [수역 manifest](../data/us_tiger_la_state_areawater.toml)에 고정했다. 원천 ZIP 다운로드는 도로 54,191,320바이트, 수역 42,375,990바이트였다.

| 검사 | 도로 | 수역 |
| --- | --- | --- |
| 원본 DBF 행 | 262,800 | 52,011 |
| 채택·제외 | 242,570·20,230 | 51,929·82 |
| canonical GeoDB | 144,991,885바이트 | 76,184,566바이트 |
| 전체 PMTiles | 106,313개 타일·58,775,964바이트 | 79,115개 타일·46,661,421바이트 |
| 최종 팩 | [서부](../artifacts/world-roads/la-state/roads-west.pmtiles) 36,370,731바이트, [동부](../artifacts/world-roads/la-state/roads-east.pmtiles) 22,125,903바이트 | [수역](../artifacts/world-water/la-state/water.pmtiles) 46,661,421바이트 |

각 ZIP의 Shapefile·DBF·NAD83 `.prj`·SHA-256을 검사했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-94.043323, 29.103662, -89.351357, 33.01962]`, 수역 `[-94.043199, 28.855127, -88.758388, 33.01937]`이다. [도로 계보 감사](../artifacts/world-roads/la-state/lineage-audit.log)는 원본 262,800행이 채택 242,570개와 [제외 기록](../artifacts/world-roads/la-state/state.rejected.json.gz) 20,230개로 설명된다고 확인했다. 주요 제외 분류는 `S1740` 13,980개, `S1750` 3,738개, `S1500` 2,041개다. 분류 뜻은 [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수역 제외 기록](../artifacts/world-water/la-state/water.rejected.json.gz) 82개는 모호한 폴리곤 링이다.

## 타일 연결과 표시

[전체 도로 감사](../artifacts/world-roads/la-state/full-tile-audit.log)는 z10–z15 **106,313개 타일**에서 타일 내부 도로 경계 미일치 **0개**를 보고한다. 네 타일 꼭짓점에서 8 MVT 단위 이내인 **1건**은 방향이 모호해 별도 보고하며 연결 완료로 단정하지 않는다. [분할 감사](../artifacts/world-roads/la-state/shard-audit.log)는 전체 도로 타일이 서부 70,657개와 동부 35,656개에 한 번씩 들어가며 압축 바이트가 같음을 확인했다. [서부](../artifacts/world-roads/la-state/west-tile-audit.log)·[동부](../artifacts/world-roads/la-state/east-tile-audit.log)·[수역](../artifacts/world-water/la-state/full-tile-audit.log) 최종 팩의 전수 해독 결과는 각각 70,657·35,656·79,115개이고 부재·해독 실패는 모두 0개다. 최종 팩 SHA-256은 도로 서부 `72d6ede88c06ae35180624f3ebb0b92649e4fff34ceed38e8be074b6dc1b656b`, 동부 `788a90e4730ccbb3e32ff6e83d0c3769852c6405d917542adc1cce4097cf7c83`, 수역 `197db1e1eda6de15c9b5d60ffa9fcd03e602778b9f22ca5dd365790475b4f89d`이다.

기본 세계 모드의 [Baton Rouge](../artifacts/world-roads/la-state/baton-rouge-road-water-z14.png)·[New Orleans](../artifacts/world-roads/la-state/new-orleans-road-water-z14.png)·[Lafayette](../artifacts/world-roads/la-state/lafayette-road-water-z14.png) z14 화면과 [LA–MS 경계](../artifacts/world-roads/la-state/la-ms-border-world-z11.png) z11 화면의 타일 실패는 각각 0개였다. [MS](../artifacts/world-roads/la-state/ms-overlap-audit.log)와 공통 비어 있지 않은 타일은 559개, 완전히 동일한 도로 좌표열은 183개였다. 부분 중복·주 경계 실제 연결성·원천 완전성·NAD83↔WGS84 독립 위치 정확도·건물·역·공공기관·iPhone 성능은 아직 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 22 2026-09-25 data/us_tiger_2025_la_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 22 2026-09-25 data/us_tiger_2025_la_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_la_counties.txt data/local/la_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_la_areawater.txt data/local/la_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_la_counties.txt data/local/la_roads data/us_tiger_la_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_la_areawater.txt data/local/la_areawater data/us_tiger_la_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_la_state_roads.toml artifacts/world-roads/la-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_la_state_areawater.toml artifacts/world-water/la-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_la_state_roads.toml artifacts/world-roads/la-state/state.mgeodb artifacts/world-roads/la-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_la_state_areawater.toml artifacts/world-water/la-state/water.mgeodb artifacts/world-water/la-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- data/us_tiger_la_state_roads.toml artifacts/world-roads/la-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_la_state_roads.toml artifacts/world-roads/la-state/state.pmtiles artifacts/world-roads/la-state/roads
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_la_state_roads.toml artifacts/world-roads/la-state/state.mgeodb artifacts/world-roads/la-state/state.rejected.json.gz
```
