# Mappa 자체 기록 지도 — 2026-09-24

## 방향

지도 원본의 소유권과 실행 비용을 분리한다. Rust 렌더러가 로컬 파일을 읽으면 지도 타일 서버 비용은 들지 않는다. 그러나 정확한 길·건물·행정 경계는 실제 측량이나 별도 데이터 없이 코드만으로 만들 수 없다. `MAPPA_DATASET=first-party` 모드는 **Mappa가 현장에서 기록한 위치만** 표시하고, 기록하지 않은 곳은 중립색 빈 화면으로 둔다. 기본 실행은 [퍼블릭 도메인 원본으로 만든 세계지도](WORLD_MAP.md)를 보여준다. 표준 좌표 투영과 범용 Rust 라이브러리는 사용하지만 OSM·Natural Earth·공공데이터의 지형, 도로, 이름은 직접 기록 모드에 넣지 않는다.

## 보고된 상태

| 항목 | 확인된 사실 |
|---|---|
| 자체 원본 | `assets/map/first_party/survey.geojson`: 육지·수역 0건, 경계 0건, 도로 0건, 장소 0건 |
| 타일 | `assets/map/first_party.pmtiles`: 생성 타일 0개. 빈 PMTiles 파일을 실제 리더로 열고 타일 부재를 확인함 |
| 화면 | Metal 오프스크린 캡처 `artifacts/first-party/empty.png` 성공, 타일 오류 0건. 중립색 화면, 상태·축척 표시 |
| 입력 검증 | 출처 필드 누락·열린 면 도형을 거절하고, 잘못된 입력이 기존 원본을 덮어쓰지 않으며, 임시 육지·수역·경계·도로·장소를 타일로 왕복하는 자체 기록 경로 자동 테스트 6개 통과 |
| 현장 실측 | 아직 0건. iPhone에서 직접 GPS 기록·정확도 확인은 수행하지 못함 |

현재 파일에 없는 지리 정보는 추정해서 추가하지 않는다. 테스트에서 쓴 좌표는 임시 파일에만 만들고 제거했으며 배포용 지도에는 들어가지 않는다. `origin: mappa-field-survey`는 입력자가 주장하는 출처 표기이며 독립적인 현장 증거 검증은 아니다.

`boundary`는 현장에서 확인한 선형 경계의 기록 형식이다. 국가·행정구역의 **법적 경계**는 GPS로 걷거나 표지판을 보는 것만으로 확정할 수 없다. 공식 경계 원본을 쓰지 않는 조건에서는 해당 경계를 정확한 국경으로 표시하지 않는다. 마찬가지로 자체 기록이 없는 대륙 윤곽, 도로, 도시를 다른 지도를 보고 옮기면 100% 직접 제작 원본이라는 조건을 만족하지 않는다.

## Rust 작업 흐름

```bash
cargo run -p mappa-map-data --bin first_party_map -- inspect assets/map/first_party/survey.geojson
cargo run -p mappa-map-data --bin first_party_map -- build assets/map/first_party/survey.geojson assets/map/first_party.pmtiles
MAPPA_DATASET=first-party cargo run -p mappa-map-demo -- --street-demo
```

현장에서 직접 확인한 좌표를 기록할 때만 다음 형식을 사용한다. 좌표는 WGS84 경도·위도이고 `accuracy_m`은 기록 당시 기기 또는 관측자가 보고한 오차 반경이다. `observed_at`은 UTC 시각이다. `add-place`는 `city`, `station`, `civic`을, `add-line`은 `boundary`, `road-major`, `road-collector`, `road-local`을, `add-area`는 `land`, `water`를 허용한다. 선에는 실제로 관측한 두 점 이상을 순서대로 입력한다. 면의 외곽선은 첫 좌표를 마지막에 다시 적어 닫아야 한다. 면의 내부를 실측하지 않은 상태에서 관측 구역보다 넓게 칠하지 않는다.

```bash
cargo run -p mappa-map-data --bin first_party_map -- add-place assets/map/first_party/survey.geojson station "현장 확인 이름" <경도> <위도> <정확도_m> 2026-09-24T00:00:00Z
cargo run -p mappa-map-data --bin first_party_map -- add-line assets/map/first_party/survey.geojson road-local "현장 확인 도로" <정확도_m> 2026-09-24T00:00:00Z <경도,위도> <경도,위도>
cargo run -p mappa-map-data --bin first_party_map -- add-area assets/map/first_party/survey.geojson water "현장 확인 수역" <정확도_m> 2026-09-24T00:00:00Z <경도,위도> <경도,위도> <경도,위도> <처음과 같은 경도,위도>
```

입력 CLI는 `method: field-note`로 표기한다. `add-road`는 기존 스크립트를 위한 `add-line` 별칭이다. 실제 기기 GPS를 연결한 기록은 앞으로 `method: device-gps`로 구분한다. 정확도, 관측 시각, 도형 종류, 좌표 범위 및 면의 유효성을 검사하고 오류가 있으면 기존 원본 파일을 교체하지 않는다. 기록이 있으면 타일 범위를 입력 좌표의 범위로 기록한다. 타일을 다시 빌드해야 화면에 반영된다. `MAPPA_FIRST_PARTY_FILE`로 자체 제작 PMTiles의 경로만 바꿀 수 있다.

직접 기록 모드는 자체 제작 PMTiles 한 파일만 열고 외부 지도 원본으로 돌아가지 않는다. 기본 세계지도와는 아직 합쳐서 표시하지 않는다. 과거 외부 데이터 비교 시안은 `MAPPA_DATASET=legacy-osm` 또는 `MAPPA_DATASET=public-naju`를 명시해야 한다.

현재 자체 타일은 z0–z14로 만든다. 한 번의 빌드는 최대 20,000개 타일이며 세계 규모 데이터는 지역별 분할·타일 병합 작업이 추가로 필요하다. 화면은 z16까지 확대되지만 z14 이후에 새로운 측량 세부 정보가 생기지는 않는다. 오른쪽 아래 m/px는 화면 축척이며 관측 위치 정확도가 아니다.

## 다음 실제 데이터 작업

1. 기기 서명·실기기 접근이 가능해지면 Rust 기기 클라이언트에서 위치와 수평 정확도·시각을 직접 기록한다. 지금 단계에서 수집 완료로 보고하지 않는다.
2. 한 구역의 도로를 연속으로 직접 걸어 기록하고, 같은 구간을 다시 측정해 위치 차이·누락·끊김을 계산한다. 시설 이름은 현장 표지와 기록 시각을 함께 검증한다.
3. 수집 구역의 관측한 길이, 검증한 도로 수, 위치 오차 분포, 마지막 갱신일로 범위를 보고한다. 전국·세계 커버리지나 내비게이션 사용 가능 여부는 실측 전에는 주장하지 않는다.

로컬 지도 조회에는 지도 타일 서버가 필요 없지만, 넓은 지역을 직접 조사하고 최신 상태로 유지하려면 인력·기기·검증 비용이 든다. 현재 자체 기록 지도는 지리 콘텐츠가 없어 실제 길찾기나 네이버 지도와의 정확도 비교 대상이 아니다.
