# Mappa Spatial Pack 실험 — 2026-09-24

## 판정

**PARTIAL / 기본 PMTiles 대체 보류.** 나주 `MappaGeoDB`의 같은 11,006개 피처를 실험 파일 `naju-roads.msp`로 만들고 Rust mmap 리더·공간 질의·기존 Metal 렌더러까지 연결했다. 원본 좌표는 GeoDB에 남고 MSP는 1e-7도 단위 정수 좌표를 delta + zigzag varint로 저장한다. 지명은 문자열 테이블, 종류·확대 범위는 고정 스키마 바이트로 저장한다. geometry는 피처당 한 번, z12 공간 셀에는 피처 순번만 저장한다. **z10–z15용 geometry refinement는 아직 없다.** 모든 줌에서 저장된 전체 geometry를 사용하되, 원본의 피처별 확대 범위는 지킨다.

이 실험은 기존 기본 지도나 나주 canonical PMTiles를 교체하지 않는다. 현재 MSP는 화면 하나를 질의해 단일 GPU 메쉬로 그리는 별도 `--msp-capture` 경로다. 연속 팬용 메쉬 캐시·GPU 교체, 라벨 충돌 정책, 휴대폰 로딩 성능은 아직 검증되지 않았다.

## 구조와 실제 데이터

```text
승인 원본 → MappaGeoDB → Rust MSP compiler → .msp
                                           ├── checksum·mmap reader
                                           ├── z12 cell → feature ordinal 참조
                                           ├── bbox + zoom 질의
                                           └── viewport geometry → 기존 Rust/Metal renderer
```

| 항목 | 실험 결과 |
|---|---:|
| 입력 GeoDB | 나주 원본 5개, 11,006개 피처; OSM/NE 입력 없음 |
| 실제 피처 | 도로 중심선 3,827, 도로면 4,477, 영구수면 79, 수목 피복 2,609, 읍면동 지명 14 |
| 비교 좌표 | 143,704개 |
| GeoDB → MSP 최대 정수화 거리 | 0.007117m; 근사 지구 반경·위도 보정으로 계산 |
| MSP / 기존 PMTiles 크기 | 1,112,776 / 742,590바이트, **1.499배** |
| 독립 빌드 2회 SHA-256 | 두 파일 모두 `f2b68c50bee554c23e6113250a3b9ffde68560e154a4601496bf92d12a61d0ec` |
| 공간 질의 | 전체·남서·북동 영역의 z10–z15 피처 ID 집합이 GeoDB 질의와 같음 |
| 화면 | [MSP z14.6](../artifacts/map-msp/naju-msp-z14.png) / [PMTiles z14.6](../artifacts/map-msp/naju-mvt-z14.png), [MSP z15.2](../artifacts/map-msp/naju-msp-z15.png) / [PMTiles z15.2](../artifacts/map-msp/naju-mvt-z15.png) |

현재 MSP에는 건물·철도 피처가 **0개**다. 건물 파라미터 압축과 도로 라우팅 topology가 효과적이라고 주장할 수 없다. 나주 공식 도로 CRS와 독립 위치 기준점도 미확인이라 이 파일의 0.007117m는 **GeoDB 좌표 대비 저장 오차**일 뿐 실제 도로 위치 정확도가 아니다.

## 성능과 실패한 목표

제안서의 연구 목표인 `MSP ≤ PMTiles의 70%`를 통과하지 못했다. geometry와 공간 참조의 중복을 줄였어도 기존 PMTiles는 gzip MVT를 사용한다. 이 초기 MSP는 독립 레코드의 정수 metadata·좌표를 별도 압축하지 않아 파일이 더 크다. 전체 MSP 파일을 임시 gzip level 6으로 압축한 크기도 753,640바이트로, 기존 742,590바이트보다 컸다. 전체 파일 압축은 임의 피처 접근을 잃으므로 제품 형식으로 적용하지 않았다.

Mac 1200×720 z14.6 단일 MSP 캡처의 관측 시간은 viewport 질의·변환 **1.85ms**, 메쉬 준비 **4.38ms**, GPU 업로드 **0.68ms**였다. 같은 카메라의 기존 PMTiles 3회 캡처에서 타일 lookup **6.26–7.76ms**, decode **3.44–4.26ms**, 메쉬 준비 **11.81–15.19ms**, 업로드 **5.55–7.85ms**였다. 기존 경로는 화면 밖 여유 타일까지 읽고 MSP는 화면 bbox만 읽어 작업량이 다르다. 이 숫자로 MSP의 성능 우위를 판정하지 않는다. warm pan, p95 프레임, peak 메모리, iPhone 측정은 없다.

## 채택 전에 필요한 실험

1. 공간 청크별 압축과 더 작은 bbox/속성 표현을 **임의 접근을 유지한 채** 시험하고 크기·cold decode·peak 메모리를 다시 비교한다.
2. 같은 viewport/여유 영역/레이어 수로 두 경로의 cold load, warm pan, p95 프레임을 측정한다.
3. 빌드 시 계산한 누적 vertex refinement를 넣고 z10–z15 화면 형상과 교차로 연결이 기존 결과보다 나빠지지 않는지 확인한다.
4. 도로 topology는 원본의 교차·고가·터널 연결성을 먼저 검증한 뒤 구축한다. 건물 전용 블록은 승인 건물 원본이 확보된 뒤 시험한다.
5. 같은 조건에서 MSP가 크기와 속도 및 화면 품질 게이트를 통과할 때만 기본 PMTiles 경로 교체를 검토한다.

## 재현 명령

```bash
cargo run --release --offline -p mappa-map-data --bin build_spatial_pack -- \
  data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb artifacts/map-v0.3c/naju-roads.msp
cargo run --release --offline -p mappa-map-data --bin audit_spatial_pack -- \
  artifacts/map-v0.3c/naju-roads.mgeodb artifacts/map-v0.3c/naju-roads.msp artifacts/map-v0.3c/naju-roads.pmtiles
cargo run --offline -p mappa-map-demo -- --msp-capture \
  artifacts/map-v0.3c/naju-roads.msp 126.715 35.025 14.6 artifacts/map-msp/naju-msp-z14.png
```

MSP는 `MAPPASPK` magic, 버전, 원본 지역, section offsets, 피처 레코드, 출처·이름 테이블, z12 셀 참조와 SHA-256 footer로 구성된다. 출처 문구는 승인 manifest에서 생성해 파일과 캡처에 보존한다. 파일은 빌드 후 불변으로 취급한다.
