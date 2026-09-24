# 미국 웨스트버지니아주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `54`의 ZIP을 각각 55개 찾았다. 두 목록의 카운티 번호 55개는 모두 일치한다. [도로 목록](../data/us_tiger_2025_wv_counties.txt)과 [수면 목록](../data/us_tiger_2025_wv_areawater.txt)에 공식 디렉터리 HTML SHA-256을 각각 `ffa14eaa7a37faea61b09863a0c74e643a31f7276df6f0b3545e72527176f4f0`과 `8c35c67ad0ba0bd80fc77255b64be73bb316cae76ec7d500b87bb54874df3a0f`으로 고정했다. 110개 ZIP의 개별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_wv_state_roads.toml)와 [수면 manifest](../data/us_tiger_wv_state_areawater.toml)에 기록했다. 원본 ZIP은 빌드 입력이며 앱은 로컬 지도 팩을 읽는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 55개, 94,782,444바이트 | 55개, 11,164,640바이트 |
| 원본 DBF 레코드 | 253,548개 | 16,012개 |
| 채택 | 226,131개 | 16,004개 |
| 제외 | 27,417개: 현재 차량 도로 분류 밖 | 8개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 196,197,076바이트, SHA-256 `44eb2d97cd1413a265639a5947e4ac8257f86cd1156c19e175818d8c2e3246f1` | 로컬 20,068,299바이트, SHA-256 `3ab932c5b9029d8e0d2d649b3b94a4b65a40e28179e62af4ab64e418bfb89b51` |
| 전체 PMTiles | 로컬 64,699,322바이트, SHA-256 `89322c70ad5584c776f0ce8753c11c7f4c34944eddfa3755306096c6aa6e4367` | [수면](../artifacts/world-water/wv-state/water.pmtiles) 12,929,635바이트, SHA-256 `f2c8237b7d7361e88c96fc0a46a6aec10dd5237641f270d89faeaea7e44b714b` |
| 배포 PMTiles | [서부](../artifacts/world-roads/wv-state/roads-west.pmtiles) 40,114,144바이트, SHA-256 `237da6df2676998e56f96a96cf2d591782c55e0e026fc2689a596fade932d55c`; [동부](../artifacts/world-roads/wv-state/roads-east.pmtiles) 24,373,800바이트, SHA-256 `37f843daa6850df11204ebf5d6bc394cb6d974786a0eaafc940844dad6e4aea5` | 한 팩으로 배포 |
| 실제 타일 전수 해독 | [전체 80,579개](../artifacts/world-roads/wv-state/full-tile-audit.log), [서부 48,312개](../artifacts/world-roads/wv-state/west-tile-audit.log), [동부 32,267개](../artifacts/world-roads/wv-state/east-tile-audit.log): 각각 실패 0 | [28,917개](../artifacts/world-water/wv-state/tile-audit.log): 실패 0 |

Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-82.641518, 37.201541, -77.719708, 40.633345]`, 수면 `[-82.644591, 37.203653, -77.726194, 40.638801]`이다. [도로 제외](../artifacts/world-roads/wv-state/state.rejected.json.gz) 27,417개는 모두 제외 대상 도로/경로 분류이고, [수면 제외](../artifacts/world-water/wv-state/water.rejected.json.gz) 8개는 모두 모호한 폴리곤 링이다. 채택 피처와 원본 ID의 연결은 GeoDB에 남겼다.

도로 원본 PMTiles는 검증용 로컬 파일로 유지하고, z10 경도 `-80.5078125`에서 두 배포 팩으로 나눴다. [서부 manifest](../data/us_tiger_wv_state_roads_west.toml)와 [동부 manifest](../data/us_tiger_wv_state_roads_east.toml)는 동일한 55개 승인 원천을 보존한다. **원본 실제 타일 80,579개 모두가 정확히 한 팩에 있고, 각 타일의 압축 바이트가 원본과 동일**함을 Rust가 확인했다. 타일별 반복 포함 도로 수 역시 서부·동부 합계가 원본과 같다: major 7,365, collector 67,268, local 1,272,857.

`54039` 도로 파일의 첫 공식 URL 응답은 HTTP 200이지만 ZIP 대신 247바이트 거절 HTML이었다. 강화한 Rust 수집기는 ZIP 형식을 검사해 거절하고, 같은 공식 파일의 `?download=1` 요청으로 5,730,091바이트 정상 ZIP을 받았다. manifest 생성기의 ZIP·Shapefile 감사도 통과했다. 이 사례 때문에 HTTP 상태만으로 원본을 승인하지 않는다.

기본 세계 모드 [Charleston z14](../artifacts/world-roads/wv-state/charleston-road-water-z14.png)와 [Morgantown z14](../artifacts/world-roads/wv-state/morgantown-road-water-z14.png)에 시가지 도로와 하천이 함께 표시됐고 두 Metal 캡처의 타일 실패는 각각 0이었다. [Census 2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 이는 파일·타일·화면 검증이다. 실제 지형의 누락 없음, 카운티·주 경계 연결성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 54 2026-09-25 data/us_tiger_2025_wv_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 54 2026-09-25 data/us_tiger_2025_wv_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_wv_counties.txt data/local/wv_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_wv_areawater.txt data/local/wv_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_wv_counties.txt data/local/wv_roads data/us_tiger_wv_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_wv_areawater.txt data/local/wv_areawater data/us_tiger_wv_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_wv_state_roads.toml artifacts/world-roads/wv-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_wv_state_areawater.toml artifacts/world-water/wv-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_wv_state_roads.toml artifacts/world-roads/wv-state/state.mgeodb artifacts/world-roads/wv-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_wv_state_areawater.toml artifacts/world-water/wv-state/water.mgeodb artifacts/world-water/wv-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_wv_state_roads.toml artifacts/world-roads/wv-state/state.pmtiles artifacts/world-roads/wv-state/roads
```
