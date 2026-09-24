# Mappa v0.3C 지역 실증 게이트 감사 — 2026-09-24

## RESULT

`NO_GO_SOURCE_COVERAGE`. **프롬프트 전체 완료 아님. 전 세계 Mappa 자체 지도 완료 아님.** 나주 약 8×10km 범위의 도로·읍면동 지명·2021 영구수면·수목 피복까지 구현했다. 건물, 철도, 공원 경계, 독립 정확도 기준점과 동일 조건 A/B가 없으므로 S16을 통과시키지 않는다. 이 문서의 `PASS`는 해당 하위 단계에 한정된다.

## BASELINE

- 시작 HEAD: `7fd0483` (나주 도로·공식 읍면동 지명)
- 수계·수목 checkpoint: `631acd2`
- 이 감사 문서 이후의 end HEAD는 `git rev-parse --short HEAD`로 확인한다.

## SOURCE DISCOVERY / LICENSE

| 후보 | 판단 | 근거/제약 |
|---|---|---|
| 나주시 2023 도로 중심선·도로면 | 사용 | 공개 파일, 이용허락범위 제한 없음. 원본 `.prj` 부재로 EPSG:5186 변환은 기관 미확인 가정 |
| 국가데이터처 SGIS 2025 Q2 읍면동 | 사용 | 공개 파일, 이용허락범위 제한 없음. `.prj`의 EPSG:5179 확인 |
| ESA WorldCover 2021 v200 | 사용 | 공개 COG, CC BY 4.0. attribution 기록. 10m **분류** 수면·수목이며 측량 수계·공원 경계가 아님 |
| 국토교통부 GIS건물통합정보 | 원본 대기 | 공공누리 제1유형 후보. Vworld 다운로드에 로그인 필요; 파일 미확보 |
| Microsoft Global ML Building Footprints | 지역 제외 | 확인한 공식 인덱스에 나주·안산 해당 타일이 없음 |
| Overture Buildings | 제외 | OSM 포함 ODbL, 이 OSM 없는 실증과 충돌 |
| Zenodo 한국 건물 후보 | 보류 | 표시된 CC BY와 상류 건물 원천의 재배포 권리 관계 미확인 |

빌드에 승인된 source entry는 5개(나주 도로 2, SGIS 1, ESA 파생 레이어 2)이고 독립 제공기관은 3곳이다. 수익 목적 이용·변경·재배포·출처표시 조건을 [`data/sources.toml`](../data/sources.toml)에서 검사한다. 출처 권리 미확인 입력은 거부한다. ESA crop·GeoJSON 해시와 재생성 명령은 [원천 준비 기록](MAP_SOURCE_PREP_WORLDCOVER.md)에 있다.

## OSM / Natural Earth

나주 `MAPPA_DATASET=canonical-proof`의 원천 사용: **OSM NO / Natural Earth NO**. 현재 기본 세계지도 모드는 여전히 Natural Earth 110m/50m와 일부 지역 Natural Earth 10m를 사용한다. 별도의 한국·서울 상세 시안에는 OSM 파일이 남아 있다. 따라서 앱 전체나 세계 전체가 OSM/NE 없는 자체 지도가 되었다고 말할 수 없다.

## S0–S16 진행상황

| 단계 | 판정 | 확인된 사실 / 미충족 게이트 |
|---|---|---|
| S0 원천 탐색 | `PASS_REGION` | 지역 후보와 권리/접근 조사 기록 |
| S1 라이선스 | `PASS_APPROVED_INPUTS` | 승인 5개 입력만 통과; 건물 파일 미확보 |
| S2 manifest | `PASS_5_INPUTS` | GeoJSON 5개 해시, ESA crop 해시, 원천 버전 고정 |
| S3 어댑터 | `PASS_5_INPUTS` | Rust 어댑터가 도로·지명·수면·수목을 canonical로 변환 |
| S4 schema | `PASS_CURRENT_KINDS` | WGS84 f64, Mappa ID, 종류·bbox·줌·이름·provenance |
| S5 geometry 검증 | `PARTIAL` | 도로면 invalid 17건 거부; 공식 수리/재조사 정책 없음 |
| S6 객체 매칭 | `NOT_STARTED` | 같은 실세계 객체를 가진 두 승인 원천 없음 |
| S7 fusion | `NOT_STARTED` | 일치 후보와 충돌 0; fusion 재현 판정 불가 |
| S8 GeoDB | `PASS_CURRENT_INPUTS` | 11,006개 feature, checksum footer, R-tree, 5,245,737B |
| S9 LOD | `PARTIAL` | z10–15 선택 규칙; 실제 형상 일반화·건물 LOD 없음 |
| S10 provenance | `PARTIAL` | 11,006개 source 추적; 수정 revision 체인·tombstone 없음 |
| S11 타일 | `PASS_CURRENT_INPUTS` | z10–15 149개 PMTiles, 742,590B, 전부 디코드 |
| S12 렌더러 | `PASS_MAC_CURRENT_INPUTS` | 로컬 PMTiles의 강·수목·도로를 Mac Metal로 캡처; 실기기 미검증 |
| S13 fidelity | `PARTIAL` | 처리 경로 도로 100개, 도로 타일 경계 미일치 0. 독립 현실 기준점·교차로 연결률·건물/수면 정확도 없음 |
| S14 건물 | `NO_GO_SOURCE_COVERAGE` | 건물 polygon 0개, 50개 fixture/완성도/IoU 불가 |
| S15 크기·속도 | `PARTIAL` | 5개 입력 Mac 1회 측정; 건물·zoom 이동·peak 메모리·iPhone 비용 없음 |
| S16 지역 통과 | `NO_GO_SOURCE_COVERAGE` | 필수 건물·철도·공원·독립 A/B 미충족 |

## CANONICAL SCHEMA / GEODB / FUSION

feature 종류는 도로 3등급·도로면·행정 지명·영구수면·수목 피복이다. 건물·철도·법정 공원은 enum 종류가 정의돼 있어도 데이터와 화면 산출물이 없다. GeoDB v1은 버전·전체 SHA-256·공간 인덱스와 별도 provenance를 가진다. 런타임 PMTiles에는 provenance를 넣지 않는다. 이번 빌드의 후보 매칭·신뢰도 등급·conflict는 모두 0이며, 단일 원천을 fusion 완료로 표기하지 않는다.

## ROADS / BUILDINGS / LOD

- 도로 중심선 3,827개, 유효 도로면 4,477개. 완성도·실제 위치 오차·교차로 연결률은 **미측정**이다. z10–15 도로 중심선 타일 경계에서 1 MVT 단위 초과 미일치 0개였다. 도로면 경계까지 통과했다는 뜻은 아니다.
- 건물 0개. 건물 존재율, centroid 거리, 면적 비, polygon IoU, 중복, 크기·렌더 비용은 **미측정**이다.
- z13에는 도로선 4,008, 도로면 4,509, 수면 92, 지명 14개가 중복 포함되어 디코드된다. z14에는 수목 2,777개, z15에는 2,988개가 중복 포함되어 디코드된다. z16은 새 상세 데이터가 아니라 z15 타일 확대다.

## SIZE / PERFORMANCE

- GeoDB 5,245,737B; PMTiles 742,590B. ESA 나주 crop 1,170,362B; 변환 수면 181,888B; 변환 수목 2,333,633B. 전체 원본 COG와 SGIS ZIP은 제품 패키지에 넣지 않는다.
- 줌별 디코드 MVT 합계와 Mac 측정은 [성능 기록](BENCHMARK_MAP_V0_3C.md)에 있다. PMTiles 압축 바이트의 레이어별 분해, MLT 비교, peak 메모리, GPU upload/prepare 단계 시간은 **미측정**이다.
- 동일 카메라 1200×720 z14.6에서 수계·수목 포함 cold load 34.789ms, pan 40프레임 p50/p95/p99 4.175/6.274/22.430ms. 이전 도로·지명 파일의 2.486/3.513/24.915ms와 비교해 중앙값과 p95가 늘었다. 1회 측정이므로 기기 성능 결론은 아니다.

## PIPELINE LOSS / VISUAL

- 100개 도로선 표본: 변환 GeoJSON → Rust 어댑터 좌표 정확 일치 100/100; 어댑터 → GeoDB 정확 일치 100/100; z15 LOD 포함 100/100. 디코드된 같은 등급 도로선까지의 거리 p50/p95/max는 0.036/0.088/0.110m이다. z15.2 화면 투영 수학 거리 p50/p95/max는 0.021/0.052/0.065px이다. MVT에 ID가 없어 다른 근접 도로와 혼동 가능하고, 실제 raster 픽셀 오차는 측정하지 않았다.
- [나주 수계·수목 화면](../artifacts/map-v0.3c/naju-water-tree-river-z14.png)은 Mac Metal 캡처다. 기존 OSM 화면과 같은 카메라·같은 레이어의 독립 A/B, pin overlay는 만들지 못했다. 본 화면이 네이버 지도 또는 실제 측량과 일치한다는 근거가 아니다.

## REGRESSION / DEFERRED / BLOCKERS

- `cargo fmt --check`, workspace Clippy `-D warnings`, `cargo test --workspace --offline`, 149개 타일 감사, 100개 도로 처리 경로 감사, 실제 Metal 캡처 통과.
- 원본 도로 CRS 기관 확인, 도로 50+/건물 50+/수면 20+ 독립 기준점, 원본 교차로 topology, 건물 fixture/건물 타일 경계, 동일 조건 A/B, correction 이력, fusion, 일반화 LOD, 모바일 측정은 남았다.
- iPhone 실기기 설치는 기존 Apple 개발 팀의 기기 등록 상한 때문에 막혀 있다. 지도 Mac 게이트와 분리해 기록한다.
- v0.3C 프롬프트 §125는 S16 통과 전 세계 건물·도로 global ingestion을 금지한다. 세계 전체 자체 DB 구축은 **시작하지 않았다**. 기본 세계지도 표시 기능은 있지만 Mappa 자체 세계 GeoDB 완성과 다르다.

## NEXT DECISION

`Adjust source strategy`. 공식 건물 SHP와 권리·좌표계·독립 기준 자료를 확보하여 이 지역 S16을 다시 검증한다. 이 파일들이 없으면 `NO_GO_SOURCE_COVERAGE`를 유지한다. S16이 실제로 통과한 뒤에만 국가별 세계 원천 matrix와 대륙 단위 구축을 시작한다.
