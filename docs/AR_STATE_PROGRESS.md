# Arkansas 2025 TIGER/Line 도로·수역 실증 — 2026-09-25

## 원천과 계보

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수역 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 Arkansas 주 FIPS `05`의 ZIP을 각각 **75개** 확인했다. [도로 목록](../data/us_tiger_2025_ar_counties.txt)과 [수역 목록](../data/us_tiger_2025_ar_areawater.txt)의 카운티 번호는 일치한다. 공식 디렉터리 HTML SHA-256은 각각 `2eefa688d4c125943f8d5c63931f979552729edaa74d4b0545ac9edfbed14a48`, `6427d7e5ae58e3617007f2e14c2211c7b51a2bd95bcf482bbb8ea54ca486306c`다. ZIP별 공식 URL·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_ar_state_roads.toml)와 [수역 manifest](../data/us_tiger_ar_state_areawater.toml)에 고정했다. 원천 ZIP 다운로드는 도로 83,307,717바이트, 수역 25,872,166바이트였다. 도로 ZIP 한 건의 첫 응답은 ZIP 중앙 디렉터리가 없어 재시도했고 최종 75개 ZIP은 검사에 통과했다.

| 검사 | 도로 | 수역 |
| --- | --- | --- |
| 원본 DBF 행 | 285,084 | 57,669 |
| 채택·제외 | 254,273·30,811 | 57,664·5 |
| canonical GeoDB | 191,218,306바이트 | 53,557,161바이트 |
| 전체 PMTiles | 156,168개 타일·82,527,434바이트 | 60,971개 타일·30,417,753바이트 |
| 최종 팩 | [서부](../artifacts/world-roads/ar-state/roads-west.pmtiles) 44,929,325바이트, [동부](../artifacts/world-roads/ar-state/roads-east.pmtiles) 37,178,316바이트 | [수역](../artifacts/world-water/ar-state/water.pmtiles) 30,417,753바이트 |

각 ZIP의 Shapefile·DBF·NAD83 `.prj`·SHA-256을 검사했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-94.617859, 33.004389, -89.680243, 36.49957]`, 수역 `[-94.58156, 33.004305, -89.640997, 36.499168]`이다. [도로 계보 감사](../artifacts/world-roads/ar-state/lineage-audit.log)는 원본 285,084행이 채택 254,273개와 [제외 기록](../artifacts/world-roads/ar-state/state.rejected.json.gz) 30,811개로 설명된다고 확인했다. 주요 제외 분류는 `S1740` 23,479개, `S1750` 3,596개, `S1500` 2,508개다. 분류 뜻은 [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수역 제외 기록](../artifacts/world-water/ar-state/water.rejected.json.gz) 5개는 모호한 폴리곤 링이다.

## 타일 연결과 표시

[전체 도로 감사](../artifacts/world-roads/ar-state/full-tile-audit.log)는 z10–z15 **156,168개 타일**에서 확정 가능한 타일 내부 도로 경계 미일치 **0개**를 보고한다. 네 타일 꼭짓점 근처의 **11건**과 정수화된 얕은 선분의 추정 교차점 **1건**은 별도 모호 사례로 보고하며 실제 연결 완료로 단정하지 않는다. 후자는 z15/7877/13064 경계에서 꼭짓점까지 18 MVT 단위의 가짜 추정 교차점이었다. 감사 도구가 선분의 정수화 오차 범위를 계산해 이 사례를 별도로 기록하도록 바꿨고, 명시적으로 입력된 경계점의 미일치는 계속 실패하게 했다. 같은 코드로 [Georgia](GA_STATE_PROGRESS.md)·[Alabama](AL_STATE_PROGRESS.md)·[Mississippi](MS_STATE_PROGRESS.md)·[Louisiana](LA_STATE_PROGRESS.md)를 재감사했을 때 추가 모호 사례와 내부 미일치는 없었다.

[분할 감사](../artifacts/world-roads/ar-state/shard-audit.log)는 전체 도로 타일이 서부 78,271개와 동부 77,897개에 한 번씩 들어가며 압축 바이트가 같음을 확인했다. [서부](../artifacts/world-roads/ar-state/west-tile-audit.log)·[동부](../artifacts/world-roads/ar-state/east-tile-audit.log)·[수역](../artifacts/world-water/ar-state/full-tile-audit.log) 최종 팩의 전수 해독 결과는 각각 78,271·77,897·60,971개이고 부재·해독 실패는 모두 0개다. 최종 팩 SHA-256은 도로 서부 `722de9df0cb03365f90a0778b69e5c51a2104c42e91dff8e047541f535ac271e`, 동부 `241ffbde679aea2d986db487dd3b9a9c42e6472d1d7c74cb52cfd95399b23088`, 수역 `c80dd18903573b20ba690c2bd8a84df357c07ecdc4d441f298a69e8d7fc2942b`이다.

기본 세계 모드의 [Little Rock](../artifacts/world-roads/ar-state/little-rock-road-water-z14.png)·[Fayetteville](../artifacts/world-roads/ar-state/fayetteville-road-water-z14.png)·[Jonesboro](../artifacts/world-roads/ar-state/jonesboro-road-water-z14.png) z14 화면과 [AR–LA 경계](../artifacts/world-roads/ar-state/ar-la-border-world-z11.png) z11 화면의 타일 실패는 각각 0개였다. [LA](../artifacts/world-roads/ar-state/la-overlap-audit.log)·[MS](../artifacts/world-roads/ar-state/ms-overlap-audit.log)·[TN](../artifacts/world-roads/ar-state/tn-overlap-audit.log)과 공통 비어 있지 않은 타일은 386·300·115개, 완전히 동일한 도로 좌표열은 588·52·0개였다. 중복 0개도 실제 도로 단절 또는 연결의 증거가 아니다. 부분 중복·주 경계 실제 연결성·원천 완전성·NAD83↔WGS84 독립 위치 정확도·건물·역·공공기관·iPhone 성능은 아직 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 05 2026-09-25 data/us_tiger_2025_ar_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 05 2026-09-25 data/us_tiger_2025_ar_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_ar_counties.txt data/local/ar_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_ar_areawater.txt data/local/ar_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ar_counties.txt data/local/ar_roads data/us_tiger_ar_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ar_areawater.txt data/local/ar_areawater data/us_tiger_ar_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ar_state_roads.toml artifacts/world-roads/ar-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ar_state_areawater.toml artifacts/world-water/ar-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ar_state_roads.toml artifacts/world-roads/ar-state/state.mgeodb artifacts/world-roads/ar-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ar_state_areawater.toml artifacts/world-water/ar-state/water.mgeodb artifacts/world-water/ar-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- data/us_tiger_ar_state_roads.toml artifacts/world-roads/ar-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ar_state_roads.toml artifacts/world-roads/ar-state/state.pmtiles artifacts/world-roads/ar-state/roads
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_ar_state_roads.toml artifacts/world-roads/ar-state/state.mgeodb artifacts/world-roads/ar-state/state.rejected.json.gz
```
