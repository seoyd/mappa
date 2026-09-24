# 미국 켄터키주 도로·수면 확장 — 2026-09-25

Rust `discover_tiger_inventory`가 [US Census 2025 TIGER/Line 도로 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)와 [수면 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/)에서 주 FIPS `21`의 ZIP을 각각 **120개** 찾았다. 두 목록의 카운티 번호가 전부 일치한다. [도로 목록](../data/us_tiger_2025_ky_counties.txt)과 [수면 목록](../data/us_tiger_2025_ky_areawater.txt)은 공식 디렉터리 HTML SHA-256 `f8bcb275e1145bffbc47a61124664ae79b8d659834950e7ad8126235179cb050`, `4db3636958ab5b64d8e27bd2454b6557d63acee7d78eaeafef568ee1176caa77`를 고정한다. ZIP별 출처·SHA-256·권리 판정은 [도로 manifest](../data/us_tiger_ky_state_roads.toml)와 [수면 manifest](../data/us_tiger_ky_state_areawater.toml)에 있다.

| 단계 | 도로 | 수면 |
|---|---:|---:|
| 공식 ZIP | 120개, 101,686,459바이트 | 120개, 51,622,000바이트 |
| 원본 DBF 행 | 261,570개 | 185,665개 |
| 채택 | 233,240개 | 185,649개 |
| 제외 | 28,330개: 현재 차량 도로 표시 분류 밖 | 16개: 내부 링 짝이 모호함 |
| GeoDB | 로컬 211,206,156바이트, SHA-256 `ea83432d841bad326cf6c740e4f98e95ea5a4f1d629366d2fcb5f0e7bdbdc959` | 로컬 127,346,321바이트, SHA-256 `8ab2a28686a1511902f8eef4b8b88c515d563ced8c0dfa3d03a3bc8bb8a9784a` |
| 전체 PMTiles | 로컬 85,073,450바이트, SHA-256 `77c5559be05af776bd59370f94d066a035fec1891333f9f29d18179007a40268` | 로컬 56,162,162바이트, SHA-256 `d3b610ec9d64d316b62df7735a99450496be942ecfda71ae370c847c3cec19ff` |
| 최종 도로 팩 | [서부](../artifacts/world-roads/ky-state/roads-west.pmtiles) 26,999,847바이트, SHA `4a6d17d11aab98c9acdfdbfea7203420cfb87443aff59e2455207ac0c12c0a83`; [중동부](../artifacts/world-roads/ky-state/roads-east-west.pmtiles) 33,337,633바이트, SHA `220d6654d411c90399bb5e681bca455e6f6f3bb12d4cc908a9ed43853fd8f588`; [동부](../artifacts/world-roads/ky-state/roads-east-east.pmtiles) 24,396,909바이트, SHA `d4b1b3e0e28d69b6c68fd8771be989671dc3ec00a2c8b9692014874867c4bfaa` | 해당 없음 |
| 최종 수면 팩 | 해당 없음 | [서부](../artifacts/world-water/ky-state/water-west.pmtiles) 24,075,659바이트, SHA `8f795f2081c0d467db3db3de7b4b5d79509f2570eace16620e912b793bd46dbe`; [동부](../artifacts/world-water/ky-state/water-east.pmtiles) 31,795,898바이트, SHA `2d76d87977e4f27b4e0c1471410a333e6049598765d734b50c98d1d3b179ef4b` |
| 전체 타일 해독 | [원본 도로 131,626개](../artifacts/world-roads/ky-state/full-tile-audit.log), [서부 47,339개](../artifacts/world-roads/ky-state/west-tile-audit.log), [중동부 48,164개](../artifacts/world-roads/ky-state/east-west-tile-audit.log), [동부 36,123개](../artifacts/world-roads/ky-state/east-east-tile-audit.log): 부재·실패 0 | [원본 수면 109,195개](../artifacts/world-water/ky-state/full-tile-audit.log), [서부 46,163개](../artifacts/world-water/ky-state/west-tile-audit.log), [동부 63,032개](../artifacts/world-water/ky-state/east-tile-audit.log): 부재·실패 0 |

각 ZIP의 Shapefile 구조·DBF 행 수·NAD83 `.prj`·SHA-256을 확인했다. 원본 헤더의 숫자 좌표 범위 합집합은 도로 `[-89.525945, 36.497553, -82.001354, 39.140527]`, 수면 `[-89.571203, 36.497058, -82.062892, 39.147732]`이다. [도로 제외 기록](../artifacts/world-roads/ky-state/state.rejected.json.gz) 28,330개 중 원천 분류 `S1740`이 22,627개, `S1750`이 2,594개, `S1500`이 1,592개다. [Census 기술 문서의 MTFCC 정의](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. [수면 제외 기록](../artifacts/world-water/ky-state/water.rejected.json.gz) 16개는 모두 모호한 폴리곤 링이다. [Rust 도로 계보 감사](../artifacts/world-roads/ky-state/lineage-audit.log)에서 원본 **261,570행이 채택 233,240개와 제외 28,330개로 모두 설명**됐다.

도로 원본 PMTiles는 z10 경도 `-85.78125`에서 분할했고 큰 동부 팩을 `-84.0234375`에서 다시 나눴다. [1차](../artifacts/world-roads/ky-state/shard-audit.log)·[2차](../artifacts/world-roads/ky-state/east-shard-audit.log) 감사에서 원본 131,626개 실제 타일이 정확히 한 최종 팩에 있고 압축 바이트가 동일했다. 수면 원본 PMTiles도 `-85.78125`에서 [분할 감사](../artifacts/world-water/ky-state/shard-audit.log)를 거쳐 원본 109,195개 실제 타일의 압축 바이트를 보존했다. 최종 도로·수면 팩의 manifest는 동일 승인 원천과 각 공간 경계를 보존한다. 전체·중간 아카이브는 로컬 검증용이다.

기본 세계 모드의 [Paducah](../artifacts/world-roads/ky-state/paducah-road-water-z14.png), [Louisville](../artifacts/world-roads/ky-state/louisville-road-water-z14.png), [Pikeville](../artifacts/world-roads/ky-state/pikeville-road-water-z14.png) z14 Metal 화면과 [TN–KY 경계](../artifacts/world-roads/ky-state/tn-ky-border-world-z11.png) z11 화면의 타일 실패는 각각 0개였다. [TN](../artifacts/world-roads/ky-state/tn-overlap-audit.log), [VA](../artifacts/world-roads/ky-state/va-overlap-audit.log), [WV](../artifacts/world-roads/ky-state/wv-overlap-audit.log), [OH](../artifacts/world-roads/ky-state/oh-overlap-audit.log) 경계의 공통 비어 있지 않은 타일은 각각 787개·218개·323개·371개이고 완전히 동일한 도로 좌표열은 653개·97개·4개·0개다. 0개는 실제 도로 단절의 증거도, 연결의 증거도 아니다. 세계 모드는 정확히 동일한 좌표열의 중복 표시를 제거한다. 부분 중복, 실제 연결성, NAD83↔WGS84 독립 위치 정확도, 원본 완전성, 건물·역·공공기관, iPhone 성능은 검증하지 않았다.

## 재현

```sh
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --roads 21 2026-09-25 data/us_tiger_2025_ky_counties.txt
cargo run --release --offline -p mappa-map-acquire --bin discover_tiger_inventory -- --areawater 21 2026-09-25 data/us_tiger_2025_ky_areawater.txt
cargo run --release -p mappa-map-acquire -- data/us_tiger_2025_ky_counties.txt data/local/ky_roads
cargo run --release -p mappa-map-acquire -- --areawater data/us_tiger_2025_ky_areawater.txt data/local/ky_areawater
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ky_counties.txt data/local/ky_roads data/us_tiger_ky_state_roads.toml 2026-09-25
cargo run --release --offline -p mappa-map-data --bin make_us_tiger_roads_manifest -- data/us_tiger_2025_ky_areawater.txt data/local/ky_areawater data/us_tiger_ky_state_areawater.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ky_state_roads.toml artifacts/world-roads/ky-state/state.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- data/us_tiger_ky_state_areawater.toml artifacts/world-water/ky-state/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ky_state_roads.toml artifacts/world-roads/ky-state/state.mgeodb artifacts/world-roads/ky-state/state.pmtiles
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- data/us_tiger_ky_state_areawater.toml artifacts/world-water/ky-state/water.mgeodb artifacts/world-water/ky-state/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ky_state_roads.toml artifacts/world-roads/ky-state/state.pmtiles artifacts/world-roads/ky-state/roads
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ky_state_roads_east.toml artifacts/world-roads/ky-state/roads-east.pmtiles artifacts/world-roads/ky-state/roads-east
cargo run --release --offline -p mappa-map-data --bin shard_canonical_pmtiles -- data/us_tiger_ky_state_areawater.toml artifacts/world-water/ky-state/water.pmtiles artifacts/world-water/ky-state/water
cargo run --release --offline -p mappa-map-data --bin audit_tiger_road_lineage -- data/us_tiger_ky_state_roads.toml artifacts/world-roads/ky-state/state.mgeodb artifacts/world-roads/ky-state/state.rejected.json.gz
```
