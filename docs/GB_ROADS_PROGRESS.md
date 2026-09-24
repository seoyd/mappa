# 영국 도로 원천 확보 게이트 — 2026-09-24

## 현재 판정

영국 Ordnance Survey의 [OS Open Roads](https://www.ordnancesurvey.co.uk/products/os-open-roads)는 무료 OpenData로, [Open Government Licence v3.0](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/) 조건에서 재사용할 수 있다. 공식 [제품 메타데이터](../data/os_open_roads_2026_04_product.json)는 버전 `2026-04`, 범위 `GB`로 표시한다. 이는 Great Britain 원천이며 북아일랜드까지 포함한 영국 전체 원천이라고 부르지 않는다. 현재 **TQ 한 격자만 Mappa 오프라인 팩으로 변환했다.**

[공식 다운로드 목록](../data/os_open_roads_2026_04_downloads.json)에 고정된 전국 ESRI Shapefile ZIP은 `oproad_essh_gb.zip`, 게시 크기 606,145,264바이트, 게시 MD5 `8c6e66a255f79e3e31845307cefee298`이다. Rust 빌드 전용 도구가 스트리밍으로 받아 게시 크기와 MD5를 대조했고 일치했다. 원본 ZIP은 로컬 `data/local/uk_open_roads/`에만 둔다. ZIP SHA-256은 `bb2bbff8be3d637b38ed5d06277856c15f41ea262614edf974be8f4b96fdc791`이다. ZIP 500개 항목 전부 압축 해제·CRC 검사를 통과했고 Shapefile은 124개, 그중 RoadLink는 52개이다. [원본 목록](../artifacts/world-roads/gb/source-index.tsv)과 [RoadLink 감사표](../artifacts/world-roads/gb/roadlink-index.tsv)에 항목 크기·DBF 레코드 수·British National Grid 좌표 범위를 기록했다. 52개 RoadLink의 DBF 레코드 합계는 3,967,825개다. 이 수에는 격자 경계 중복이 있을 수 있다.

원천의 [좌표계 안내](https://docs.os.uk/os-downloads/products/transport-network-portfolio/os-open-roads/os-open-roads-overview/os-open-roads-data)에 따르면 Shapefile은 British National Grid **EPSG:27700**, 벡터 타일은 Web Mercator **EPSG:3857**이다. 모든 RoadLink 파일의 `.prj`에 EPSG:27700 표기가 있고 `.shp` 도형 타입은 고도값을 포함한 **PolylineZ(13)**이다. Shapefile을 Mappa의 경위도 GeoDB에 넣으려면 OSTN15 변환과 독립 기준점 대조가 필요하다. [공식 파일 구조 문서](https://docs.os.uk/os-downloads/products/transport-network-portfolio/os-open-roads/os-open-roads-overview/coverage-and-file-sizes)에 따르면 Shapefile은 100km 격자로 나뉘고 경계를 넘는 피처가 여러 파일에 있을 수 있으므로 공식 `identifier`로 중복을 제거해야 한다. [제품 기술 문서](https://docs.os.uk/os-downloads/products/transport-network-portfolio/os-open-roads/os-open-roads-technical-specification)는 이 자료가 일반화된 약 1:25,000 축척 도로망이며 차량 내비게이션용 제약 정보가 아니라고 설명한다.

[OS OpenData 표기 안내](https://www.ordnancesurvey.co.uk/customers/public-sector/public-sector-licensing/copyright-acknowledgments)에 따른 저작권·출처 표기가 필요하다. 스코틀랜드 도로를 사용할 때는 Scottish Local Government의 추가 출처 표기도 확인해야 한다. 빌드 도구의 [`lonlat_bng`](https://github.com/urschrei/lonlat_bng)와 [`ostn15_phf`](https://github.com/urschrei/OSTN15_PHF)는 [Blue Oak 1.0.0](https://blueoakcouncil.org/license/1.0.0)이고, OSTN15 격자 자료에는 별도의 [OS·MOD BSD 2-Clause 고지](https://github.com/urschrei/OSTN15_PHF/blob/master/OSTN15_license.txt)가 따른다. 이 Rust 변환 도구는 빌드 단계에만 쓰며 지도 런타임의 네트워크 의존성을 만들지 않는다.

## TQ 격자 실증

공식 `data/TQ_RoadLink.shp`에는 DBF 레코드 **452,943개**가 있다. Rust 어댑터는 같은 ZIP의 `.prj`에서 EPSG:27700을 확인하고, 공식 `identifier`와 `function`을 읽어 PolylineZ의 X/Y에 OSTN15를 적용했다. 결과는 채택 **452,943개**, 거절 **0개**, MappaGeoDB 171,518,192바이트다. GeoDB는 원본 ZIP처럼 빌드 중간 산출물이므로 로컬에만 둔다. [원천 manifest](../data/os_open_roads_tq.toml)의 범위 `[-0.6212911803963149, 50.77023482362841, 0.8937973649076472, 51.7032767827252]`는 변환된 도형의 좌표 합집합이다.

[TQ 오프라인 팩](../artifacts/world-roads/gb/tq.pmtiles)은 34,325,388바이트, SHA-256 `047ccd84cfa943a2352651892af4d5e29499e9663f709210a6c95b6eb6412ae3`이다. 비어 있지 않은 타일 **20,851개**를 전부 해독해 실패 **0개**를 확인했다. 확대 단계별 중복을 포함한 타일 선 도형 수는 주요 **296,074**, 보조 **434,157**, 생활 **1,465,356**이다. 이 집계는 도로 원본의 고유 링크 수가 아니다. 원본 ZIP `/doc/licence.txt`의 저작권 문구를 그대로 manifest와 팩에 넣고 기본 세계 모드에도 보이도록 했다. [런던 기본 세계 화면](../artifacts/world-roads/gb/london-world-z14.png)은 z14.6에서 타일 오류 **0개**로 캡처했다. 도로 외 상세 건물·수면·공원은 아직 비어 있다. 독립 기준점과 도로 연결성, 격자 간 중복, 북아일랜드 원천, Great Britain 나머지 51개 격자는 아직 검증·구축되지 않았다. OS Open Roads는 일반화된 도로망이므로 이 결과를 개별 차선이나 턴 제약이 맞는 내비게이션 지도라고 판정하지 않는다.

## 빌드 원천 받기

다음 URL은 **원천 파일을 받아 오프라인 팩을 만드는 빌드 단계**에서만 쓴다. 앱 지도 런타임은 로컬 파일만 읽는다. 중단 시 Rust 도구가 `.part` 파일과 HTTP Range를 이용해 이어받고 최종 MD5를 대조한다.

```bash
cargo run --release -p mappa-map-acquire -- --file \
  'https://api.os.uk/downloads/v1/products/OpenRoads/downloads?area=GB&format=ESRI%C2%AE+Shapefile&redirect' \
  data/local/uk_open_roads/oproad_essh_gb.zip \
  606145264 8c6e66a255f79e3e31845307cefee298

cargo run --release -p mappa-map-data --bin audit_os_open_roads_zip -- \
  data/local/uk_open_roads/oproad_essh_gb.zip \
  artifacts/world-roads/gb/source-index.tsv \
  artifacts/world-roads/gb/roadlink-index.tsv

cargo run --release -p mappa-map-data --features gb-roads \
  --bin build_os_open_roads_grid -- \
  data/local/uk_open_roads/oproad_essh_gb.zip TQ \
  data/os_open_roads_2026_04_product.json \
  data/os_open_roads_2026_04_downloads.json 2026-09-24 \
  artifacts/world-roads/gb/tq.mgeodb data/os_open_roads_tq.toml
cargo run --release -p mappa-map-data --bin build_canonical_tiles -- \
  data/os_open_roads_tq.toml artifacts/world-roads/gb/tq.mgeodb \
  artifacts/world-roads/gb/tq.pmtiles
cargo run --release -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/gb/tq.pmtiles
```
