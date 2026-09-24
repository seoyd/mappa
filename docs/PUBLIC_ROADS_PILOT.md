# OSM 없는 공공 도로 데이터 시안 (2026-09-24)

## 방향과 현재 상태

Mappa의 로컬 지도 엔진과 타일 생성 코드는 Rust다. 이번 시안은 OSM 대신 지자체가 공개한 **실제 도로 중심선**을 넣어 데이터 공급원을 교체할 수 있는지 검증한다. 실행 중 지도 API 호출은 없다. 이 시안이 기존 상록수 지도를 대체하거나 전국 도로를 제공한다는 뜻은 아니다.

| 단계 | 보고된 사실 |
|---|---|
| 원본 확보 | [공공데이터포털의 나주시 도로(차도)](https://www.data.go.kr/data/15123619/fileData.do) ZIP 다운로드. 포털 표기: 2023-01-01 기준, 무료, 이용허락범위 제한 없음. |
| 원본 확인 | ZIP에 고가도로·교량·도로경계선·도로면·도로중심선·지하차도·터널 SHP가 있다. 이 시안은 도로중심선 5,736개와 도로면 7,667개를 사용한다. |
| 좌표 변환 | 원본에 `.prj`가 없어서 EPSG:5186으로 해석하여 WGS84로 변환했다. 변환 결과 경위도 범위는 126.674854–126.807885°E, 34.967209–35.071595°N으로 나주에 해당한다. 원 제공기관의 좌표계 확인은 아직 받지 못했다. |
| 타일 생성 | Rust 빌더가 도로 폭 `rdl_wid` 속성으로 표시 등급을 나누고 도로면을 z11–z12에 추가하여, Natural Earth 육지와 함께 z8–z12 로컬 PMTiles를 생성했다. 폭 20 이상은 큰 도로, 10 이상은 중간, 나머지는 작은 도로로 그린다. 이 경계값은 스타일 규칙이다. |
| 검증 | 301개 타일, 인코딩된 지형·도로 피처 31,225개, 376,732바이트. 전 타일 디코드 감사 실패 0개. Metal 오프스크린 z11.4·z13.4 화면 캡처 실패 0개. |

이 Mac의 z12.4 비동기 벤치마크는 첫 화면 타일 준비 42.333ms, 40회 이동 프레임 중앙값 1.821ms, p95 2.299ms, p99 20.057ms, 미해결 프레임 0개, 오류 0개였다. 이 수치는 해당 Mac의 데모 실행 기록이며 휴대전화 성능 측정값은 아니다.

## 출처와 재현

| 파일 | SHA-256 |
|---|---|
| `assets/map/source/public/naju_roads_20230101.zip` | `6fecafcbb19f37e6afade0e9fe755b274bcfff6a885b1f2361a60e350c982577` |
| `assets/map/source/public/naju_centerline_wgs84.geojson` | `77b3d9bfd17f1bd5acac6184b002f580c5f251e7b3b79eac90224677585f9124` |
| `assets/map/source/public/naju_surface_wgs84.geojson` | `4d3a9c2b77c45f6e61205504037158e8cb730503dd7c13b02fe9553119739a5c` |
| `assets/map/naju_public_roads.pmtiles` | `457202c979ea93300d35f663bb5939716dcee1454c726487503145d6c89a4ae7` |

원본 SHP의 `도로중심선`·`도로면` 파일을 ZIP에서 꺼내 `ogr2ogr -s_srs EPSG:5186 -t_srs EPSG:4326 -lco RFC7946=YES`로 GeoJSON으로 변환했다. 이것은 원본 변환에 사용한 **빌드 도구**이며 앱 실행에는 필요하지 않다. 변환된 GeoJSON을 포함했으므로 다음의 타일 재생성은 Rust 명령 하나로 가능하다.

```sh
cargo run -p mappa-map-data --bin build_public_roads -- assets/map/source/ne_110m_land.geojson assets/map/source/public/naju_centerline_wgs84.geojson assets/map/source/public/naju_surface_wgs84.geojson assets/map/naju_public_roads.pmtiles 8 12
cargo run -p mappa-map-data --bin audit_fixture -- assets/map/naju_public_roads.pmtiles
MAPPA_DATASET=public-naju cargo run -p mappa-map-demo -- --street-demo
```

`MAPPA_DATASET=public-naju`에서는 z0–z7에 Natural Earth, z8–z12의 나주 범위에 이 공공도로 타일을 읽는다. OSM PMTiles는 선택하지 않는다. 창에서 지도를 확대해도 z12 이후에는 같은 원본을 키워 그리므로 실제 지리 세부 사항은 추가되지 않는다. 화면의 m/px는 표시 축척이고 데이터 위치 정확도가 아니다. 원본 육지 데이터는 [Natural Earth의 공개 도메인 자료](https://www.naturalearthdata.com/about/terms-of-use/)다.

## 시안의 한계와 다음 데이터 조건

이 원본의 도로 중심선은 나주 일부 범위에만 모여 있으며 화면에는 큰 공백과 끊긴 길이 보인다. 원본에는 이 시안에서 쓸 도로명·역·공공기관 라벨과 수면·녹지 레이어가 없다. 따라서 내비게이션, 주소 검색, 상록수 지역의 실제 도로 비교에 아직 사용할 수 없다. 다음 지역으로 확장하려면 같은 이용 조건의 **최신·연속된 도로, 수면, 녹지, 건물, 역·공공기관** 데이터를 확보하고 좌표계를 원 제공기관 자료로 검증해야 한다. 출처가 다른 레이어를 합칠 때에는 위치 정합성과 누락을 실측해야 한다.

정밀도로지도는 [국토지리정보원 공개 안내](https://www.data.go.kr/data/15059912/fileData.do)에 출처표시 조건으로 소개되지만 로그인과 대용량 전송 소프트웨어가 필요하다. 국가교통정보센터 표준노드링크는 내려받기 목록을 확인했으나 [사이트 저작권 정책](https://www.its.go.kr/common/infoPolicyPage?service=copyrightPolicy)의 재배포·상업적 이용 문구를 별도로 확인해야 하므로 이 시안에는 사용하지 않았다. 지도 데이터의 권리와 Mappa가 작성한 Rust 엔진의 권리는 구분한다.
