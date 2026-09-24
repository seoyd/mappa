# 빅토리아 DTP 관리 도로 실증 — 2026-09-25

## 범위와 권리

[Transport Victoria의 DTP Managed Roads](https://opendata.transport.vic.gov.au/dataset/dtp-managed-roads/resource/7085bd59-9bed-4284-b226-703e3cfe3404)는 빅토리아주 Department of Transport and Planning이 관리하는 도로의 선형이다. **주 전체 모든 도로를 포함하는 Vicmap Transport Road Line이 아니다.** 공식 자원 페이지는 WGS84 GeoJSON, CC BY 4.0, 2026-03-10 갱신 자료로 표시한다. [DataVic의 자원 메타데이터](https://discover.data.vic.gov.au/dataset/dtp-managed-roads/resource/7085bd59-9bed-4284-b226-703e3cfe3404)에 적힌 `Copyright (c) The State of Victoria, Department of Energy, Environment and Climate Action` 문구를 manifest·타일 헤더·화면 출처 표기에 사용한다.

전국 또는 빅토리아주 전체 생활 도로를 채울 수 있는 [Vicmap Transport Road Line](https://discover.data.vic.gov.au/dataset/vicmap-transport-road-line)도 CC BY 4.0으로 게시돼 있다. 다만 그 SHP 자원은 현재 DataShare 주문 화면으로 연결된다. 이 실증은 바로 내려받을 수 있는 DTP 관리 도로 파일만 사용하며 Vicmap 전체 확보로 표시하지 않는다.

| 단계 | 관측 결과 |
|---|---|
| 공식 원본 | [GeoJSON 정적 다운로드](https://opendata.transport.vic.gov.au/dataset/858f87e7-c089-4b00-9d06-ff9c4c682367/resource/7085bd59-9bed-4284-b226-703e3cfe3404/download/dtp_managed_roads.geojson) 84,724,865바이트; SHA-256 `f6b88130da0b42b3d24356878a87cfc62ddbd6e44d8aa6707b03497befeff243`. 로컬 `data/local/au_vic/dtp_managed_roads.geojson`은 Git 제외. |
| 입력 구조 | GeoJSON `FeatureCollection`, `LineString` 90,797개, `OBJECTID` 90,797개 고유. 명시적인 CRS 속성은 없으며 공급자 자원 설명을 WGS84 판정 근거로 보존. |
| 클래스 | `CLASSN`: MR 52,584, HW 25,696, FW 7,405, TR 4,350, FR 572, NR 177, PR 13개. 공식 자원 페이지에 코드값 범례가 없어 `NR`·`PR` 190개는 의미를 추정해 도로로 넣지 않고 [거절 기록](../artifacts/world-roads/au-vic-dtp/roads.rejected.json)에 보존. |
| 정규화 | FW·HW는 주요, MR은 보조, TR·FR은 일반 도로 선으로 표시하는 **Mappa 화면 등급 규칙**을 적용. 90,607개 채택, 190개 제외. 출처별 SHA·`OBJECTID`·도형·CLASSN·이름 해시를 피처 계보에 기록. 이 화면 등급은 법적 노선 등급의 독립 해석이 아니다. |
| 범위 | 채택 도형의 경위도 최소·최대 `[140.9630929111, -39.0307555304, 149.7485225259, -34.1145370274]`. 독립 기준점과 현장 관측에 의한 실제 위치 정확도는 미검증. |
| GeoDB·팩 | GeoDB 37,912,101바이트; [지역 PMTiles](../artifacts/world-roads/au-vic-dtp/roads.pmtiles) 15,008,857바이트. z10–15 비어 있지 않은 타일 47,604개 전수 해독. 내부 타일 경계 일치 128,926건, 확정 불일치 0건, 꼭짓점 모호 1건. [감사 로그](../artifacts/world-roads/au-vic-dtp/roads-tile-audit.log). |
| 화면 | 기본 세계 모드 [Melbourne z14 Metal 캡처](../artifacts/world-roads/au-vic-dtp/melbourne-world-z14.png)의 타일 오류 0개. 화면에 지방정부의 일반 도로·건물·수면 상세가 빠져 있다. |

이 결과는 출처가 허용한 선형의 변환·파일 해독·표시 검증이다. 도로망의 완전성, 교차로 연결성, 출처 간 이음, 실제 위치 정확도와 iPhone 성능을 통과한 지도가 아니다. 앱 런타임은 외부 지도 API를 호출하지 않는다.

## 재현

```sh
cargo run --release --offline -p mappa-map-data --bin build_au_vic_dtp_roads -- \
  data/local/au_vic/dtp_managed_roads.geojson 2026-09-25 \
  artifacts/world-roads/au-vic-dtp/roads.mgeodb data/au_vic_dtp_roads.toml
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/au_vic_dtp_roads.toml artifacts/world-roads/au-vic-dtp/roads.mgeodb \
  artifacts/world-roads/au-vic-dtp/roads.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_canonical_tiles -- \
  data/au_vic_dtp_roads.toml artifacts/world-roads/au-vic-dtp/roads.pmtiles
```
