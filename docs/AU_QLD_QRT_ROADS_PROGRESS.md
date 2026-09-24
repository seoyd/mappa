# 퀸즐랜드 Roads and Tracks 도로 실증 — 2026-09-25

## 원천과 범위

[Queensland Roads and Tracks](https://www.data.qld.gov.au/dataset/queensland-roads-and-tracks)는 퀸즐랜드 정부의 CC BY 4.0 도로·트랙 자료다. [공식 데이터 사전](https://www.qld.gov.au/__data/assets/pdf_file/0030/563457/roads-tracks-data-dictionary.pdf)은 도형을 **대략적인 중심선**으로 설명하고 내비게이션용 제품이 아니라고 명시한다. Mappa는 [공식 Roads and Tracks 선형 서비스](https://spatial-gis.information.qld.gov.au/arcgis/rest/services/Transportation/RoadsAndTracks/MapServer/10)를 **빌드 시에만** 읽어 로컬 스냅샷과 팩을 만들었다. 앱 실행 중 이 서비스 또는 외부 지도 API를 호출하지 않는다.

| 검증 항목 | 보고된 결과 |
|---|---|
| 권리 | 자료 페이지의 CC BY 4.0. `© State of Queensland (Department of Natural Resources and Mines, Manufacturing and Regional and Rural Development) 2025`를 출처 manifest·팩·화면 라벨에 보존. 공유조건 없음. |
| 원본 수집 | 2026-09-25에 서버가 반환한 `objectid` **562,728개**. 처음과 마지막 ID 목록이 같았고, 서로 다른 ID 562,728개를 **563개 GeoJSON 페이지**에서 각각 정확히 한 번씩 확인했다. 각 페이지의 크기와 SHA-256은 로컬 `data/local/au_qld/qrt-2026-09-25/snapshot.toml`에 기록했다. 스냅샷 manifest SHA-256은 `38ae4db69a21c0d258088f583bb20b821f50a0b394569df70068073b59c50fec`; `ids.json` SHA-256은 `06942349def433885fa1dd3f901ce50ad06b8f96db2debdc0eb9b413535f2cf6`. 원본 약 924 MiB는 Git에서 제외했다. |
| 좌표계 | 공식 서비스는 Web Mercator 102100이고 수집 요청은 `outSR=4326`으로 GeoJSON 경위도를 받았다. 서버 재투영 결과를 보존했으며 독립 기준점으로 WGS84 위치 정확도를 검증하지 않았다. |
| 원본 종류 | `class`별 Local 315,281, Track 105,985, Secondary 43,774, Connector 41,561, Restricted 21,934, Highway 16,894, Walkway 12,358, Bikeway 2,346, Motorway 2,138, Busway 373, Ferry 63, Mall 21개. 모두 LineString. |
| 정규화 | 공식 데이터 사전의 차량 도로 분류 Motorway·Highway·Secondary·Connector·Local 중 현재(`record_status=C`)·운영(`op_status_ind=Operational`) 중인 선형을 채택했다. Private·Restricted 접근과 Crossover를 별도로 제외했다. **411,847개 채택, 150,881개 제외**; [압축 거절 기록](../artifacts/world-roads/au-qld/rejected.json.gz)에 원천 ID와 이유를 보존. 도로 이름과 원본 속성·도형 해시를 피처 계보에 기록. |
| 지역 분할 | 원본 `lga_name_left`, 없으면 `lga_name_right`로 할당한 **79개 지역**. 경계 걸친 선형은 한 팩에만 배치한다. 다른 팩과의 연결성은 아직 감사하지 않았다. GeoDB 79개 약 184 MiB는 로컬 빌드 산출물로 Git에서 제외. |
| PMTiles | [79개 지역 카탈로그](../assets/map/au_qld_qrt_regional_packs.toml)의 합계 **84,241,946바이트**, z10–15 비어 있지 않은 타일 **303,681개** 전수 해독. 팩 내부의 정확한 경계 일치 905,156건, 확정 불일치 0건, 꼭짓점 모호 1건, 양자화 모호 0건. 각 팩의 [감사 로그](../artifacts/world-roads/au-qld/)를 확인한다. |
| 배포 | 235개 전체 재고표 해시 검증과 새 79개 팩의 별도 설치가 통과했다. GitHub 비공개 초안 `map-packs-2026-09-25-au-qld-qrt`의 79개 서버 자산 이름·크기·SHA-256도 로컬 재고표와 일치한다. 235개 팩의 정적 주소를 재고표에 고정했지만 릴리스가 공개되기 전에는 일반 다운로드가 되지 않는다. |
| 화면 | 기본 세계 카탈로그 **235개 팩**의 z14 읽기 테스트 통과. 이 Mac이 잠긴 동안 Metal 어댑터를 찾지 못해 브리즈번 화면 캡처는 아직 성공하지 않았다. 이것을 화면 품질 통과로 기록하지 않는다. |

수집 시작·종료 시 `objectid` 목록 일치는 수집 도중 **기존 ID의 속성·좌표가 바뀌지 않았음**을 증명하지 않는다. 원본의 주간 다운로드와 서비스가 갱신될 수 있으므로 같은 URL을 다시 읽어도 이 스냅샷 SHA가 재현된다고 보장할 수 없다. 현장 기준점 위치 정확도, 전체 도로 완전성, 노선 연결성, 접근 권한을 고려한 길찾기, 팩 사이 이음, iPhone 성능은 미검증이다. 트랙·보행로·자전거로·접근 제한 도로는 별도 표현 규칙이 생길 때까지 화면 도로 레이어에 넣지 않았다.

## 재현 절차

```sh
cargo run --offline -p mappa-map-acquire --bin acquire_qld_roads -- data/local/au_qld/qrt-2026-09-25
cargo run --release --offline -p mappa-map-data --bin build_au_qld_qrt_roads -- \
  data/local/au_qld/qrt-2026-09-25/snapshot.toml 2026-09-25 artifacts/world-roads/au-qld data
```

첫 명령에는 공식 서비스에 접근할 네트워크가 필요하다. 같은 원본이 유지되는지 수집 종료 시 ID를 재확인하고, 저장한 각 페이지의 SHA-256은 두 번째 명령에서 다시 검증한다. 이후 각 `au_qld_qrt_roads_*.toml`의 해당 GeoDB에 `build_canonical_tiles`, 생성된 PMTiles에 `audit_canonical_tiles`를 실행하고 `make_au_qld_qrt_catalog`로 카탈로그를 만든다.
