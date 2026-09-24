# Georgia 2025 TIGER/Line 도로·수역 실증 — 2026-09-25

## 원천과 계보

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수역 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 Georgia 주 FIPS `13`의 ZIP을 각각 **159개** 확인했다. [도로 목록](../data/us_tiger_2025_ga_counties.txt)과 [수역 목록](../data/us_tiger_2025_ga_areawater.txt)의 카운티 번호는 전부 일치한다. 공식 디렉터리 HTML SHA-256은 각각 `602c1ccfbb2f6a96ab6296a28d7c80cba823ca6097023ef69a3eae3fe025029f`, `d127d3c8cfea38d020cf0f801fd39039355203db42847ad4146c85bdf9f3ee3e`다. ZIP별 공식 URL·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_ga_state_roads.toml)와 [수역 manifest](../data/us_tiger_ga_state_areawater.toml)에 고정했다. 원천 ZIP 다운로드는 도로 107,300,466바이트, 수역 42,287,682바이트였다.

| 검사 | 도로 | 수역 |
| --- | --- | --- |
| 원본 DBF 행 | 440,929 | 88,757 |
| 채택·제외 | 412,918·28,011 | 88,722·35 |
| canonical GeoDB | 265,109,606바이트 | 84,515,863바이트 |
| 전체 PMTiles | 172,941개 타일·110,161,558바이트 | 99,703개 타일·48,920,242바이트 |
| 최종 팩 | [서부](../artifacts/world-roads/ga-state/roads-west-west.pmtiles) 17,299,642바이트, [중서부](../artifacts/world-roads/ga-state/roads-west-east-west.pmtiles) 30,066,582바이트, [중동부](../artifacts/world-roads/ga-state/roads-west-east-east.pmtiles) 24,485,675바이트, [동부](../artifacts/world-roads/ga-state/roads-east.pmtiles) 37,880,773바이트 | [수역](../artifacts/world-water/ga-state/water.pmtiles) 48,920,242바이트 |

각 ZIP의 Shapefile·DBF·NAD83 `.prj`·SHA-256을 검사했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-85.601507, 30.359312, -80.841082, 35.000335]`, 수역 `[-85.542936, 30.375238, -80.78296, 35.000659]`이다. [도로 계보 감사](../artifacts/world-roads/ga-state/lineage-audit.log)는 원본 440,929행이 채택 412,918개와 [제외 기록](../artifacts/world-roads/ga-state/state.rejected.json.gz) 28,011개로 설명된다고 확인했다. 주요 제외 분류는 `S1740` 20,746개, `S1750` 2,961개, `S1500` 2,292개, `S1780` 1,567개다. 분류 뜻은 [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수역 제외 기록](../artifacts/world-water/ga-state/water.rejected.json.gz) 35개는 모호한 폴리곤 링이다.

## 타일 연결과 표시

기존 빌더는 도로가 타일 경계를 지날 때 양쪽 버퍼 끝점을 각각 정수화해 중앙 연결점에 최대 여러 MVT 단위 차이를 남겼다. 빌더가 동일한 원본 선분에서 타일 경계 교차점을 계산해 양쪽 타일에 명시적으로 넣도록 수정했다. [전체 도로 감사](../artifacts/world-roads/ga-state/full-tile-audit.log)는 z10–z15 **172,941개 타일**에서 타일 내부를 통과하는 도로 경계 연결의 미일치 **0개**를 보고한다. 네 타일이 만나는 꼭짓점에서 3 MVT 단위 이내인 **7건**은 방향이 모호해 별도 보고하며 연결 완료로 단정하지 않는다.

[1차](../artifacts/world-roads/ga-state/shard-audit.log)·[2차](../artifacts/world-roads/ga-state/west-shard-audit.log)·[3차](../artifacts/world-roads/ga-state/west-east-shard-audit.log) 분할 감사는 전체 도로 타일 172,941개가 최종 네 팩에 한 번씩 들어가며 압축 바이트가 같음을 확인했다. [서부](../artifacts/world-roads/ga-state/west-west-tile-audit.log)·[중서부](../artifacts/world-roads/ga-state/west-east-west-tile-audit.log)·[중동부](../artifacts/world-roads/ga-state/west-east-east-tile-audit.log)·[동부](../artifacts/world-roads/ga-state/east-tile-audit.log) 최종 도로 팩의 개별 전수 해독 결과는 각각 25,654·35,822·36,656·74,809개, [수역 팩](../artifacts/world-water/ga-state/full-tile-audit.log)은 99,703개이며 모두 부재·해독 실패 0개였다. 최종 팩 SHA-256은 도로 서부 `ac9793cbe3702d50eb7a9435f6d5bf4cab13935e5be53024d2f304ef9604db72`, 중서부 `bccd408894efc2ef31408489422d404a994666ca5fdbd056e9a812a783f9fe3d`, 중동부 `36d6bb65587aed090787fc70e18f2b1da7790db466e3e3ca7e40ae09a6596887`, 동부 `6c227147edf4733be35b603aa369a874918298a3b340c30fbe19c6d5e6865bd2`, 수역 `f54de4bf86e7e79d9dfb226b8f961760b0bc68e3dea5644c8769f1d458cd85d9`다.

기본 세계 모드의 [Atlanta](../artifacts/world-roads/ga-state/atlanta-road-water-z14.png)·[Savannah](../artifacts/world-roads/ga-state/savannah-road-water-z14.png)·[Augusta](../artifacts/world-roads/ga-state/augusta-road-water-z14.png) z14 화면과 [NC–GA 경계](../artifacts/world-roads/ga-state/nc-ga-border-world-z11.png) z11 화면의 타일 실패는 각각 0개였다. [SC](../artifacts/world-roads/ga-state/sc-overlap-audit.log)·[NC](../artifacts/world-roads/ga-state/nc-overlap-audit.log)·[TN](../artifacts/world-roads/ga-state/tn-overlap-audit.log)과 공통 비어 있지 않은 타일은 354·151·194개, 완전히 동일한 도로 좌표열은 0·4·56개였다. 중복 0개는 실제 도로 단절 또는 연결의 증거가 아니다. 부분 중복·주 경계의 실제 연결성·원천 완전성·NAD83↔WGS84 독립 위치 정확도·건물·역·공공기관·iPhone 성능은 아직 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 13 2026-09-25 data/us_tiger_2025_ga_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 13 2026-09-25 data/us_tiger_2025_ga_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_ga_counties.txt data/local/ga_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_ga_areawater.txt data/local/ga_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ga_counties.txt data/local/ga_roads data/us_tiger_ga_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ga_areawater.txt data/local/ga_areawater data/us_tiger_ga_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ga_state_roads.toml artifacts/world-roads/ga-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ga_state_areawater.toml artifacts/world-water/ga-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ga_state_roads.toml artifacts/world-roads/ga-state/state.mgeodb artifacts/world-roads/ga-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ga_state_areawater.toml artifacts/world-water/ga-state/water.mgeodb artifacts/world-water/ga-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- data/us_tiger_ga_state_roads.toml artifacts/world-roads/ga-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ga_state_roads.toml artifacts/world-roads/ga-state/state.pmtiles artifacts/world-roads/ga-state/roads
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ga_state_roads_west.toml artifacts/world-roads/ga-state/roads-west.pmtiles artifacts/world-roads/ga-state/roads-west
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ga_state_roads_west_east.toml artifacts/world-roads/ga-state/roads-west-east.pmtiles artifacts/world-roads/ga-state/roads-west-east
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_ga_state_roads.toml artifacts/world-roads/ga-state/state.mgeodb artifacts/world-roads/ga-state/state.rejected.json.gz
```
