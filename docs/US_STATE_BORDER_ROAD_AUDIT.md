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

원시 출력과 후보 좌표 표본은 [KS–MO](../artifacts/world-roads/ks-state/border-endpoints-mo.log), [MO–IA](../artifacts/world-roads/mo-state/border-endpoints-ia.log), [MO–IL](../artifacts/world-roads/mo-state/border-endpoints-il.log), [NE–KS](../artifacts/world-roads/ne-state/border-endpoints-ks.log), [NE–IA](../artifacts/world-roads/ne-state/border-endpoints-ia.log), [KS–OK](../artifacts/world-roads/ok-state/border-endpoints-ks.log)에 기록했다. `--details` 옵션을 적용한 [KS–OK 후보 원본 계보](../artifacts/world-roads/ok-state/border-endpoints-ks-detailed.log)는 후보 끝점 55개의 관련 원본 도로 행 67개와 도로 분류·이름을 기록했다. 67개는 모두 `RoadResidential`이며, 이는 통행 가능성과 실제 연결 여부의 판정이 아니다. `상대 끝점 없음`과 `상대 선 위`의 차이는 두 주 원천이 같은 도로를 다른 지점에서 분할할 수 있음을 보여준다. `후보 공백`에는 강가·주 경계에서 끝나는 정상 도로가 포함될 수 있다. 선을 임의로 이어 붙이지 않았다.

## KS–OK 원본 행 대조

새 Rust [원본 도로 근접 감사](../crates/mappa-map-data/src/bin/audit_tiger_raw_border_candidates.rs)는 후보와 반대편 주의 ZIP 원본 행을 비교한다. 공식 manifest의 ZIP SHA-256과 NAD83 `.prj`를 확인하고 선형에서 후보까지 거리를 계산한다. [OK 후보 47개 대 KS 원본](../artifacts/world-roads/ok-state/raw-kansas-near-ok-candidates.log)은 Kansas ZIP 11개·27,456행을 검사했다. **16개** 후보의 20m 이내에 원본 선이 있었지만 채택된 차량도로 선은 0개였다. 가장 가까운 원본 분류는 `S1500` 10개, `S1740` 5개, `S1750` 1개다. 나머지 **31개** 후보 주변 20m에는 Kansas 원본 선도 없었다. [KS 후보 8개 대 OK 원본](../artifacts/world-roads/ok-state/raw-oklahoma-near-ks-candidates.log)은 Oklahoma ZIP 6개·26,454행을 검사했고 8개 모두 20m 이내 원본 선이 없었다.

[Census MTFCC 정의](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf)에 따르면 `S1500`은 4륜구동 차량이 필요한 비포장 길, `S1740`은 대체로 사유지 안의 산업·농장 등 접근로, `S1750`은 Census 내부용 분류다. 따라서 16개 선을 일반 차량도로 레이어로 자동 합치지 않는다. 정상적인 막다른 길인지, 다른 자료에서 누락된 일반 도로가 있는지는 아직 검증되지 않았다. 별도 길 종류를 제공할 때는 접근 제한과 시각 표현을 먼저 정해야 한다.

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
