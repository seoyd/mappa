# 미국 버지니아주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `51`의 ZIP을 각각 133개 찾았다. 두 목록의 카운티 상당 단위 번호 133개는 모두 일치한다. [도로 목록](../data/us_tiger_2025_va_counties.txt)과 [수면 목록](../data/us_tiger_2025_va_areawater.txt)에 공식 디렉터리 HTML SHA-256을 각각 `526e051dd75d176e84b27f7ae691aea6304443bb866b4ad4e5e8b7fbba28a135`와 `abb2eb611525fbf247deec620b6e2791ee2e4826b776831d2fc42d8ea0848a64`로 고정했다. 266개 ZIP의 개별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_va_state_roads.toml)와 [수면 manifest](../data/us_tiger_va_state_areawater.toml)에 기록했다. 원본 ZIP은 빌드 입력이며 앱은 로컬 지도 팩을 읽는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 133개, 194,926,543바이트 | 133개, 34,526,228바이트 |
| 원본 DBF 레코드 | 621,520개 | 55,326개 |
| 채택 | 447,095개 | 55,296개 |
| 제외 | 174,425개: 현재 차량 도로 분류 밖 | 30개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 343,990,306바이트, SHA-256 `3cfcfd2f1272d704815b6643b41fe681c5a57f74333dacc24518365eed505f84` | 로컬 65,247,897바이트, SHA-256 `7d5402e6613fcd27068d7e6967266d0c95fa956e110a2452818f062fab583630` |
| 전체 PMTiles | 로컬 118,410,675바이트, SHA-256 `e29567d1f5ccaa04d6fb6a8216bd7903953e8abe0d6f052f4e95c6cbbc7c31f3` | [수면](../artifacts/world-water/va-state/water.pmtiles) 38,403,054바이트, SHA-256 `c60968bebfab91f0530aec3a5209a8b67c74bd5feeecfa2cc2c38b1058ebbb67` |
| 배포 PMTiles | [서부](../artifacts/world-roads/va-state/roads-west.pmtiles) 37,265,960바이트, SHA-256 `94422b090e1b4a842b15a16bb625849a7ca8937371733117ce55b36fbbee5eef`; [중서부](../artifacts/world-roads/va-state/roads-east-west-west.pmtiles) 21,821,047바이트, SHA-256 `e660acf22fbbb0ebf01efd7482ff89bff6aa0b5801a16aab819d4f2b47cddbc7`; [중동부](../artifacts/world-roads/va-state/roads-east-west-east.pmtiles) 33,820,075바이트, SHA-256 `08d25d261e20b1b3b661a22c81887a93076bbaa4ae6f6782de855b632ca72eca`; [동부](../artifacts/world-roads/va-state/roads-east-east.pmtiles) 25,168,070바이트, SHA-256 `30d89882790ba4749b2d727322b43658a052992cd7b539e1e6363fa75f3167e5` | 한 팩으로 배포 |
| 실제 타일 전수 해독 | [전체 135,199개](../artifacts/world-roads/va-state/full-tile-audit.log), [서부 41,474개](../artifacts/world-roads/va-state/west-tile-audit.log), [중서부 29,086개](../artifacts/world-roads/va-state/east-west-west-tile-audit.log), [중동부 37,909개](../artifacts/world-roads/va-state/east-west-east-tile-audit.log), [동부 26,730개](../artifacts/world-roads/va-state/east-east-tile-audit.log): 각각 실패 0 | [80,358개](../artifacts/world-water/va-state/tile-audit.log): 실패 0 |

Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-83.669611, 36.540856, -75.245877, 39.465979]`, 수면 `[-83.642712, 36.540901, -75.166435, 39.437306]`이다. [도로 제외](../artifacts/world-roads/va-state/state.rejected.json.gz) 174,425개 중 Census 분류 `S1740` 서비스 도로가 80,269개, `S1500` 4륜구동 전용 비포장길이 69,695개, `S1750` 내부 사용 선이 21,146개다. [Census 기술 문서의 MTFCC 정의](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. 이들은 원본에 있지만 현재 차량 도로 표시 정책에서 제외한 선이다. [수면 제외](../artifacts/world-water/va-state/water.rejected.json.gz) 30개는 모두 모호한 폴리곤 링이다. [Rust 계보 감사](../artifacts/world-roads/va-state/lineage-audit.log)는 621,520개 원본 도로 행이 채택 447,095개·제외 174,425개로 모두 설명됨을 확인했다.

수면 파일 `51141`과 `51690`의 첫 공식 URL 응답은 ZIP 형식이 아니었다. Rust 수집기가 두 응답을 거절하고 같은 공식 파일을 `?download=1`로 재요청했으며, 이후 manifest 생성기의 ZIP·Shapefile 감사가 통과했다. HTTP 성공 상태만으로 원본을 승인하지 않는다.

도로 원본 PMTiles는 검증용 로컬 파일로 유지했다. Rust `shard_canonical_pmtiles`로 z10 경도 `-79.453125`에서 1차, 동부를 `-77.34375`에서 2차, 그 중서부를 `-78.3984375`에서 3차 분할했다. [서부 manifest](../data/us_tiger_va_state_roads_west.toml), [중서부 manifest](../data/us_tiger_va_state_roads_east_west_west.toml), [중동부 manifest](../data/us_tiger_va_state_roads_east_west_east.toml), [동부 manifest](../data/us_tiger_va_state_roads_east_east.toml)는 동일한 133개 승인 원천을 보존한다. 중간 [동부 manifest](../data/us_tiger_va_state_roads_east.toml)와 [중서부 manifest](../data/us_tiger_va_state_roads_east_west.toml)도 재현에 필요하다. 중간 PMTiles의 SHA-256은 각각 `551693193d4ea417a1fc9a542fd758130b9071e060520d16b73ee81047177014`, `f07b00b1e3e82ceb6bf1210617d2a59cd5cbdba42bab84da670c51162e14e88b`다. **원본 실제 타일 135,199개 모두가 정확히 한 최종 팩에 있고, 각 타일의 압축 바이트가 원본과 동일**함을 분할 단계마다 확인했다. 타일별 반복 포함 도로 수는 major 22,291, collector 150,030, local 2,458,366이며 최종 네 팩의 합계가 원본과 같다.

기본 세계 모드 [Roanoke z14](../artifacts/world-roads/va-state/roanoke-road-water-z14.png), [Charlottesville z14](../artifacts/world-roads/va-state/charlottesville-road-water-z14.png), [Richmond z14](../artifacts/world-roads/va-state/richmond-road-water-z14.png), [Norfolk z14](../artifacts/world-roads/va-state/norfolk-road-water-z14.png)에 도시 도로와 화면 일부의 수면이 함께 표시됐고 네 Metal 캡처의 타일 실패는 각각 0이었다. [Census 2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 이는 파일·타일·화면 검증이다. 실제 지형의 누락 없음, 카운티·주 경계 연결성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 51 2026-09-25 data/us_tiger_2025_va_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 51 2026-09-25 data/us_tiger_2025_va_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_va_counties.txt data/local/va_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_va_areawater.txt data/local/va_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_va_counties.txt data/local/va_roads data/us_tiger_va_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_va_areawater.txt data/local/va_areawater data/us_tiger_va_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_va_state_roads.toml artifacts/world-roads/va-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_va_state_areawater.toml artifacts/world-water/va-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_va_state_roads.toml artifacts/world-roads/va-state/state.mgeodb artifacts/world-roads/va-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_va_state_areawater.toml artifacts/world-water/va-state/water.mgeodb artifacts/world-water/va-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_va_state_roads.toml artifacts/world-roads/va-state/state.pmtiles artifacts/world-roads/va-state/roads
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_va_state_roads_east.toml artifacts/world-roads/va-state/roads-east.pmtiles artifacts/world-roads/va-state/roads-east
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_va_state_roads_east_west.toml artifacts/world-roads/va-state/roads-east-west.pmtiles artifacts/world-roads/va-state/roads-east-west
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_va_state_roads.toml artifacts/world-roads/va-state/state.mgeodb artifacts/world-roads/va-state/state.rejected.json.gz
```
