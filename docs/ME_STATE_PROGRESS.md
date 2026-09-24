# 미국 메인주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `23`의 파일을 각각 16개 찾았다. [도로 목록](../data/us_tiger_2025_me_counties.txt)과 [수면 목록](../data/us_tiger_2025_me_areawater.txt)은 디렉터리 HTML SHA-256을 포함한다. 공식 ZIP 32개를 Rust 수집기로 받았고 파일별 SHA-256·출처·권리 판정은 [도로 manifest](../data/us_tiger_me_state_roads.toml)와 [수면 manifest](../data/us_tiger_me_state_areawater.toml)에 기록했다. 원본 ZIP은 빌드 입력이며 지도 실행 중 외부 API는 호출하지 않는다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 16개, 39,732,631바이트 | 16개, 13,185,553바이트 |
| 원본 DBF 레코드 | 135,188개 | 8,679개 |
| 채택 | 118,918개 | 8,665개 |
| 제외 | 16,270개: 현재 차량 도로 분류 밖 | 14개: 내부 링의 짝이 모호함 |
| GeoDB | 로컬 87,317,226바이트, SHA-256 `bbc1b26729f85941c84a09a3b67a0bcaf58390bf1a01ab58e2c2cd66da617395` | 로컬 21,359,000바이트, SHA-256 `e90671a85ab648a4beead2bad2caae0ef913f2d4b9ed8078904040e0be18a246` |
| PMTiles | [도로](../artifacts/world-roads/me-state/state.pmtiles) 44,434,955바이트, SHA-256 `9ba930517ffb8c0a443bb21a163dd2237ea89c568bb5f38852a44eada2cfcd50` | [수면](../artifacts/world-water/me-state/water.pmtiles) 20,305,722바이트, SHA-256 `21def33c7ea380ea4690e293dd987333a1562e87eb968cfaf8fd433655bce7fd` |
| 실제 타일 전수 해독 | [102,440개](../artifacts/world-roads/me-state/tile-audit.log), 실패 0 | [57,555개](../artifacts/world-water/me-state/tile-audit.log), 실패 0 |

Rust 감사는 ZIP마다 Shapefile 형식·DBF 레코드 수·NAD83 `.prj`·해시를 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-71.078292, 43.065873, -66.952021, 47.459634]`, 수면 `[-71.051247, 42.917126, -66.885444, 47.417249]`다. 채택 피처의 원본 ID와 파일 출처는 GeoDB에 있으며 [도로 제외](../artifacts/world-roads/me-state/state.rejected.json.gz)와 [수면 제외](../artifacts/world-water/me-state/water.rejected.json.gz) 사유를 별도로 보존했다.

기본 세계 모드 [Portland z14 도로·수면 화면](../artifacts/world-water/me-state/portland-road-water-z14.png)에 해안 수면과 도로가 함께 보였고 Metal 타일 실패는 0이었다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 원천 재사용 조건과 출처 표기를 적용했다. 이는 파일·타일·화면 검증이다. 실제 도로·수면 누락 없음, 카운티·주 경계 연결성, NAD83↔WGS84 독립 위치 정확도, iPhone 성능은 검증하지 않았다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 23 2026-09-25 data/us_tiger_2025_me_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 23 2026-09-25 data/us_tiger_2025_me_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_me_counties.txt data/local/me_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_me_areawater.txt data/local/me_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_me_counties.txt data/local/me_roads data/us_tiger_me_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_me_areawater.txt data/local/me_areawater data/us_tiger_me_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_me_state_roads.toml artifacts/world-roads/me-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_me_state_areawater.toml artifacts/world-water/me-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_me_state_roads.toml artifacts/world-roads/me-state/state.mgeodb artifacts/world-roads/me-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_me_state_areawater.toml artifacts/world-water/me-state/water.mgeodb artifacts/world-water/me-state/water.pmtiles
```
