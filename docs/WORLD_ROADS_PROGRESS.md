# 국가별 공식 도로 구축 현황 — 2026-09-25

## 결정과 판정

사용자 결정에 따라 **ODbL 원천을 현재 Mappa canonical GeoDB에 넣지 않는다.** 전 세계 도로의 국가별 공식·허용형 원천을 찾고, 같은 출처·품질 게이트로 지역별로 구축한다. 첫 실증인 미국 Census 뉴욕 카운티 한 파일을 뉴욕시 5개 카운티로, 이어 [뉴욕주 62개 카운티](NY_STATE_ROADS_PROGRESS.md)로 확장하고 [뉴저지주 21개 카운티](NJ_STATE_ROADS_PROGRESS.md), [델라웨어주 3개 카운티](DE_STATE_ROADS_PROGRESS.md), [코네티컷주 9개 계획지역](CT_STATE_ROADS_PROGRESS.md), [로드아일랜드주 5개 카운티](RI_STATE_ROADS_PROGRESS.md), [매사추세츠주 14개 카운티](MA_STATE_ROADS_PROGRESS.md), [뉴햄프셔주 10개 카운티](NH_STATE_PROGRESS.md), [버몬트주 14개 카운티](VT_STATE_PROGRESS.md), [메인주 16개 카운티](ME_STATE_PROGRESS.md), [메릴랜드주 24개 카운티 상당 단위](MD_STATE_PROGRESS.md), [펜실베이니아주 67개 카운티](PA_STATE_PROGRESS.md), [웨스트버지니아주 55개 카운티](WV_STATE_PROGRESS.md), [오하이오주 88개 카운티](OH_STATE_PROGRESS.md), [버지니아주 133개 카운티 상당 단위](VA_STATE_PROGRESS.md), [워싱턴 DC](DC_PROGRESS.md)를 추가했다. [프랑스 IGN 파리 D075 도로](FR_PARIS_ROADS_PROGRESS.md)도 첫 지역 팩으로 구축했다. 현재 기본 세계 모드는 미국 14개 주와 DC, 프랑스 파리 원천 범위, [Great Britain 공식 RoadLink 52격자](GB_ROADS_PROGRESS.md), [캐나다 13개 지역 공식 NRN](CA_ROADS_PROGRESS.md)의 도로 팩을 사용한다. **전 세계 상세 도로망은 아직 없다.**

## 미국 공식 원천 실증

| 항목 | 관측 사실 |
|---|---|
| 원천 | [U.S. Census Bureau 2025 TIGER/Line All Roads](https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html), New York County 36061 ZIP 한 파일 |
| 권리 | [2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)의 미국 정부 저작물 재사용 허용 안내. 출처 표기를 요청하며 TIGER/Line 명칭은 Census 등록상표로 설명함. Mappa에는 데이터 출처만 표시. |
| 입력 무결성 | [원본 ZIP](../assets/map/source/public/us_tiger_2025_36061_roads.zip) SHA-256 `dd3c7163c148f70afb4d004cfbfa0dae221d9b854962973da5ce7e595eb675ca`; [manifest](../data/us_tiger_manhattan_roads.toml)에 URL·버전·CRS·권리 고정 |
| Rust 처리 | ZIP 내부 `.shp`·`.dbf`·`.prj` 직접 읽기, 입력 해시와 NAD83 `.prj` 확인, MTFCC 종류 분류, 도형·출처 레코드 생성 |
| 실제 피처 | 원본 선형 레코드 2,214개 → 도로 채택 1,893개, 도보·계단·자전거 등 차도와 구분해야 하는 종류 321개 거절 기록. 다중 부분선의 별도 피처는 포함되지 않았음. |
| GeoDB·타일 | 1,893개 피처와 출처, GeoDB 954,548바이트. 175개 비어 있지 않은 타일, 확대 단계별 중복 포함 도로선 11,373개, PMTiles 306,877바이트. 전체 타일 읽기 오류 0개. |
| 화면 | [맨해튼 도로 z14.6](../artifacts/world-roads/manhattan-roads-z14.png) 단독 모드와 [세계 모드 결합](../artifacts/world-integration/manhattan-world.png) Metal 캡처, 타일 오류 0개. 공식 도로선만 표시한 화면이라 해안·건물·공원은 비어 있음. |

채택한 MTFCC는 `S1100` 주요 도로, `S1200` 보조 도로, `S1400` 생활 도로, `S1630` 진출입로, `S1640` 서비스 도로다. 제외된 321개는 `S1710` 219, `S1780` 52, `S1730` 36, `S1750` 13, `S1820` 1개다. 별도 보행·자전거 레이어가 생기면 원본 종류 그대로 다시 검토한다. 원본 All Roads에는 이름이 다른 중복 선분이 있을 수 있다. 이번 실증은 중복 제거와 연결성 검사까지 완료한 도로 네트워크가 아니다.

ZIP의 `.prj`는 **EPSG:4269 NAD83**다. 현재 어댑터는 숫자 좌표를 그대로 보존하여 Web Mercator에 투영한다. WGS84 기준의 독립 기준점·datum 차이·연결성·노선 최신성은 검증되지 않았다. [Census 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)는 여섯 자리 소수 좌표가 그만큼의 위치 정확도를 뜻하지 않고 원천별 정확도가 다르다고 명시한다. 따라서 이 화면을 미터급 위치 정확도 통과로 판정하지 않는다.

## 뉴욕시 5개 카운티 확장

[미국 Census의 2025 TIGER/Line All Roads 배포 디렉터리](https://www2.census.gov/geo/tiger/TIGER2025/ROADS/)에서 Bronx(36005), Kings(36047), New York(36061), Queens(36081), Richmond(36085) ZIP을 받았다. 네 새 원본과 기존 맨해튼 파일의 SHA-256·NAD83 `.prj`·Shapefile 헤더 범위·DBF 레코드 수를 Rust `audit_us_road_source`로 확인했다. 헤더 수치 범위의 합집합 `[-74.255694, 40.497866, -73.70036, 40.914931]`을 [5원천 manifest](../data/us_tiger_nyc_roads.toml)에 기록했다. 이는 관측된 원천 헤더 범위이며 WGS84 독립 정확도 검증 결과가 아니다.

| 단계 | 관측 결과 |
|---|---|
| 원본 | 5개 ZIP의 DBF 도로·경로 레코드 합계 22,014개; 추가 4개 ZIP 합계 약 2.4 MiB. ZIP SHA-256은 manifest에 각각 고정. |
| 정규화 | 도로 채택 20,882개, 보행·자전거 등 종류 규칙으로 제외 1,132개; 출처 5개와 피처별 provenance 20,882개. [거절 기록](../artifacts/world-roads/nyc-five-boroughs.rejected.json) 포함. |
| GeoDB | [5개 카운티 GeoDB](../artifacts/world-roads/nyc-five-boroughs.mgeodb) 9,707,642바이트. |
| 타일 | [지역 PMTiles](../artifacts/world-roads/nyc-five-boroughs.pmtiles) 1,522개 비어 있지 않은 타일, 2,697,728바이트. 전체 파일 감사에서 해독 실패 0; 확대 단계별 중복 포함 주요 도로선 5,297개, 보조 1,794개, 생활 111,323개. |
| 화면 | 기본 세계 모드 [Queens](../artifacts/world-roads/nyc-queens-world-z14.png), [Richmond](../artifacts/world-roads/nyc-richmond-world-z14.png), [Manhattan](../artifacts/world-integration/manhattan-world.png) z14.6 Metal 캡처 각각 실패 0. 건물·공원·해안 상세는 비어 있음. |

당시 기본 세계 모드에서 맨해튼 한 카운티 팩을 이 5개 카운티 팩으로 대체했다. 현재는 [뉴욕주 62개 카운티 팩](NY_STATE_ROADS_PROGRESS.md)이 이를 대체하며, 두 이전 결과는 비교용으로 보관한다. 도시 경계 안쪽의 도로 완전성, 경계 연결성, 명칭 중복 제거, 별도 기준점 위치 정확도와 iPhone 성능은 아직 통과하지 않았다.

이후 같은 기본 세계 모드에 [Census 수면](WORLD_WATER_PROGRESS.md), [뉴욕주 공원 경계](WORLD_PARKS_PROGRESS.md), [Queens 동부 Microsoft 건물](WORLD_BUILDINGS_PROGRESS.md)을 별도 출처 패키지로 추가했다. [네 레이어 합성 화면](../artifacts/world-integration/queens-four-layers-world.png)은 타일 오류 없이 표시됐지만, 이로써 도로 연결성이나 현장 위치 정확도가 검증된 것은 아니다.

## 세계 확장에 필요한 상태

| 원천/지역 | 확보·판정 |
|---|---|
| 한국 나주 | 공식 도로 ZIP 실증은 기존 [지역 게이트](MAP_V0_3C_GATE_AUDIT.md)에 기록. 원본 CRS와 기준점 미확인으로 S16 `NO_GO`. |
| 미국 | Census 2025의 **20개 주·DC, 1,109개 카운티 상당 원천 단위**를 실증. [세계 지도 범위 표](WORLD_MAP.md)와 각 주의 검증 기록에 채택·제외·타일 수를 남겼다. 최근 [앨라배마주 67개 카운티 도로](AL_STATE_PROGRESS.md)의 원본 338,523행 계보와 144,241개 타일을 감사했다. 다른 주 카운티는 미수집·미검증. 공식 배포 단위는 카운티 상당 단위 All Roads 파일. |
| 프랑스 | [IGN BD TOPO 3.5 파리 D075 도로 팩](FR_PARIS_ROADS_PROGRESS.md): [Licence Ouverte 2.0](https://www.data.gouv.fr/pages/legal/licences/etalab-2.0), 공식 2026-06-15 7z 원본 142,874,290바이트를 범위 요청으로 확보하고 7z CRC·파생 ZIP·RGF93/Lambert-93·속성 판정. 도로 115,758개 채택, 36,375개 제외. 지역 팩 991타일 해독 실패 0, 파리 z14 화면 실패 0. 다른 프랑스 지역과 건물·시설은 미구축이며, [파리 영구 수면](FR_PARIS_WATER_PROGRESS.md)과 [`FICTIF=Non` 여객역](FR_PARIS_STATIONS_PROGRESS.md)은 별도 팩으로 표시한다. |
| Great Britain | [OS Open Roads 52격자 구축](GB_ROADS_PROGRESS.md): 무료 OGL v3, 2026-04 전국 Shapefile 606,145,264바이트·게시 MD5 확인. 원본 3,967,825개 중 격자 간 중복 6,748개를 제거하고 3,961,077개 고유 RoadLink를 OSTN15 변환. 52개 로컬 팩의 타일 480,930개 전수 해독 오류 0. 독립 기준점·도로 연결성·실기기 성능 검증과 GB 상세 부가 레이어는 미완료. 북아일랜드 원천은 별도로 필요. |
| 북아일랜드 | [OSNI 50K Transport Lines](https://admin.opendatani.gov.uk/dataset/osni-open-data-50k-transport-transport-lines)는 OGL로 게시됐지만 이 환경에서 공식 ZIP HTTP 403, 기존 ArcGIS ZIP HTTP 400. 원본 파일 확보·검증·구축 전. |
| 캐나다 | [NRN 13개 지역 구축](CA_ROADS_PROGRESS.md): 공식 13개 주·준주 배포본의 도로선 합계 2,548,203개 채택, 거절 0. Ontario를 3개로 나눈 총 15개 팩의 실제 타일 합계 1,977,578개 전수 해독 오류 0. 지역 경계·위치 정확도·연결성·실기기 성능은 미완료. |
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

cargo run --offline -p mappa-map-data --bin audit_us_road_source -- \
  assets/map/source/public/us_tiger_2025_36*_roads.zip
cargo run --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/us_tiger_nyc_roads.toml artifacts/world-roads/nyc-five-boroughs.mgeodb
cargo run --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/us_tiger_nyc_roads.toml artifacts/world-roads/nyc-five-boroughs.mgeodb \
  artifacts/world-roads/nyc-five-boroughs.pmtiles
cargo run --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/nyc-five-boroughs.pmtiles
```
