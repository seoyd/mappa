# Alabama 2025 TIGER/Line 도로·수역 실증 — 2026-09-25

## 원천과 계보

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수역 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 Alabama 주 FIPS `01`의 ZIP을 각각 **67개** 확인했다. [도로 목록](../data/us_tiger_2025_al_counties.txt)과 [수역 목록](../data/us_tiger_2025_al_areawater.txt)의 카운티 번호는 일치한다. 공식 디렉터리 HTML SHA-256은 각각 `01e09ff91e7474391d3660eaafbab0a11bb6828b188c0b59a03dd2335188089d`, `bce4c4eaa1554cac2ebfbf29f9b74447f10f27b934d1162a013bf0c102418d24`다. ZIP별 공식 URL·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_al_state_roads.toml)와 [수역 manifest](../data/us_tiger_al_state_areawater.toml)에 고정했다. 원천 ZIP 다운로드는 도로 82,035,276바이트, 수역 28,057,246바이트였다.

| 검사 | 도로 | 수역 |
| --- | --- | --- |
| 원본 DBF 행 | 338,523 | 53,061 |
| 채택·제외 | 310,125·28,398 | 53,047·14 |
| canonical GeoDB | 201,746,081바이트 | 55,810,349바이트 |
| 전체 PMTiles | 144,241개 타일·83,711,910바이트 | 68,612개 타일·33,194,095바이트 |
| 최종 팩 | [서부](../artifacts/world-roads/al-state/roads-west.pmtiles) 38,498,402바이트, [동부](../artifacts/world-roads/al-state/roads-east.pmtiles) 44,828,531바이트 | [수역](../artifacts/world-water/al-state/water.pmtiles) 33,194,095바이트 |

각 ZIP의 Shapefile·DBF·NAD83 `.prj`·SHA-256을 검사했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-88.472377, 30.225143, -84.901815, 35.007675]`, 수역 `[-88.456765, 30.144425, -84.888246, 35.008028]`이다. [도로 계보 감사](../artifacts/world-roads/al-state/lineage-audit.log)는 원본 338,523행이 채택 310,125개와 [제외 기록](../artifacts/world-roads/al-state/state.rejected.json.gz) 28,398개로 설명된다고 확인했다. 제외 분류에는 `S1740` 23,108개, `S1500` 2,632개, `S1750` 1,079개, `S1730` 895개, `S1780` 588개가 있다. 분류 뜻은 [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수역 제외 기록](../artifacts/world-water/al-state/water.rejected.json.gz) 14개는 모호한 폴리곤 링이다.

## 타일 연결과 표시

[전체 도로 감사](../artifacts/world-roads/al-state/full-tile-audit.log)는 z10–z15 **144,241개 타일**에서 타일 내부를 통과하는 도로 경계 연결의 미일치 **0개**를 보고한다. 네 타일이 만나는 꼭짓점에서 8 MVT 단위 이내인 **5건**은 방향이 모호해 별도로 보고하며 연결 완료로 단정하지 않는다. [분할 감사](../artifacts/world-roads/al-state/shard-audit.log)는 전체 도로 타일이 서부 70,765개와 동부 73,476개에 한 번씩 들어가며 압축 바이트가 같음을 확인했다. [서부](../artifacts/world-roads/al-state/west-tile-audit.log)·[동부](../artifacts/world-roads/al-state/east-tile-audit.log)·[수역](../artifacts/world-water/al-state/full-tile-audit.log) 최종 팩의 전수 해독 결과는 각각 70,765·73,476·68,612개이고 부재·해독 실패는 모두 0개다.

최종 팩 SHA-256은 도로 서부 `18c183b07fd8f25ad5fb1f9e4b4afcf9660ba468ccda3fc0444b7a92f2aa9880`, 동부 `82438c013997f981593b588ccfb3550d2384eb187d439aae6af8b51d5f9bb842`, 수역 `d11f9f6eb6e01665f41dca134cf95b209b0d3c16a5759605cef06de059b70283`이다.

기본 세계 모드의 [Birmingham](../artifacts/world-roads/al-state/birmingham-road-water-z14.png)·[Montgomery](../artifacts/world-roads/al-state/montgomery-road-water-z14.png)·[Mobile](../artifacts/world-roads/al-state/mobile-road-water-z14.png) z14 화면과 [AL–GA 경계](../artifacts/world-roads/al-state/al-ga-border-world-z11.png) z11 화면의 타일 실패는 각각 0개였다. [GA](../artifacts/world-roads/al-state/ga-overlap-audit.log)·[TN](../artifacts/world-roads/al-state/tn-overlap-audit.log)과 공통 비어 있지 않은 타일은 557·350개, 완전히 동일한 도로 좌표열은 32·133개였다. 부분 중복·주 경계의 실제 연결성·원천 완전성·NAD83↔WGS84 독립 위치 정확도·건물·역·공공기관·iPhone 성능은 아직 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 01 2026-09-25 data/us_tiger_2025_al_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 01 2026-09-25 data/us_tiger_2025_al_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_al_counties.txt data/local/al_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_al_areawater.txt data/local/al_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_al_counties.txt data/local/al_roads data/us_tiger_al_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_al_areawater.txt data/local/al_areawater data/us_tiger_al_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_al_state_roads.toml artifacts/world-roads/al-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_al_state_areawater.toml artifacts/world-water/al-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_al_state_roads.toml artifacts/world-roads/al-state/state.mgeodb artifacts/world-roads/al-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_al_state_areawater.toml artifacts/world-water/al-state/water.mgeodb artifacts/world-water/al-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- data/us_tiger_al_state_roads.toml artifacts/world-roads/al-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_al_state_roads.toml artifacts/world-roads/al-state/state.pmtiles artifacts/world-roads/al-state/roads
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_al_state_roads.toml artifacts/world-roads/al-state/state.mgeodb artifacts/world-roads/al-state/state.rejected.json.gz
```
