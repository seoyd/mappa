# 서호주 공식 도로 원천 구축 — 2026-09-25

## 원천·권리

[Main Roads Western Australia Road Network](https://catalogue.data.wa.gov.au/en/dataset/mrwa-road-network)은 주 도로와 도로 번호가 부여된 지방정부 도로의 중심선 및 일부 기타 선형을 제공한다. **서호주의 모든 실재 도로를 담았다는 뜻은 아니다.** WA 포털은 누구나 이용할 수 있는 CC BY 4.0 자료로 표시하며, 공유 시 Commissioner of Main Roads의 소유권·면책 고지를 포함하도록 명시한다. [Shapefile 배포](https://portal-mainroads.opendata.arcgis.com/api/download/v1/items/7febe68ed1764e0d8402264ad62f0357/shapefile?layers=17)를 빌드 시점에만 내려받았다. 앱 런타임은 로컬 팩만 읽는다.

| 항목 | 관측 결과 |
|---|---|
| 원본 | ZIP 36,332,073바이트, SHA-256 `fbdbdd56af8796394a960dcd90751bbb0a2812b6432ca2db9a7f875da521512e`; `data/local/au_wa/road_network.zip`에 로컬 보관, Git 제외 |
| 시점 | `unzip -l`에서 내부 Shapefile 파일 시각 2026-09-23을 보았으나 Rust ZIP 리더는 2026-09-22로 읽었다. WA 카탈로그의 `Data last updated`는 2025-06-05다. 어느 날짜도 검증된 발행 버전으로 해석하지 않고 원본 SHA-256을 스냅샷 식별자로 사용한다. |
| 좌표계 | `.prj`는 `GCS_GDA_1994` / GDA94 경위도, Shapefile 선형 Type 3. 어댑터가 수치 좌표를 그대로 보존하며 WGS84와의 실제 편차는 독립 측정하지 않았다. |
| 원본 행 | DBF·SHP 189,888개 선형 레코드, `GlobalID` 189,888개가 고유. 원본 헤더 경계 `[96.81832823, -35.125242098, 129.091195107, -10.413104773]`. 서호주 본토 밖 섬도 들어 있어 행정 경계와 일치하지 않는다. |
| 채택 | `State Road` 11,095, `Local Road` 174,168, `Miscellaneous Road` 3,601개. 이 중 유효한 도형 188,864개를 GeoDB에 보존했다. |
| 제외 | `Main Roads Controlled Path` 987, `Proposed Road` 34, `Crossover` 3개, 합계 1,024개. [행별 거절 기록](../artifacts/world-roads/au-wa/rejected.json) 보존. 종류별 분리는 차량 도로 렌더링을 위한 결정이며 원본 행을 삭제한 것이 아니다. |
| 출처 | 각 피처에 원본 `GlobalID:part`·원천 SHA-256·어댑터 버전과 피처 해시를 기록. [관리 지역별 manifest](../data/au_wa_roads_metropolitan.toml)에 권리·좌표·원본 해시·범위를 고정. |
| 타일 전수 감사 | 9개 팩의 비어 있지 않은 타일 253,612개를 해독했다. 같은 팩 내부의 타일 경계 일치 694,039건, 확정 불일치 0건, 꼭짓점 근처 판정 모호 1건, 정수화 모호 0건. [팩별 로그](../artifacts/world-roads/au-wa/metropolitan-tile-audit.log) 보존. 이는 관리 지역 사이의 도로 연결성 검사는 아니다. |

## 지역별 결과

원본 `RA_NAME`을 그대로 사용해 8개 관리 지역과 이름이 비어 있는 `Unassigned`를 나눴다. 한 원본 행은 한 지역만 소유하며, 각 지역 경계는 그 지역에서 **채택한 도형의 실제 범위**로 계산했다. 따라서 직사각형 범위가 다른 지역과 겹쳐도 도로 자체의 복제를 뜻하지 않는다.

표시 등급은 원본 `NETWORK_TYPE`에서만 정했다. `State Road`는 주요 도로, `Local Road`와 성격을 더 좁혀 말할 근거가 없는 `Miscellaneous Road`는 일반 도로로 표시한다. `Unassigned` 팩은 실제 첫 타일이 z12에 있어 카탈로그의 첫 표시 배율도 z12로 기록했다.

| 원본 지역 | 채택 도로선 | PMTiles 바이트 | 비어 있지 않은 z10–15 타일 |
|---|---:|---:|---:|
| Goldfields - Esperance | 5,987 | 8,015,432 | 45,809 |
| Great Southern | 9,640 | 5,343,574 | 24,777 |
| Kimberley | 2,866 | 3,074,264 | 15,318 |
| Metropolitan | 106,253 | 7,152,740 | 5,080 |
| Mid West-Gascoyne | 9,463 | 9,080,998 | 47,810 |
| Pilbara | 4,462 | 4,374,450 | 20,452 |
| South West | 28,404 | 6,491,804 | 19,643 |
| Unassigned | 287 | 740,535 | 4,124 |
| Wheatbelt | 21,502 | 13,601,627 | 70,599 |
| **합계** | **188,864** | **57,875,424** | **253,612** |

Rust 어댑터는 원본 ZIP SHA와 GDA94 `.prj`를 검증하고, `GlobalID` 중복·비정상 도형을 검사한다. [9개 팩 카탈로그](../assets/map/au_wa_regional_packs.toml)는 기본 세계 모드에서 자동 발견된다. 모든 팩 헤더에 공급자와 WA가 요청한 전체 고지 문구를 넣었다. [Perth z14 Metal 캡처](../artifacts/world-roads/au-wa/perth-world-z14.png)는 타일 오류 0개였다. 이는 표시·파일 해독 결과이며 현장 위치 정확도, 실제 도로 연결성, 도로망 완전성, iPhone 성능의 통과 판정은 아니다. 상세 수면·건물·역·기관은 이 WA 원천에 포함하지 않는다.

## 재현

```sh
cargo run --release --offline -p mappa-map-data --bin build_wa_road_regions -- \
  data/local/au_wa/road_network.zip 2026-09-25 artifacts/world-roads/au-wa data
for manifest in data/au_wa_roads_*.toml; do
  code=${manifest#data/au_wa_roads_}; code=${code%.toml}
  cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
    "$manifest" "artifacts/world-roads/au-wa/$code.mgeodb" "artifacts/world-roads/au-wa/$code.pmtiles"
  cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- \
    "$manifest" "artifacts/world-roads/au-wa/$code.pmtiles"
done
cargo run --release --offline -p mappa-map-data --bin make_wa_road_catalog -- \
  data artifacts/world-roads/au-wa assets/map/au_wa_regional_packs.toml
```
