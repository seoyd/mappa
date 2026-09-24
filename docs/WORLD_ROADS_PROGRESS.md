# 국가별 공식 도로 구축 현황 — 2026-09-24

## 결정과 판정

사용자 결정에 따라 **ODbL 원천을 현재 Mappa canonical GeoDB에 넣지 않는다.** 전 세계 도로의 국가별 공식·허용형 원천을 찾고, 같은 출처·품질 게이트로 지역별로 구축한다. 이번 첫 실증은 미국 Census의 뉴욕 카운티 도로 한 파일이다. **전 세계 상세 도로망은 아직 없다.**

## 미국 공식 원천 실증

| 항목 | 관측 사실 |
|---|---|
| 원천 | [U.S. Census Bureau 2025 TIGER/Line All Roads](https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html), New York County 36061 ZIP 한 파일 |
| 권리 | [2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 저작물 재사용 허용 안내. 출처 표기를 요청하며 TIGER/Line 명칭은 Census 등록상표로 설명함. Mappa에는 데이터 출처만 표시. |
| 입력 무결성 | [원본 ZIP](../assets/map/source/public/us_tiger_2025_36061_roads.zip) SHA-256 `dd3c7163c148f70afb4d004cfbfa0dae221d9b854962973da5ce7e595eb675ca`; [manifest](../data/us_tiger_manhattan_roads.toml)에 URL·버전·CRS·권리 고정 |
| Rust 처리 | ZIP 내부 `.shp`·`.dbf`·`.prj` 직접 읽기, 입력 해시와 NAD83 `.prj` 확인, MTFCC 종류 분류, 도형·출처 레코드 생성 |
| 실제 피처 | 원본 선형 레코드 2,214개 → 도로 채택 1,893개, 도보·계단·자전거 등 차도와 구분해야 하는 종류 321개 거절 기록. 다중 부분선의 별도 피처는 포함되지 않았음. |
| GeoDB·타일 | 1,893개 피처와 출처, GeoDB 954,548바이트. 175개 비어 있지 않은 타일, 확대 단계별 중복 포함 도로선 11,373개, PMTiles 306,877바이트. 전체 타일 읽기 오류 0개. |
| 화면 | [맨해튼 도로 z14.6](../artifacts/world-roads/manhattan-roads-z14.png) Metal 캡처, 타일 오류 0개. 공식 도로선만 표시한 화면이라 해안·건물·공원은 비어 있음. |

채택한 MTFCC는 `S1100` 주요 도로, `S1200` 보조 도로, `S1400` 생활 도로, `S1630` 진출입로, `S1640` 서비스 도로다. 제외된 321개는 `S1710` 219, `S1780` 52, `S1730` 36, `S1750` 13, `S1820` 1개다. 별도 보행·자전거 레이어가 생기면 원본 종류 그대로 다시 검토한다. 원본 All Roads에는 이름이 다른 중복 선분이 있을 수 있다. 이번 실증은 중복 제거와 연결성 검사까지 완료한 도로 네트워크가 아니다.

ZIP의 `.prj`는 **EPSG:4269 NAD83**다. 현재 어댑터는 숫자 좌표를 그대로 보존하여 Web Mercator에 투영한다. WGS84 기준의 독립 기준점·datum 차이·연결성·노선 최신성은 검증되지 않았다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)는 여섯 자리 소수 좌표가 그만큼의 위치 정확도를 뜻하지 않고 원천별 정확도가 다르다고 명시한다. 따라서 이 화면을 미터급 위치 정확도 통과로 판정하지 않는다.

## 세계 확장에 필요한 상태

| 원천/지역 | 확보·판정 |
|---|---|
| 한국 나주 | 공식 도로 ZIP 실증은 기존 [지역 게이트](MAP_V0_3C_GATE_AUDIT.md)에 기록. 원본 CRS와 기준점 미확인으로 S16 `NO_GO`. |
| 미국 | Census 2025 뉴욕 카운티 한 파일만 실증. 나머지 미국 카운티는 미수집·미검증. 공식 배포 단위는 카운티별 All Roads 파일. |
| 영국 | [OS Open Roads](https://www.ordnancesurvey.co.uk/products/os-open-roads)는 무료 OGL 원천 후보. 다운로드·라이선스 표기·좌표계·정밀도·실물 파일은 아직 검증하지 않음. |
| 그 외 국가 | 공식 원천·배포 허용 조건·파일·좌표계·독립 정확도 조사 및 실증 필요. 빈 곳은 빈 곳으로 표시. |

Microsoft Road Detections와 Overture Transportation은 세계 규모의 후보지만 [제공자 설명](https://github.com/microsoft/RoadDetections), [Overture 권리 안내](https://docs.overturemaps.org/attribution/)상 ODbL이다. 사용자가 선택한 기존 라이선스 규칙 때문에 현재 canonical 입력에서 제외한다. 이 결정으로 세계 상세 도로 구축은 국가별 자료 확보가 완료될 때까지 **진행 중**이다.

## 재현

```bash
cargo run --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_manhattan_roads.toml artifacts/world-roads/manhattan-roads.mgeodb
cargo run --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_manhattan_roads.toml artifacts/world-roads/manhattan-roads.mgeodb \
  artifacts/world-roads/manhattan-roads.pmtiles
cargo run --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/manhattan-roads.pmtiles
MAPPA_DATASET=canonical-proof \
MAPPA_CANONICAL_FILE=artifacts/world-roads/manhattan-roads.pmtiles \
cargo run --offline -p mappa-map-demo -- --capture \
  -73.98 40.77 14.6 artifacts/world-roads/manhattan-roads-z14.png
```
