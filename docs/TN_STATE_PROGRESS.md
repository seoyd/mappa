# 미국 테네시주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `47`의 ZIP을 각각 **95개** 찾았다. 두 목록의 카운티 번호가 모두 일치한다. [도로 목록](../data/us_tiger_2025_tn_counties.txt)과 [수면 목록](../data/us_tiger_2025_tn_areawater.txt)은 공식 디렉터리 HTML SHA-256 `619ee552b15b97f56e03096eaa50c6c32a281fac8f2cefcbb9669ee7529e1ad9`, `4668b9cad3a1633c1aaf245b37ef5a62921546fb336100c42ad497e2b3d5173e`를 고정한다. ZIP별 출처·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_tn_state_roads.toml)와 [수면 manifest](../data/us_tiger_tn_state_areawater.toml)에 있다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 95개, 108,121,388바이트 | 95개, 11,653,444바이트 |
| 원본 DBF 행 | 344,502개 | 3,836개 |
| 채택 | 307,068개 | 3,831개 |
| 제외 | 37,434개: 현재 차량 도로 표시 분류 밖 | 5개: 내부 링 짝이 모호함 |
| GeoDB | 로컬 235,750,368바이트, SHA-256 `9cdc6ffedf47894fcb984756b754619ce9e3fa1b5f7761e79af557a73f818b86` | 로컬 17,281,925바이트, SHA-256 `fddbb26924b74303bc5f020c7d15675a1491536783278af9862ae1cc82a78650` |
| 전체 PMTiles | 로컬 96,291,131바이트, SHA-256 `fa7c3eee2576c5ec0a6905a1bdbdec5073ba6c808c39e403a901db1b557f5a80` | [수면](../artifacts/world-water/tn-state/water.pmtiles) 13,071,421바이트, SHA-256 `949be582a9131ad057ecb0d92a8ff91818b854ca4ee08b4003eec162f51fb63e` |
| 최종 도로 팩 | [서부](../artifacts/world-roads/tn-state/roads-west.pmtiles) 46,464,828바이트, SHA `f11f8037c97af654085127aa28e39c89cf038e389e60eef15e16ba3a00430857`; [동부](../artifacts/world-roads/tn-state/roads-east.pmtiles) 49,468,493바이트, SHA `cf7990e107441fa3ec4f6579dcd15a26670a90b35144f750c5704532ea3303c6` | 해당 없음 |
| 전체 타일 해독 | [원본 도로 131,851개](../artifacts/world-roads/tn-state/full-tile-audit.log), [서부 71,948개](../artifacts/world-roads/tn-state/west-tile-audit.log), [동부 59,903개](../artifacts/world-roads/tn-state/east-tile-audit.log): 부재·실패 0 | [수면 21,899개](../artifacts/world-water/tn-state/tile-audit.log): 부재·실패 0 |

각 ZIP의 Shapefile 구조·DBF 행 수·NAD83 `.prj`·SHA-256을 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-90.246957, 34.982924, -81.652101, 36.677953]`, 수면 `[-90.310491, 34.983035, -81.918051, 36.678255]`이다. [도로 제외 기록](../artifacts/world-roads/tn-state/state.rejected.json.gz) 37,434개 중 원천 분류 `S1740`이 19,954개, `S1750`이 8,550개, `S1500`이 3,385개다. [Census 기술 문서의 MTFCC 정의](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수면 제외 기록](../artifacts/world-water/tn-state/water.rejected.json.gz) 5개는 모두 모호한 폴리곤 링이다. [Rust 도로 계보 감사](../artifacts/world-roads/tn-state/lineage-audit.log)에서 원본 **344,502행이 채택 307,068개와 제외 37,434개로 모두 설명**됐다.

도로 원본 PMTiles는 검증용 로컬 파일이다. Rust `shard_canonical_pmtiles`로 z10 경도 `-86.1328125`에서 나눴다. [분할 감사](../artifacts/world-roads/tn-state/shard-audit.log)에서 원본 131,851개 실제 타일이 정확히 한 최종 팩에 있고 압축 바이트가 동일했다. 최종 [서부 manifest](../data/us_tiger_tn_state_roads_west.toml)와 [동부 manifest](../data/us_tiger_tn_state_roads_east.toml)는 동일한 95개 승인 원천과 각 공간 경계를 보존한다.

기본 세계 모드의 [Memphis](../artifacts/world-roads/tn-state/memphis-road-water-z14.png), [Nashville](../artifacts/world-roads/tn-state/nashville-road-water-z14.png), [Knoxville](../artifacts/world-roads/tn-state/knoxville-road-water-z14.png) z14 Metal 화면에서 도로가 보이고 타일 실패는 각각 0개였다. 수면 원본의 3,831개 폴리곤이 특정 화면의 강·호수를 모두 표현한다는 뜻은 아니다. [NC–TN 경계 감사](../artifacts/world-roads/tn-state/nc-overlap-audit.log)는 공통 비어 있지 않은 타일 325개·완전히 동일한 도로 좌표열 91개, [VA–TN 경계 감사](../artifacts/world-roads/tn-state/va-overlap-audit.log)는 296개·95개를 관측했다. 세계 모드의 동일 좌표열 도로 중복 표시 제거가 적용된다. [NC–TN 경계](../artifacts/world-roads/tn-state/nc-tn-border-world-z11.png)와 [VA–TN 경계](../artifacts/world-roads/tn-state/va-tn-border-world-z11.png) z11 화면은 타일 실패 0개다. 부분 중복, 실제 도로 연결성, NAD83↔WGS84 독립 위치 정확도, 원본 완전성, 건물·역·공공기관, iPhone 성능은 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 47 2026-09-25 data/us_tiger_2025_tn_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 47 2026-09-25 data/us_tiger_2025_tn_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_tn_counties.txt data/local/tn_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_tn_areawater.txt data/local/tn_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_tn_counties.txt data/local/tn_roads data/us_tiger_tn_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_tn_areawater.txt data/local/tn_areawater data/us_tiger_tn_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_tn_state_roads.toml artifacts/world-roads/tn-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_tn_state_areawater.toml artifacts/world-water/tn-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_tn_state_roads.toml artifacts/world-roads/tn-state/state.mgeodb artifacts/world-roads/tn-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_tn_state_areawater.toml artifacts/world-water/tn-state/water.mgeodb artifacts/world-water/tn-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_tn_state_roads.toml artifacts/world-roads/tn-state/state.pmtiles artifacts/world-roads/tn-state/roads
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_tn_state_roads.toml artifacts/world-roads/tn-state/state.mgeodb artifacts/world-roads/tn-state/state.rejected.json.gz
```
