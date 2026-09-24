# 캐나다 NRN 도로 구축 현황 — 2026-09-24

## 현재 판정

**Prince Edward Island(PE)와 Nova Scotia(NS) 공식 도로선을 오프라인 지도에 연결했다. 캐나다의 나머지 11개 주·준주와 전 세계 상세지도는 미완성이다.** [캐나다 정부 NRN 목록](https://open.canada.ca/data/en/dataset/3d282116-e556-400c-9306-ca1a3cada77f?res_page=3)은 13개 주·준주별 Road Segment ZIP을 제공하고 [Open Government Licence – Canada](https://open.canada.ca/en/open-government-licence-canada)를 적용한다. 라이선스는 상업적 재사용·수정·재배포를 허용하고 출처 표기를 요구하며 공유조건은 없다. Mappa 앱은 원천 다운로드 API나 외부 지도 서버를 런타임에 호출하지 않는다.

| 단계 | 확인한 사실 |
|---|---|
| 원본 | 통계청 공식 [PE SHAPE ZIP](https://geo.statcan.gc.ca/nrn_rrn/pe/nrn_rrn_pe_SHAPE.zip) 12,067,032바이트, SHA-256 `02219e7e69eeaafaeab84882477a23f129f2b7fa4d24698372ed03afe45303e5`. 파일명 내부 버전 `23_0`, 이 배포본의 다운로드 날짜 2026-09-24. 원본은 로컬 `data/local/`에만 저장. |
| 압축·도형 | [Rust 원천 감사](../artifacts/world-roads/ca/pe-source-audit.log): ZIP 44개 항목 끝까지 읽어 CRC 오류 0, 영문 `ROADSEG` Polyline 19,705개, `.prj`는 NAD83(CSRS) 지리 좌표. 영문·불문은 같은 ZIP에 있고 영문 도로선만 채택. |
| ID 검사 | `NID`는 고유 15,466개여서 레코드 키로 쓸 수 없다. `ROADSEGID`는 19,705개 모두 고유해 이를 원본 ID로 사용. 원천 `ROADCLASS` 10종을 감사하고 전부 도로 선형으로 분류. |
| Rust 변환 | 고유 도로선 **19,705개 채택, 거절 0개**. [출처·SHA·권리 manifest](../data/ca_nrn_pe.toml), 로컬 MappaGeoDB 8,051,223바이트. NAD83(CSRS) 경위도 숫자를 유지해 표시한다. |
| 오프라인 팩 | [PE PMTiles](../artifacts/world-roads/ca/pe.pmtiles) 3,944,770바이트, SHA-256 `cc1493a85ec7a97d15ec069e9a655efe13ed1ba95484b31645cff0e762168fef`. z10–15의 비어 있지 않은 타일 9,554개, 확대별 중복 포함 선 도형 110,359개. [전수 해독 기록](../artifacts/world-roads/ca/pe-tile-audit.log) 실패 0. |
| 기본 화면 | [Charlottetown z14 Metal 캡처](../artifacts/world-roads/ca/charlottetown-world-z14.png), 타일 실패 0, `Contains information licensed under the Open Government Licence – Canada.` 출처 표기 확인. |

NS도 같은 [공식 주별 ZIP](https://geo.statcan.gc.ca/nrn_rrn/ns/nrn_rrn_ns_SHAPE.zip)을 실제로 확보했다. 원본은 **117,536,741바이트**, SHA-256 `19ffea53ae709e5c8a4c6d84610389fdef60a0615927d51eab34de2ced03145a`, 내부 버전 `18_0`이다. [Rust 감사](../artifacts/world-roads/ca/ns-source-audit.log)에서 ZIP 44개 항목 CRC 오류 0, NAD83(CSRS) Polyline ROADSEG **119,846개**, 고유 `ROADSEGID` 119,846개, 고유 `NID` 97,801개를 확인했다. `Freeway`·`Unknown`을 포함한 실제 9개 분류를 매핑해 **119,846개 채택·거절 0개**다. [NS manifest](../data/ca_nrn_ns.toml)와 로컬 GeoDB 88,042,051바이트에서 만든 [NS PMTiles](../artifacts/world-roads/ca/ns.pmtiles)는 **39,771,407바이트**, SHA-256 `ff14fef913ecec6a84a0b40eeff588b483056e1e03c2c10d067f9be10484cdc5`다. [비어 있지 않은 68,627개 타일 전수 해독](../artifacts/world-roads/ca/ns-tile-audit.log) 오류 0, [Halifax z14 Metal 화면](../artifacts/world-roads/ca/halifax-world-z14.png) 타일 실패 0이다.

두 주 합계 원천 도로선은 **139,551개**, 비어 있지 않은 타일은 **78,181개**, 팩 크기는 **43,716,177바이트**다. 이 합계는 두 주의 원본 레코드와 확대별 타일 수이며 다른 11개 주·준주의 완성률을 뜻하지 않는다.

분류는 원본 `ROADCLASS` 문자열을 사용한다. `Freeway`·`Expressway / Highway`·`Arterial`·`Ramp`를 주요 도로, `Collector`를 연결 도로, PE의 나머지 6종 및 NS의 분류 `Unknown`을 지역 도로로 표시한다. 실제 도로 연결성·차량 접근성·일방통행은 아직 감사하지 않았다. [공식 NRN 설명](https://open.canada.ca/data/en/dataset/3d282116-e556-400c-9306-ca1a3cada77f?res_page=3)의 13개 주·준주 범위는 **자료 배포 단위**이며 모든 실제 도로가 누락 없이 들어 있다는 검증 결과가 아니다.

좌표 원본은 NAD83(CSRS)다. 현재 숫자를 Web Mercator 경위도 입력으로 사용하며 별도 기준점으로 WGS84 변환 오차나 현장 위치 정확도를 재지 않았다. `ROADSEGID`는 각 주 안에서만 검증했고, 두 주 사이의 중복·경계 연결성도 미검증이다. PE·NS 화면에는 도로 외의 지역 상세 수면·건물·공원·역·공공기관이 없다. iPhone 실기기 성능도 아직 측정하지 않았다.

## 재현

원천은 빌드 때만 다운로드한다. 앱 런타임은 커밋된 로컬 팩을 읽는다.

```bash
mkdir -p data/local artifacts/world-roads/ca
curl --fail --location --output data/local/nrn_rrn_pe_SHAPE.zip \
  'https://geo.statcan.gc.ca/nrn_rrn/pe/nrn_rrn_pe_SHAPE.zip'
cargo run --release --offline -p mappa-map-data --bin audit_ca_nrn_zip -- \
  data/local/nrn_rrn_pe_SHAPE.zip PE
cargo run --release --offline -p mappa-map-data --bin build_ca_nrn_province -- \
  data/local/nrn_rrn_pe_SHAPE.zip PE 23.0 2026-09-24 \
  artifacts/world-roads/ca/pe.mgeodb data/ca_nrn_pe.toml
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/ca_nrn_pe.toml artifacts/world-roads/ca/pe.mgeodb \
  artifacts/world-roads/ca/pe.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/ca/pe.pmtiles

curl --fail --location --output data/local/nrn_rrn_ns_SHAPE.zip \
  'https://geo.statcan.gc.ca/nrn_rrn/ns/nrn_rrn_ns_SHAPE.zip'
cargo run --release --offline -p mappa-map-data --bin audit_ca_nrn_zip -- \
  data/local/nrn_rrn_ns_SHAPE.zip NS
cargo run --release --offline -p mappa-map-data --bin build_ca_nrn_province -- \
  data/local/nrn_rrn_ns_SHAPE.zip NS 18.0 2026-09-24 \
  artifacts/world-roads/ca/ns.mgeodb data/ca_nrn_ns.toml
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/ca_nrn_ns.toml artifacts/world-roads/ca/ns.mgeodb \
  artifacts/world-roads/ca/ns.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/ca/ns.pmtiles
```

재다운로드 시 ZIP SHA가 달라지면 공식 원천 갱신 여부와 스키마를 다시 감사해야 한다. manifest의 날짜는 실제 다운로드 날짜로 기록한다. 다음 단계는 나머지 11개 주·준주에 같은 스키마·권리·메모리 한계를 확인하고 지역별로 빌드하는 것이다. 북아일랜드 OSNI 도로선도 별도 공식 OGL 원천이 있지만 [현재 게시 ZIP 다운로드](https://admin.opendatani.gov.uk/dataset/osni-open-data-50k-transport-transport-lines)는 이 환경에서 HTTP 403을 반환해 아직 확보하지 못했다.
