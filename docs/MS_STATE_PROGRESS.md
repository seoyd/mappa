# Mississippi 2025 TIGER/Line 도로·수역 실증 — 2026-09-25

## 원천과 계보

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수역 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 Mississippi 주 FIPS `28`의 ZIP을 각각 **82개** 확인했다. [도로 목록](../data/us_tiger_2025_ms_counties.txt)과 [수역 목록](../data/us_tiger_2025_ms_areawater.txt)의 카운티 번호는 일치한다. 공식 디렉터리 HTML SHA-256은 각각 `feb919f89d1706ea6f7cc11d7b5520727cae3f4ca537cbd75916e3c3846dfba3`, `a646387eefbfcfbd77fb74efac47bdd8d0931996cb358484ae3dec6ebe6f5fd6`다. ZIP별 공식 URL·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_ms_state_roads.toml)와 [수역 manifest](../data/us_tiger_ms_state_areawater.toml)에 고정했다. 원천 ZIP 다운로드는 도로 65,837,126바이트, 수역 14,472,920바이트였다.

| 검사 | 도로 | 수역 |
| --- | --- | --- |
| 원본 DBF 행 | 275,018 | 20,778 |
| 채택·제외 | 240,758·34,260 | 20,770·8 |
| canonical GeoDB | 159,398,512바이트 | 26,338,570바이트 |
| 전체 PMTiles | 136,057개 타일·74,262,791바이트 | 43,194개 타일·18,177,066바이트 |
| 최종 팩 | [서부](../artifacts/world-roads/ms-state/roads-west.pmtiles) 25,264,108바이트, [동부](../artifacts/world-roads/ms-state/roads-east.pmtiles) 48,637,082바이트 | [수역](../artifacts/world-water/ms-state/water.pmtiles) 18,177,066바이트 |

각 ZIP의 Shapefile·DBF·NAD83 `.prj`·SHA-256을 검사했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-91.60938, 30.20471, -88.099916, 34.996099]`, 수역 `[-91.655009, 30.139845, -88.097888, 34.995694]`이다. [도로 계보 감사](../artifacts/world-roads/ms-state/lineage-audit.log)는 원본 275,018행이 채택 240,758개와 [제외 기록](../artifacts/world-roads/ms-state/state.rejected.json.gz) 34,260개로 설명된다고 확인했다. 주요 제외 분류는 `S1750` 17,203개, `S1740` 14,796개, `S1500` 1,524개, `S1780` 599개다. 분류 뜻은 [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수역 제외 기록](../artifacts/world-water/ms-state/water.rejected.json.gz) 8개는 모호한 폴리곤 링이다.

## 타일 연결과 표시

[전체 도로 감사](../artifacts/world-roads/ms-state/full-tile-audit.log)는 z10–z15 **136,057개 타일**에서 타일 내부 도로 경계 미일치 **0개**를 보고한다. 네 타일 꼭짓점에서 8 MVT 단위 이내인 **6건**은 방향이 모호해 별도 보고하며 연결 완료로 단정하지 않는다. 이 중 z15/8277/13246 아래쪽의 1건은 얕은 각도로 꼭짓점을 지나는 선을 정수화할 때 생긴 교차점으로 관측됐고, 이전 3단위 기준에서는 미일치로 판정됐다. 이 사례 때문에 감사 기준을 8단위 모호 영역으로 확장했으며, [Georgia](GA_STATE_PROGRESS.md)·[Alabama](AL_STATE_PROGRESS.md)를 같은 기준으로 재감사했을 때 두 지역의 기존 모호 건수 7·5개와 내부 미일치 0개는 변하지 않았다. 이 변경은 해당 지점의 실제 도로 연결을 증명하지 않는다.

[분할 감사](../artifacts/world-roads/ms-state/shard-audit.log)는 전체 도로 타일이 서부 48,837개와 동부 87,220개에 한 번씩 들어가며 압축 바이트가 같음을 확인했다. [서부](../artifacts/world-roads/ms-state/west-tile-audit.log)·[동부](../artifacts/world-roads/ms-state/east-tile-audit.log)·[수역](../artifacts/world-water/ms-state/full-tile-audit.log) 최종 팩의 전수 해독 결과는 각각 48,837·87,220·43,194개이고 부재·해독 실패는 모두 0개다. 최종 팩 SHA-256은 도로 서부 `41fe26ec5723b1bd5be054e2450ce551306401b594198141825bd713e2775e6a`, 동부 `7b787d8aba89e58fff53137d1e0591cccf221a79bc7c7a1fc93d1ab54cbd21c2`, 수역 `b1f39b1c6e31e08da0be2eb004cc3af78586d0e28c7d73551a756384360de208`이다.

기본 세계 모드의 [Jackson](../artifacts/world-roads/ms-state/jackson-road-water-z14.png)·[Gulfport](../artifacts/world-roads/ms-state/gulfport-road-water-z14.png)·[Tupelo](../artifacts/world-roads/ms-state/tupelo-road-water-z14.png) z14 화면과 [MS–AL 경계](../artifacts/world-roads/ms-state/ms-al-border-world-z11.png) z11 화면의 타일 실패는 각각 0개였다. [AL](../artifacts/world-roads/ms-state/al-overlap-audit.log)·[TN](../artifacts/world-roads/ms-state/tn-overlap-audit.log)과 공통 비어 있지 않은 타일은 649·252개, 완전히 동일한 도로 좌표열은 213·26개였다. 부분 중복·주 경계 실제 연결성·원천 완전성·NAD83↔WGS84 독립 위치 정확도·건물·역·공공기관·iPhone 성능은 아직 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 28 2026-09-25 data/us_tiger_2025_ms_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 28 2026-09-25 data/us_tiger_2025_ms_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_ms_counties.txt data/local/ms_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_ms_areawater.txt data/local/ms_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ms_counties.txt data/local/ms_roads data/us_tiger_ms_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ms_areawater.txt data/local/ms_areawater data/us_tiger_ms_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ms_state_roads.toml artifacts/world-roads/ms-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ms_state_areawater.toml artifacts/world-water/ms-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ms_state_roads.toml artifacts/world-roads/ms-state/state.mgeodb artifacts/world-roads/ms-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ms_state_areawater.toml artifacts/world-water/ms-state/water.mgeodb artifacts/world-water/ms-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- data/us_tiger_ms_state_roads.toml artifacts/world-roads/ms-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ms_state_roads.toml artifacts/world-roads/ms-state/state.pmtiles artifacts/world-roads/ms-state/roads
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_ms_state_roads.toml artifacts/world-roads/ms-state/state.mgeodb artifacts/world-roads/ms-state/state.rejected.json.gz
```
