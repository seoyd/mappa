# 빅토리아 Vicmap 주 전체 도로 — 진행 기록

## 원천과 권리

| 항목 | 확인한 사실 |
| --- | --- |
| 공식 원천 | [DataVic Vicmap Transport - Road Line](https://discover.data.vic.gov.au/dataset/vicmap-transport-road-line). 소개 문구는 빅토리아 주 전체 도로망의 선형 데이터이며 도로·교량·연결로·터널 등을 포함한다고 명시한다. |
| 라이선스와 표기 | DataVic는 [CC BY 4.0](https://discover.data.vic.gov.au/dataset/vicmap-transport-road-line/resource/3b7eba5f-701d-4b43-b71c-edfc8351c6e7)을 표시하고 `Copyright (c) The State of Victoria, Department of Energy, Environment and Climate Action`을 요구한다. 공유조건(ShareAlike)은 없다. |
| 취득 경로 | DataVic가 연결한 [공식 WFS](https://discover.data.vic.gov.au/dataset/vicmap-transport-road-line/resource/43f18f7d-e623-483f-9025-779772409db7)의 `open-data-platform:tr_road`를 **빌드 시점에만** 읽는다. 지도 클라이언트는 WFS를 호출하지 않고 로컬 팩을 사용한다. |
| 좌표계 | 요청에 `srsName=EPSG:4326`을 지정하고 GeoJSON의 경도·위도 순서를 확인했다. 제공자의 좌표 변환과 위치 정확도를 별도 기준점으로 검증하지 않았다. |
| 갱신 | 원천 페이지의 갱신 주기는 `Continual`이다. 따라서 이 로컬 캡처는 특정 시점의 자료이지, 영구히 고정된 정부 버전은 아니다. |

## 수집·정규화 방법

`acquire_vicmap_roads`는 정렬된 `ufi` 목록을 5,000건씩 받아 중복·순서·총건수를 검사한다. 각 UFI 구간의 전체 GeoJSON을 수집해 ID가 정확히 일치하는지 확인하고 파일별 SHA-256을 기록한다. 수집 종료 때 전체 ID 목록을 다시 조회해 시작 시점과 비교한다. 이 검사는 ID 추가·삭제를 찾지만 수집 도중 **같은 ID의 좌표나 속성 변경**까지 증명하지는 못한다.

정규화 코드는 [공식 Vicmap 분류 코드](https://services-ap1.arcgis.com/w6r4LlwgJu8O0neQ/ArcGIS/rest/services/Road_network/FeatureServer/0?f=pjson)를 적용한다. `road_status=O`(개방), 차량 접근 `1`(2WD)·`2`(건기)·`3`(4WD), 도로 클래스 0–8을 차량 도로 후보로 삼고, 폐쇄 상태와 사유지 제한을 제외한다. 도로·연결로·교량·터널·여울·낮은 구간·철도 평면교차 선형을 채택한다. 보행 전용·종이도로·페리 노선과 해석하지 못한 값은 이유와 원천 ID를 남겨 별도 보관한다. 경로 탐색 가능 여부를 뜻하지 않는다.

채택한 각 선형은 지리적 중간점을 계산해 Web Mercator z8 격자에 한 번만 배정한다. 이는 외부 경계 자료 없이 로컬 다운로드 단위를 만들기 위한 분할이며, 원래 좌표를 바꾸지 않는다. 기존 `DTP Managed Roads` 팩은 Vicmap의 부분집합과 겹칠 수 있어 새 전주 팩이 검증되면 기본 카탈로그에서 교체해야 한다.

## 현재 게이트

| 게이트 | 상태 |
| --- | --- |
| 공식 권리·원천 및 코드 확인 | 통과 |
| 전체 UFI 1,271,893개 초기 목록 | 확보; 출처가 반환한 총건수와 일치 |
| 도로 형상 전수 수집·시작/종료 ID 비교 | 진행 중 |
| GeoDB·PMTiles·타일 경계 감사 | 미실행 |
| 기본 세계지도 화면·오프라인 설치 | 미실행 |
| 현장 위치 정확도·차량 연결성 | 미검증 |

완료 전에는 빅토리아 주 전체 상세도로를 구현했다고 보고하지 않는다.
