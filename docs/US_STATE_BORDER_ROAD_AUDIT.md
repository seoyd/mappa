# 미국 주 경계 도로 끝점 진단 — 2026-09-25

## 목적과 입력

기존 `audit_canonical_tiles`는 **한 팩 안의 타일 경계**를 검사한다. 다른 주 팩의 도로가 실제 주 경계에서 이어지는지는 검사하지 않는다. 새 Rust [경계 끝점 감사 도구](../crates/mappa-map-data/src/bin/audit_us_state_border_endpoints.rs)는 공식 [Census 2025 State and Equivalent ZIP](https://www2.census.gov/geo/tiger/TIGER2025/STATE/tl_2025_us_state.zip)의 두 주 폴리곤에서 좌표열이 정확히 같은 경계 선분을 추출하고, 그 선분 20m 이내의 두 GeoDB 도로 끝점을 비교한다. 원본 ZIP SHA-256은 `59a220888a8d9be8117c4fcd38f542bd02d81abf0d198c78113595ad540dd957`이며 7개 ZIP 멤버 CRC, `.prj` NAD83, `STATEFP`를 확인했다. 사용 조건은 [Census 2025 기술 문서](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)를 따른다. 지도 런타임은 이 파일이나 Census API에 의존하지 않는다.

`같은 끝점`은 상대 GeoDB 끝점이 0.1m 이내인 경우다. 끝점이 없어도 상대 도로선 위 0.1m 이내면 화면 선형은 이어질 가능성이 있다. `후보 공백`은 반대쪽 끝점과 도로선 모두 20m 이내에 없는 끝점이다. 거리 계산은 점의 위도에서 국소 경도 축척을 적용한 근사치다. 이 구분은 **진단**이며 운행 가능한 교차로 또는 실제 도로 단절의 최종 판정이 아니다.

## 관측 결과

| 주 경계 | 동일 경계 선분 | 방향 | 경계 근처 끝점 | 같은 끝점 ≤0.1m | 상대 끝점 없음 ≤20m | 그중 상대 선 위 ≤0.1m | 후보 공백 >20m |
|---|---:|---|---:|---:|---:|---:|---:|
| KS–MO | 4,304 | KS→MO | 408 | 283 | 116 | 107 | 7 |
| KS–MO | 4,304 | MO→KS | 404 | 283 | 109 | 89 | 17 |
| MO–IA | 3,016 | MO→IA | 285 | 241 | 36 | 32 | 3 |
| MO–IA | 3,016 | IA→MO | 314 | 241 | 66 | 49 | 17 |
| MO–IL | 3,929 | MO→IL | 39 | 35 | 3 | 0 | 3 |
| MO–IL | 3,929 | IL→MO | 37 | 35 | 1 | 0 | 1 |
| NE–KS | 4,210 | NE→KS | 566 | 387 | 167 | 153 | 7 |
| NE–KS | 4,210 | KS→NE | 580 | 387 | 177 | 160 | 7 |
| NE–IA | 4,050 | NE→IA | 43 | 39 | 4 | 0 | 4 |
| NE–IA | 4,050 | IA→NE | 42 | 39 | 1 | 0 | 0 |
| KS–OK | 3,706 | KS→OK | 546 | 353 | 178 | 167 | 8 |
| KS–OK | 3,706 | OK→KS | 595 | 353 | 226 | 169 | 47 |
| CO–KS | 1,075 | CO→KS | 155 | 135 | 17 | 7 | 10 |
| CO–KS | 1,075 | KS→CO | 179 | 135 | 36 | 19 | 17 |
| CO–NE | 1,388 | CO→NE | 203 | 137 | 58 | 39 | 17 |
| CO–NE | 1,388 | NE→CO | 197 | 137 | 50 | 26 | 23 |
| CO–OK | 136 | CO→OK | 21 | 20 | 0 | 0 | 0 |
| CO–OK | 136 | OK→CO | 41 | 20 | 20 | 3 | 17 |
| SD–NE | 2,796 | SD→NE | 177 | 140 | 35 | 17 | 18 |
| SD–NE | 2,796 | NE→SD | 182 | 140 | 37 | 10 | 27 |
| SD–IA | 9,804 | SD→IA | 26 | 18 | 8 | 7 | 1 |
| SD–IA | 9,804 | IA→SD | 27 | 18 | 9 | 5 | 4 |
| WY–CO | 1,868 | WY→CO | 182 | 125 | 48 | 1 | 46 |
| WY–CO | 1,868 | CO→WY | 159 | 125 | 30 | 1 | 28 |
| WY–NE | 867 | WY→NE | 120 | 91 | 29 | 10 | 16 |
| WY–NE | 867 | NE→WY | 154 | 91 | 60 | 19 | 40 |
| WY–SD | 1,116 | WY→SD | 102 | 69 | 33 | 5 | 28 |
| WY–SD | 1,116 | SD→WY | 81 | 69 | 10 | 4 | 6 |
| MT–WY | 1,932 | MT→WY | 147 | 106 | 33 | 0 | 33 |
| MT–WY | 1,932 | WY→MT | 161 | 106 | 50 | 1 | 48 |
| MT–SD | 341 | MT→SD | 19 | 16 | 2 | 0 | 2 |
| MT–SD | 341 | SD→MT | 18 | 16 | 2 | 1 | 1 |
| ND–MT | 2,026 | ND→MT | 244 | 147 | 88 | 35 | 51 |
| ND–MT | 2,026 | MT→ND | 196 | 147 | 39 | 16 | 23 |
| ND–SD | 3,286 | ND→SD | 376 | 247 | 123 | 82 | 40 |
| ND–SD | 3,286 | SD→ND | 349 | 247 | 100 | 65 | 35 |
| MN–IA | 3,607 | MN→IA | 418 | 236 | 176 | 163 | 11 |
| MN–IA | 3,607 | IA→MN | 381 | 236 | 141 | 134 | 4 |
| MN–ND | 26,565 | MN→ND | 42 | 38 | 3 | 0 | 3 |
| MN–ND | 26,565 | ND→MN | 42 | 38 | 4 | 0 | 4 |
| MN–SD | 2,610 | MN→SD | 183 | 152 | 28 | 19 | 9 |
| MN–SD | 2,610 | SD→MN | 180 | 152 | 26 | 16 | 10 |
| MN–WI | 4,846 | MN→WI | 52 | 46 | 6 | 5 | 1 |
| MN–WI | 4,846 | WI→MN | 54 | 46 | 7 | 4 | 2 |
| ID–MT | 21,370 | ID→MT | 436 | 309 | 101 | 1 | 98 |
| ID–MT | 21,370 | MT→ID | 371 | 309 | 42 | 1 | 39 |
| ID–UT | 778 | ID→UT | 117 | 87 | 26 | 9 | 16 |
| ID–UT | 778 | UT→ID | 124 | 87 | 29 | 6 | 23 |
| ID–WY | 1,290 | ID→WY | 174 | 126 | 42 | 24 | 15 |
| ID–WY | 1,290 | WY→ID | 167 | 126 | 34 | 17 | 17 |
| UT–CO | 1,467 | UT→CO | 224 | 159 | 58 | 2 | 56 |
| UT–CO | 1,467 | CO→UT | 189 | 159 | 23 | 5 | 18 |
| UT–WY | 1,517 | UT→WY | 146 | 108 | 34 | 2 | 32 |
| UT–WY | 1,517 | WY→UT | 127 | 108 | 17 | 3 | 12 |
| AZ–NM | 2,116 | AZ→NM | 282 | 201 | 51 | 8 | 42 |
| AZ–NM | 2,116 | NM→AZ | 278 | 201 | 52 | 16 | 35 |
| AZ–NV | 2,174 | AZ→NV | 30 | 23 | 6 | 0 | 6 |
| AZ–NV | 2,174 | NV→AZ | 25 | 23 | 1 | 0 | 1 |
| AZ–UT | 1,760 | AZ→UT | 172 | 125 | 34 | 11 | 22 |
| AZ–UT | 1,760 | UT→AZ | 176 | 125 | 41 | 14 | 27 |
| NM–CO | 1,584 | NM→CO | 259 | 167 | 86 | 0 | 84 |
| NM–CO | 1,584 | CO→NM | 206 | 167 | 32 | 3 | 29 |
| NM–OK | 231 | NM→OK | 29 | 10 | 18 | 12 | 5 |
| NM–OK | 231 | OK→NM | 21 | 10 | 10 | 9 | 1 |
| NV–ID | 1,071 | NV→ID | 107 | 90 | 11 | 3 | 8 |
| NV–ID | 1,071 | ID→NV | 106 | 90 | 15 | 0 | 15 |
| NV–UT | 1,972 | NV→UT | 198 | 175 | 13 | 2 | 10 |
| NV–UT | 1,972 | UT→NV | 228 | 175 | 43 | 4 | 38 |

원시 출력과 후보 좌표 표본은 [KS–MO](../artifacts/world-roads/ks-state/border-endpoints-mo.log), [MO–IA](../artifacts/world-roads/mo-state/border-endpoints-ia.log), [MO–IL](../artifacts/world-roads/mo-state/border-endpoints-il.log), [NE–KS](../artifacts/world-roads/ne-state/border-endpoints-ks.log), [NE–IA](../artifacts/world-roads/ne-state/border-endpoints-ia.log), [KS–OK](../artifacts/world-roads/ok-state/border-endpoints-ks.log)에 기록했다. `--details` 옵션을 적용한 [KS–OK 후보 원본 계보](../artifacts/world-roads/ok-state/border-endpoints-ks-detailed.log)는 후보 끝점 55개의 관련 원본 도로 행 67개와 도로 분류·이름을 기록했다. 67개는 모두 `RoadResidential`이며, 이는 통행 가능성과 실제 연결 여부의 판정이 아니다. `상대 끝점 없음`과 `상대 선 위`의 차이는 두 주 원천이 같은 도로를 다른 지점에서 분할할 수 있음을 보여준다. `후보 공백`에는 강가·주 경계에서 끝나는 정상 도로가 포함될 수 있다. 선을 임의로 이어 붙이지 않았다.

## KS–OK 원본 행 대조

새 Rust [원본 도로 근접 감사](../crates/mappa-map-data/src/bin/audit_tiger_raw_border_candidates.rs)는 후보와 반대편 주의 ZIP 원본 행을 비교한다. 공식 manifest의 ZIP SHA-256과 NAD83 `.prj`를 확인하고 선형에서 후보까지 거리를 계산한다. [OK 후보 47개 대 KS 원본](../artifacts/world-roads/ok-state/raw-kansas-near-ok-candidates.log)은 Kansas ZIP 11개·27,456행을 검사했다. **16개** 후보의 20m 이내에 원본 선이 있었지만 채택된 차량도로 선은 0개였다. 가장 가까운 원본 분류는 `S1500` 10개, `S1740` 5개, `S1750` 1개다. 나머지 **31개** 후보 주변 20m에는 Kansas 원본 선도 없었다. [KS 후보 8개 대 OK 원본](../artifacts/world-roads/ok-state/raw-oklahoma-near-ks-candidates.log)은 Oklahoma ZIP 6개·26,454행을 검사했고 8개 모두 20m 이내 원본 선이 없었다.

[Census MTFCC 정의](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)에 따르면 `S1500`은 4륜구동 차량이 필요한 비포장 길, `S1740`은 대체로 사유지 안의 산업·농장 등 접근로, `S1750`은 Census 내부용 분류다. 따라서 16개 선을 일반 차량도로 레이어로 자동 합치지 않는다. 정상적인 막다른 길인지, 다른 자료에서 누락된 일반 도로가 있는지는 아직 검증되지 않았다. 별도 길 종류를 제공할 때는 접근 제한과 시각 표현을 먼저 정해야 한다.

## CO–KS·NE·OK 원본 행 대조

[CO–KS 끝점](../artifacts/world-roads/co-state/border-endpoints-ks.log), [CO–NE 끝점](../artifacts/world-roads/co-state/border-endpoints-ne.log), [CO–OK 끝점](../artifacts/world-roads/co-state/border-endpoints-ok.log)의 방향별 후보는 각각 27·40·17개, 합계 84개다. 공식 반대편 ZIP 원본을 같은 20m 기준으로 검사했다.

| 후보 방향 | 반대편 원본 행 감사 | 후보 | 원본 선 ≤20m | 채택 도로 ≤20m | 제외 분류 |
|---|---|---:|---:|---:|---|
| CO→KS | [Kansas 원본](../artifacts/world-roads/co-state/raw-ks-near-co-candidates.log) | 10 | 1 | 0 | `S1740` 1 |
| KS→CO | [Colorado 원본](../artifacts/world-roads/co-state/raw-co-near-ks-candidates.log) | 17 | 1 | 0 | `S1740` 1 |
| CO→NE | [Nebraska 원본](../artifacts/world-roads/co-state/raw-ne-near-co-candidates.log) | 17 | 4 | 0 | `S1500` 4 |
| NE→CO | [Colorado 원본](../artifacts/world-roads/co-state/raw-co-near-ne-candidates.log) | 23 | 7 | 0 | `S1740` 6, `S1500` 1 |
| OK→CO | [Colorado 원본](../artifacts/world-roads/co-state/raw-co-near-ok-candidates.log) | 17 | 0 | 0 | 없음 |

검사한 후보 **84개 중 13개**는 반대편에 원본 선이 있었으나 `S1500` 5개·`S1740` 8개로 현재 일반 도로 레이어에서 제외한 종류다. 나머지 **71개**는 반대편 공식 원본 선이 20m 이내에 없었다. 이 결과도 정상적인 막다른 길과 실제 누락을 구별하지 못한다. CO→OK는 후보 0개라 별도 원본 근접 감사를 실행하지 않았다.

## SD–NE·IA 및 WY–CO·NE·SD 원본 행 대조

[South Dakota–Nebraska](../artifacts/world-roads/sd-state/border-endpoints-ne.log), [South Dakota–Iowa](../artifacts/world-roads/sd-state/border-endpoints-ia.log), [Wyoming–Colorado](../artifacts/world-roads/wy-state/border-endpoints-co.log), [Wyoming–Nebraska](../artifacts/world-roads/wy-state/border-endpoints-ne.log), [Wyoming–South Dakota](../artifacts/world-roads/wy-state/border-endpoints-sd.log) 경계의 양방향 후보를 같은 20m 기준으로 각 반대편 공식 ZIP 원본과 대조했다.

| 후보 방향 | 반대편 원본 행 감사 | 후보 | 원본 선 ≤20m | 채택 도로 ≤20m | 제외 분류 |
|---|---|---:|---:|---:|---|
| SD→NE | [Nebraska 원본](../artifacts/world-roads/sd-state/raw-ne-near-sd-candidates.log) | 18 | 6 | 0 | `S1500` 4, `S1740` 2 |
| NE→SD | [South Dakota 원본](../artifacts/world-roads/sd-state/raw-sd-near-ne-candidates.log) | 27 | 7 | 0 | `S1500` 7 |
| SD→IA | [Iowa 원본](../artifacts/world-roads/sd-state/raw-ia-near-sd-candidates.log) | 1 | 0 | 0 | 없음 |
| IA→SD | [South Dakota 원본](../artifacts/world-roads/sd-state/raw-sd-near-ia-candidates.log) | 4 | 0 | 0 | 없음 |
| WY→CO | [Colorado 원본](../artifacts/world-roads/wy-state/raw-co-near-wy-candidates.log) | 46 | 8 | 0 | `S1500` 7, `S1740` 1 |
| CO→WY | [Wyoming 원본](../artifacts/world-roads/wy-state/raw-wy-near-co-candidates.log) | 28 | 12 | 0 | `S1500` 9, `S1740` 3 |
| WY→NE | [Nebraska 원본](../artifacts/world-roads/wy-state/raw-ne-near-wy-candidates.log) | 16 | 6 | 0 | `S1500` 6 |
| NE→WY | [Wyoming 원본](../artifacts/world-roads/wy-state/raw-wy-near-ne-candidates.log) | 40 | 12 | 0 | `S1500` 7, `S1740` 5 |
| WY→SD | [South Dakota 원본](../artifacts/world-roads/wy-state/raw-sd-near-wy-candidates.log) | 28 | 3 | 0 | `S1500` 3 |
| SD→WY | [Wyoming 원본](../artifacts/world-roads/wy-state/raw-wy-near-sd-candidates.log) | 6 | 1 | 0 | `S1740` 1 |

방향별 후보 **214개 중 55개**는 반대편 공식 원본 선이 20m 안에 있지만 전부 현재 일반 도로 레이어에서 제외한 종류다. 나머지 **159개**는 그 거리 안에 공식 원본 선도 없다. 같은 좌표의 중복 끝점도 방향별 후보에 포함되므로 이를 독립된 실제 도로 214개로 해석하지 않는다. 정상적인 막다른 길과 실제 원천 누락은 아직 구별하지 못한다.

## MT–WY·SD 원본 행 대조

[Montana–Wyoming](../artifacts/world-roads/mt-state/border-endpoints-wy.log)·[Montana–South Dakota](../artifacts/world-roads/mt-state/border-endpoints-sd.log) 경계의 방향별 후보도 같은 20m 기준으로 반대편 공식 ZIP 원본과 대조했다.

| 후보 방향 | 반대편 원본 행 감사 | 후보 | 원본 선 ≤20m | 채택 도로 ≤20m | 제외 분류 |
|---|---|---:|---:|---:|---|
| MT→WY | [Wyoming 원본](../artifacts/world-roads/mt-state/raw-wy-near-mt-candidates.log) | 33 | 21 | 0 | `S1500` 14, `S1740` 7 |
| WY→MT | [Montana 원본](../artifacts/world-roads/mt-state/raw-mt-near-wy-candidates.log) | 48 | 18 | 0 | `S1500` 14, `S1740` 4 |
| MT→SD | [South Dakota 원본](../artifacts/world-roads/mt-state/raw-sd-near-mt-candidates.log) | 2 | 2 | 0 | `S1500` 2 |
| SD→MT | [Montana 원본](../artifacts/world-roads/mt-state/raw-mt-near-sd-candidates.log) | 1 | 1 | 0 | `S1500` 1 |

방향별 후보 **84개 중 42개**는 반대편 공식 원본 선이 20m 안에 있지만 모두 현재 일반 도로 레이어에서 제외한 종류다. **42개**는 그 거리 안에 원본 선이 없다. 이 수는 실제 통행 가능한 도로 단절 건수가 아니다.

## ND–MT·SD 원본 행 대조

[North Dakota–Montana](../artifacts/world-roads/nd-state/border-endpoints-mt.log)·[North Dakota–South Dakota](../artifacts/world-roads/nd-state/border-endpoints-sd.log) 경계의 방향별 후보도 같은 20m 기준으로 반대편 공식 ZIP 원본과 대조했다.

| 후보 방향 | 반대편 원본 행 감사 | 후보 | 원본 선 ≤20m | 채택 도로 ≤20m | 제외 분류 |
|---|---|---:|---:|---:|---|
| ND→MT | [Montana 원본](../artifacts/world-roads/nd-state/raw-mt-near-nd-candidates.log) | 51 | 28 | 0 | `S1500` 28 |
| MT→ND | [North Dakota 원본](../artifacts/world-roads/nd-state/raw-nd-near-mt-candidates.log) | 23 | 16 | 0 | `S1500` 15, `S1740` 1 |
| ND→SD | [South Dakota 원본](../artifacts/world-roads/nd-state/raw-sd-near-nd-candidates.log) | 40 | 23 | 0 | `S1500` 22, `S1740` 1 |
| SD→ND | [North Dakota 원본](../artifacts/world-roads/nd-state/raw-nd-near-sd-candidates.log) | 35 | 29 | 0 | `S1500` 29 |

방향별 후보 **149개 중 96개**는 반대편 공식 원본 선이 20m 안에 있지만 현재 일반 도로 레이어에서 제외한 종류다. **53개**는 그 거리 안에 원본 선이 없다. 이 수는 실제 통행 가능한 도로 단절 건수가 아니다.

## MN–IA·ND·SD·WI 원본 행 대조

[Minnesota–Iowa](../artifacts/world-roads/mn-state/border-endpoints-ia.log), [Minnesota–North Dakota](../artifacts/world-roads/mn-state/border-endpoints-nd.log), [Minnesota–South Dakota](../artifacts/world-roads/mn-state/border-endpoints-sd.log), [Minnesota–Wisconsin](../artifacts/world-roads/mn-state/border-endpoints-wi.log) 경계의 방향별 후보를 같은 20m 기준으로 반대편 공식 ZIP 원본과 대조했다.

| 후보 방향 | 반대편 원본 행 감사 | 후보 | 원본 선 ≤20m | 채택 도로 ≤20m | 제외 분류 |
|---|---|---:|---:|---:|---|
| MN→IA | [Iowa 원본](../artifacts/world-roads/mn-state/raw-ia-near-mn-candidates.log) | 11 | 5 | 0 | `S1740` 5 |
| IA→MN | [Minnesota 원본](../artifacts/world-roads/mn-state/raw-mn-near-ia-candidates.log) | 4 | 2 | 0 | `S1740` 2 |
| MN→ND | [North Dakota 원본](../artifacts/world-roads/mn-state/raw-nd-near-mn-candidates.log) | 3 | 1 | 0 | `S1500` 1 |
| ND→MN | [Minnesota 원본](../artifacts/world-roads/mn-state/raw-mn-near-nd-candidates.log) | 4 | 2 | 0 | `S1500` 2 |
| MN→SD | [South Dakota 원본](../artifacts/world-roads/mn-state/raw-sd-near-mn-candidates.log) | 9 | 2 | 0 | `S1500` 2 |
| SD→MN | [Minnesota 원본](../artifacts/world-roads/mn-state/raw-mn-near-sd-candidates.log) | 10 | 0 | 0 | 없음 |
| MN→WI | [Wisconsin 원본](../artifacts/world-roads/mn-state/raw-wi-near-mn-candidates.log) | 1 | 0 | 0 | 없음 |
| WI→MN | [Minnesota 원본](../artifacts/world-roads/mn-state/raw-mn-near-wi-candidates.log) | 2 | 0 | 0 | 없음 |

방향별 후보 **44개 중 12개**는 반대편 공식 원본 선이 20m 안에 있지만 현재 일반 도로 레이어에서 제외한 종류다. **32개**는 그 거리 안에 원본 선이 없다. 이 수는 실제 통행 가능한 도로 단절 건수가 아니다.

## ID–MT·UT·WY 및 UT–CO·WY 원본 행 대조

[Idaho–Montana](../artifacts/world-roads/id-state/border-endpoints-mt.log), [Idaho–Utah](../artifacts/world-roads/id-state/border-endpoints-ut.log), [Idaho–Wyoming](../artifacts/world-roads/id-state/border-endpoints-wy.log), [Utah–Colorado](../artifacts/world-roads/ut-state/border-endpoints-co.log), [Utah–Wyoming](../artifacts/world-roads/ut-state/border-endpoints-wy.log) 경계의 방향별 후보를 같은 20m 기준으로 반대편 공식 ZIP 원본과 대조했다. ID–UT 경계는 한 번만 센다.

| 후보 방향 | 반대편 원본 행 감사 | 후보 | 원본 선 ≤20m | 채택 도로 ≤20m | 제외 분류 |
|---|---|---:|---:|---:|---|
| ID→MT | [Montana 원본](../artifacts/world-roads/id-state/raw-mt-near-id-candidates.log) | 98 | 12 | 0 | `S1500` 9, `S1740` 3 |
| MT→ID | [Idaho 원본](../artifacts/world-roads/id-state/raw-id-near-mt-candidates.log) | 39 | 12 | 0 | `S1500` 11, `S1740` 1 |
| ID→UT | [Utah 원본](../artifacts/world-roads/id-state/raw-ut-near-id-candidates.log) | 16 | 1 | 0 | `S1740` 1 |
| UT→ID | [Idaho 원본](../artifacts/world-roads/id-state/raw-id-near-ut-candidates.log) | 23 | 5 | 0 | `S1500` 3, `S1740` 2 |
| ID→WY | [Wyoming 원본](../artifacts/world-roads/id-state/raw-wy-near-id-candidates.log) | 15 | 2 | 0 | `S1500` 2 |
| WY→ID | [Idaho 원본](../artifacts/world-roads/id-state/raw-id-near-wy-candidates.log) | 17 | 3 | 0 | `S1740` 3 |
| UT→CO | [Colorado 원본](../artifacts/world-roads/ut-state/raw-co-near-ut-candidates.log) | 56 | 9 | 0 | `S1500` 8, `S1740` 1 |
| CO→UT | [Utah 원본](../artifacts/world-roads/ut-state/raw-ut-near-co-candidates.log) | 18 | 9 | 0 | `S1500` 3, `S1740` 6 |
| UT→WY | [Wyoming 원본](../artifacts/world-roads/ut-state/raw-wy-near-ut-candidates.log) | 32 | 9 | 0 | `S1500` 9 |
| WY→UT | [Utah 원본](../artifacts/world-roads/ut-state/raw-ut-near-wy-candidates.log) | 12 | 5 | 0 | `S1500` 5 |

방향별 후보 **326개 중 67개**는 반대편 공식 원본 선이 20m 안에 있지만 현재 일반 도로 레이어에서 제외한 종류다. **259개**는 그 거리 안에 원본 선이 없다. 같은 좌표의 중복 끝점이 포함될 수 있고, 실제 통행 가능한 도로 단절 건수로 해석하지 않는다.

## AZ–UT·NM·NV, NV–UT·ID 및 NM–CO·OK 원본 행 대조

[Arizona–Utah](../artifacts/world-roads/az-state/border-endpoints-ut.log), [Arizona–New Mexico](../artifacts/world-roads/az-state/border-endpoints-nm.log), [Arizona–Nevada](../artifacts/world-roads/az-state/border-endpoints-nv.log), [Nevada–Utah](../artifacts/world-roads/nv-state/border-endpoints-ut.log), [Nevada–Idaho](../artifacts/world-roads/nv-state/border-endpoints-id.log), [New Mexico–Colorado](../artifacts/world-roads/nm-state/border-endpoints-co.log), [New Mexico–Oklahoma](../artifacts/world-roads/nm-state/border-endpoints-ok.log)의 방향별 후보를 반대편 공식 ZIP 원본과 20m 기준으로 대조했다. 7개 경계는 각각 한 번만 센다.

| 후보 방향 | 반대편 원본 행 감사 | 후보 | 원본 선 ≤20m | 채택 도로 ≤20m | 가장 가까운 원본 제외 분류 |
|---|---|---:|---:|---:|---|
| AZ→UT | [Utah 원본](../artifacts/world-roads/az-state/raw-ut-near-az-candidates.log) | 22 | 3 | 0 | `S1500` 2, `S1740` 1 |
| UT→AZ | [Arizona 원본](../artifacts/world-roads/az-state/raw-az-near-ut-candidates.log) | 27 | 0 | 0 | 없음 |
| AZ→NM | [New Mexico 원본](../artifacts/world-roads/az-state/raw-nm-near-az-candidates.log) | 42 | 10 | 0 | `S1500` 6, `S1740` 3, `S1750` 1 |
| NM→AZ | [Arizona 원본](../artifacts/world-roads/az-state/raw-az-near-nm-candidates.log) | 35 | 15 | 0 | `S1500` 8, `S1740` 7 |
| AZ→NV | [Nevada 원본](../artifacts/world-roads/az-state/raw-nv-near-az-candidates.log) | 6 | 0 | 0 | 없음 |
| NV→AZ | [Arizona 원본](../artifacts/world-roads/az-state/raw-az-near-nv-candidates.log) | 1 | 0 | 0 | 없음 |
| NV→UT | [Utah 원본](../artifacts/world-roads/nv-state/raw-ut-near-nv-candidates.log) | 10 | 1 | 0 | `S1500` 1 |
| UT→NV | [Nevada 원본](../artifacts/world-roads/nv-state/raw-nv-near-ut-candidates.log) | 38 | 8 | 0 | `S1500` 8 |
| NV→ID | [Idaho 원본](../artifacts/world-roads/nv-state/raw-id-near-nv-candidates.log) | 8 | 3 | 0 | `S1500` 2, `S1740` 1 |
| ID→NV | [Nevada 원본](../artifacts/world-roads/nv-state/raw-nv-near-id-candidates.log) | 15 | 12 | 0 | `S1500` 10, `S1740` 2 |
| NM→CO | [Colorado 원본](../artifacts/world-roads/nm-state/raw-co-near-nm-candidates.log) | 84 | 37 | 0 | `S1500` 36, `S1740` 1 |
| CO→NM | [New Mexico 원본](../artifacts/world-roads/nm-state/raw-nm-near-co-candidates.log) | 29 | 12 | 0 | `S1500` 2, `S1740` 9, `S1750` 1 |
| NM→OK | [Oklahoma 원본](../artifacts/world-roads/nm-state/raw-ok-near-nm-candidates.log) | 5 | 3 | 0 | `S1740` 3 |
| OK→NM | [New Mexico 원본](../artifacts/world-roads/nm-state/raw-nm-near-ok-candidates.log) | 1 | 0 | 0 | 없음 |

방향별 후보 **323개 중 104개**는 반대편 공식 원본 선이 20m 안에 있지만 현재 일반 차량도로에서 제외한 종류다. **219개**는 그 거리 안에 원본 선이 없었다. 이 수는 중복 좌표가 포함될 수 있는 진단 후보 수이며 실제 통행 가능한 도로 단절 건수가 아니다. 도로 종류를 임의로 승격하거나 선을 이어 붙이지 않았다.

## 이 검사로 확인할 수 없는 것

- 폴리곤 경계에서 **정확히 동일한 좌표 선분**만 사용하므로 양쪽 폴리곤의 선분 분할 방식이 다른 구간은 빠질 수 있다. 위 선분 수를 주 경계 전체 길이 또는 전수 검사로 해석하지 않는다.
- 도로의 끝점이 경계에서 떨어진 긴 선분으로 그려졌거나, 다리·터널의 높이 레벨이 다른 경우에는 토폴로지와 통행 가능성을 판정하지 않는다.
- 원본 NAD83 숫자 좌표를 지도의 WGS84 입력으로 보존한 현재 경로의 datum 차이, 독립 현장 위치 정확도, 현재 운행 가능성은 검증하지 않았다.
- 0.1m/20m 수치는 **두 공식 파일 사이의 기하 비교 기준**이며 실세계 측량 오차 보증이 아니다.

다음 품질 게이트는 후보 공백의 원본 행·도로 종류·주 경계 수계/교량을 대조하고, 실제 연결 여부가 확인된 사례만 수정 규칙에 반영하는 것이다. 결과가 확보되기 전에는 주 경계 연결성을 `미검증`으로 둔다.

```sh
cargo run --offline -p mappa-map-data --bin audit_us_state_border_endpoints -- \
  data/local/us_state/tl_2025_us_state.zip \
  59a220888a8d9be8117c4fcd38f542bd02d81abf0d198c78113595ad540dd957 \
  20 artifacts/world-roads/ks-state/roads.mgeodb \
  29 artifacts/world-roads/mo-state/roads.mgeodb
```

후보 행 추적은 같은 명령의 두 GeoDB 뒤에 `--details`를 추가한다.

```sh
cargo run --release --offline -p mappa-map-data --bin audit_tiger_raw_border_candidates -- \
  data/us_tiger_ks_state_roads.toml \
  artifacts/world-roads/ok-state/border-endpoints-ks-detailed.log 40 20
cargo run --release --offline -p mappa-map-data --bin audit_tiger_raw_border_candidates -- \
  data/us_tiger_ok_state_roads.toml \
  artifacts/world-roads/ok-state/border-endpoints-ks-detailed.log 20 20
```
