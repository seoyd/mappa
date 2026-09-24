# 세계 상세지도 구축 현황 — 2026-09-24

## 판정

**세계지도 미완성.** 세계 개략 지도는 전 지역에 있고, 이번에는 세계 상세 데이터의 첫 실증으로 Microsoft 건물 원본 한 구역(모나코 분할 파일)을 MappaGeoDB → PMTiles → Rust/Metal 화면까지 연결했다. 도로·철도·역·공공기관·주소의 세계 상세 데이터는 구축되지 않았다. 지도 앱은 이 실증에서도 로컬 파일만 읽는다. 위치·형상은 Microsoft 공개 원본에서 왔으며 Mappa가 직접 측량한 것이 아니다.

## 실제로 확보한 데이터

| 항목 | 확인된 사실 |
|---|---|
| 원천 | [Microsoft Global ML Building Footprints](https://github.com/microsoft/GlobalMLBuildingFootprints), 2026-08-13 릴리스, Monaco / z9 quadkey `120223030` 한 파일 |
| 배포 조건 | [CDLA Permissive 2.0](https://cdla.dev/permissive-2-0/); [라이선스 전문](../data/CDLA-Permissive-2.0.txt) 포함, 화면·타일에 출처 기재 |
| 전체 원천 인덱스 | [공식 CSV](https://bfppub.blob.core.windows.net/%24web/2026-08-13/dataset-links.csv) SHA-256 `de61b569b0c23c364d61173632d8c5487ea30bf20912c55b9fa1ab4a89d01148`; 저장본 [gzip](../data/ms_buildings_index_2026-08-13.csv.gz) |
| 인덱스 구조 | 30,340행, 고유 z9 quadkey 25,606개, 제공자 지역명 225개, z3 세계 셀 64개 중 33개에 하나 이상의 파일 존재. **데이터 존재 지표이지 건물 완성률이 아님.** |
| 전체 입력 규모 추정 | 인덱스의 반올림된 `Size` 항목 합계 약 **117.3 GB 압축**. 실제 전송량·압축 해제 크기·중복 제거 후 크기는 아직 측정하지 않음. |
| 나주 확인 | 나주 좌표가 속한 z9 quadkey `132112120`의 파일 0개. 다른 건물 원천이 필요함. |
| 모나코 파일 | [고정 원본](../assets/map/source/public/ms_monaco_120223030_2026-08-13.csv.gz) 908개 GeoJSONL 레코드, SHA-256 `fc501fe4c0f52fb2c6ad0001d8a7ba9ed9cbe1fc7ca1b5dd5641fbb686274eee` |
| 정규화 | 908개 승인, 거절 0개, 출처 908개, GeoDB 456,886바이트. 원본의 건물 ID가 없어 원본 레코드 SHA-256과 원천 ID로 Mappa ID 생성. |
| 타일 | 14개 비어 있지 않은 타일, 확대 단계별 중복 포함 건물 1,900개, PMTiles 62,116바이트, 전체 읽기 오류 0개. |
| 화면 | [모나코 건물 z14.6](../artifacts/world-buildings/monaco-z14.png). 도로·해안·건물 외 지역이 빈 것은 해당 상세 원천을 넣지 않았기 때문. Mac Metal 캡처, 타일 오류 0개. |

Microsoft는 전 세계 약 14억 건물 탐지 결과를 공개한다고 설명하지만, 지역·영상 시점·탐지 품질에 공백이 있다. 인덱스의 225개 제공자 지역명은 국가 225개 완성이라는 뜻이 아니다. 한 z9 셀에 여러 지역 파일이 겹칠 수 있어 행 수 역시 면적·정확도·건물 수의 대체 지표가 아니다. 모나코 좌표의 같은 셀에도 Europe·France·Italy·Monaco 파일 4개가 있으며, 이번 GeoDB에는 **Monaco 파일만** 넣었다.

## 구축 방식과 남은 큰 장애물

1. Rust 빌드 도구가 원본 SHA-256, 라이선스 manifest, 각 도형의 유효성을 검사하고, 거절 도형을 별도 JSON으로 남긴다. 정규화한 건물은 `Building` 종류와 원본 레코드 해시·원천 버전을 가진다.
2. Mappa 타일 빌더는 건물 폴리곤을 `building` 레이어로 인코딩하고 Rust 디코더·Metal 렌더러가 이를 별도 색으로 그린다. `MAPPA_DATASET=canonical-proof`와 `MAPPA_CANONICAL_FILE`로 이 실증 파일을 열 수 있다. 기본 세계지도 파일에는 아직 결합하지 않았다.
3. 현 GeoDB는 파일당 최대 100만 피처이고 빌드 중 피처를 메모리에 모은다. 약 14억 건물을 한 파일로 넣을 수 없다. z9 등 공간 단위 샤딩, 중복 구역 병합, 갱신·배포 인덱스가 필요하다. 모든 원본을 내려받거나 독립 정확도 검사를 수행하지 않았다.
4. 도로는 별도 원천·연결성 검증이 필요하다. [Microsoft Road Detections](https://github.com/microsoft/RoadDetections)는 전 세계 도로 탐지 자료를 공개하지만 **ODbL**이다. 현 canonical manifest의 공유조건 금지 게이트와 호환되지 않으므로 이 GeoDB에 섞지 않았다. OSM과 ODbL 소스의 독립 레이어·고지·배포 방식을 먼저 설계해야 한다. [Overture Transportation](https://docs.overturemaps.org/attribution/)도 ODbL이다.
5. [Overture Places](https://docs.overturemaps.org/guides/places/)는 OSM을 포함하지 않고 다수의 허용형 라이선스 원천을 사용하나, 지명·시설의 소스별 권리와 중복 제거·좌표 검증·원본 파일 확보가 필요하다.

## 재현

```bash
cargo run --offline -p mappa-map-data --bin audit_building_index -- \
  data/ms_buildings_index_2026-08-13.csv.gz 7.42 43.73
cargo run --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/ms_monaco_buildings.toml artifacts/world-buildings/monaco.mgeodb
cargo run --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/ms_monaco_buildings.toml artifacts/world-buildings/monaco.mgeodb \
  artifacts/world-buildings/monaco.pmtiles
cargo run --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-buildings/monaco.pmtiles
MAPPA_DATASET=canonical-proof \
MAPPA_CANONICAL_FILE=artifacts/world-buildings/monaco.pmtiles \
cargo run --offline -p mappa-map-demo -- --capture \
  7.42 43.73 14.6 artifacts/world-buildings/monaco-z14.png
```

## 현재 완료 기준 대비

| 범위 | 상태 |
|---|---|
| 세계 저배율 해안·국경·일부 강/도로 | 표시 가능, Natural Earth 개략 원천 |
| 세계 고배율 건물 | 어댑터와 모나코 단일 분할 실증 완료; 세계 구축 미완료 |
| 세계 실제 도로·철도·역·시설 | 미구축 |
| 국가별 원천 커버리지·독립 위치 정확도 | 미검증 |
| 세계 다운로드·갱신·샤딩·휴대폰 성능 | 미구축·미검증 |

원천 파일 사용료나 런타임 지도 API 비용은 이번 실증에서 발생하지 않았다. 전 세계 원천 다운로드·보관·갱신에 필요한 저장 공간과 네트워크 비용이 항상 0이라는 뜻은 아니다.
