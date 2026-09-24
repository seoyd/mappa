# 미국 펜실베이니아주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `42`의 ZIP을 각각 67개 찾았다. 두 목록의 카운티 번호 67개는 모두 일치한다. [도로 목록](../data/us_tiger_2025_pa_counties.txt)과 [수면 목록](../data/us_tiger_2025_pa_areawater.txt)에 공식 디렉터리 HTML SHA-256을 각각 `927a3ddee657137e54591712719fe8c9706768ae22cb9e1300456d42f31e038a`와 `5077e810ba9269c7484a98681f31fb923c1dad0b9e5cc729048c3af0548e653e`으로 고정했다. 134개 ZIP의 개별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_pa_state_roads.toml)와 [수면 manifest](../data/us_tiger_pa_state_areawater.toml)에 기록했다. 원본 ZIP은 빌드 입력이며 앱은 로컬 지도 팩을 읽는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 67개, 158,084,722바이트 | 67개, 26,770,105바이트 |
| 원본 DBF 레코드 | 555,659개 | 46,909개 |
| 채택 | 490,912개 | 46,883개 |
| 제외 | 64,747개: 현재 차량 도로 분류 밖 | 26개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 361,446,050바이트, SHA-256 `250fc281b2af078daa4c9520e078a97f7831d060013ee2946aa14db12fc22f4f` | 로컬 51,381,946바이트, SHA-256 `844c7b88b3ae7b93fae7044d467e10525550c5578edb0ae59aca898101775e3f` |
| 전체 PMTiles | 로컬 132,086,138바이트, SHA-256 `67f0b0f97c4d94be24cc6ba235093cc8ce6ce46bf133aa47ef8a2ea07753c725` | [수면](../artifacts/world-water/pa-state/water.pmtiles) 29,540,681바이트, SHA-256 `21ec149352bbb9aacfa521481df69a55579b1abb6940aa48ee5c196bc40275c5` |
| 배포 PMTiles | [서부](../artifacts/world-roads/pa-state/roads-west.pmtiles) 63,087,078바이트, SHA-256 `8695ed44ab3f152c5ae2f65f1fb68c27f78be4c689c86c90da4d36a45dd7e79c`; [동부](../artifacts/world-roads/pa-state/roads-east.pmtiles) 68,540,203바이트, SHA-256 `7b2db1b7f5d7fb49a06a2901ad619a277e15fc8939310154355c6cbb305d6037` | 한 팩으로 배포 |
| 실제 타일 전수 해독 | [전체 167,144개](../artifacts/world-roads/pa-state/full-tile-audit.log), [서부 86,243개](../artifacts/world-roads/pa-state/west-tile-audit.log), [동부 80,901개](../artifacts/world-roads/pa-state/east-tile-audit.log): 각각 실패 0 | [63,699개](../artifacts/world-water/pa-state/tile-audit.log): 실패 0 |

Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-80.519495, 39.719823, -74.693695, 42.267546]`, 수면 `[-80.519851, 39.719845, -74.689561, 42.516072]`이다. [도로 제외](../artifacts/world-roads/pa-state/state.rejected.json.gz) 64,747개는 모두 제외 대상 도로/경로 분류이고, [수면 제외](../artifacts/world-water/pa-state/water.rejected.json.gz) 26개는 모두 모호한 폴리곤 링이다. 채택 피처와 원본 ID의 연결은 GeoDB에 남겼다.

도로 전체 PMTiles가 132MB이므로 Git 저장소에는 넣지 않고 검증용 로컬 원본으로 유지했다. Rust `shard_canonical_pmtiles`는 원본 범위에서 계산한 z10 경계 경도 `-77.6953125`에서 두 배포 팩으로 나눴다. [서부 manifest](../data/us_tiger_pa_state_roads_west.toml)와 [동부 manifest](../data/us_tiger_pa_state_roads_east.toml)는 동일한 67개 승인 원천을 보존한다. **원본 실제 타일 167,144개 모두가 정확히 한 팩에 있고, 각 타일의 압축 바이트가 원본과 동일**함을 확인했다. 타일별 반복 포함 도로 수 역시 서부·동부 합계가 원본과 같다: major 43,762, collector 200,107, local 2,766,871. 분할은 도로 형상을 단순화하거나 재계산하지 않는다.

기본 세계 모드 [Pittsburgh z14](../artifacts/world-roads/pa-state/pittsburgh-road-water-z14.png)와 [Philadelphia z14](../artifacts/world-roads/pa-state/philadelphia-road-water-z14.png)에 시가지 도로와 하천이 함께 표시됐고 두 Metal 캡처의 타일 실패는 각각 0이었다. [Census 2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 이는 파일·타일·화면 검증이다. 실제 지형의 누락 없음, 카운티·주 경계 연결성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 42 2026-09-25 data/us_tiger_2025_pa_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 42 2026-09-25 data/us_tiger_2025_pa_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_pa_counties.txt data/local/pa_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_pa_areawater.txt data/local/pa_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_pa_counties.txt data/local/pa_roads data/us_tiger_pa_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_pa_areawater.txt data/local/pa_areawater data/us_tiger_pa_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_pa_state_roads.toml artifacts/world-roads/pa-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_pa_state_areawater.toml artifacts/world-water/pa-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_pa_state_roads.toml artifacts/world-roads/pa-state/state.mgeodb artifacts/world-roads/pa-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_pa_state_areawater.toml artifacts/world-water/pa-state/water.mgeodb artifacts/world-water/pa-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_pa_state_roads.toml artifacts/world-roads/pa-state/state.pmtiles artifacts/world-roads/pa-state/roads
```
