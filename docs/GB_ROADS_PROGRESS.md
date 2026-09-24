# Great Britain 도로 지도 구축 현황 — 2026-09-24

## 판정

Mappa 기본 세계 모드는 Ordnance Survey의 [OS Open Roads](https://www.ordnancesurvey.co.uk/products/os-open-roads) **2026-04** 배포본에서 RoadLink가 들어 있는 **52개 100km 격자 모두**를 로컬 PMTiles로 읽는다. 이는 해당 공식 원천의 GB RoadLink 전체 처리 결과다. **북아일랜드는 GB 원천에 없고**, OS Open Roads 자체가 약 1:25,000 축척으로 일반화된 도로망이므로 모든 실제 도로·차선·회전 제약까지 완성된 지도라는 뜻은 아니다. 전 세계 상세지도 목표도 계속 진행 중이다.

| 단계 | 확인된 결과 |
|---|---|
| 공식 입력 | [제품 메타데이터](../data/os_open_roads_2026_04_product.json) `2026-04`, 범위 `GB`. [배포 목록](../data/os_open_roads_2026_04_downloads.json)의 Shapefile ZIP 606,145,264바이트, MD5 `8c6e66a255f79e3e31845307cefee298`. 실제 로컬 ZIP SHA-256 `bb2bbff8be3d637b38ed5d06277856c15f41ea262614edf974be8f4b96fdc791`. |
| ZIP 검사 | 500개 항목을 끝까지 읽어 CRC 오류 0개. Shapefile 124개, 그중 RoadLink 52개. 모든 RoadLink `.prj`가 EPSG:27700, `.shp`가 PolylineZ(13). [파일 목록](../artifacts/world-roads/gb/source-index.tsv)과 [RoadLink 레코드·범위](../artifacts/world-roads/gb/roadlink-index.tsv). |
| 중복 검사 | DBF 원본 레코드 3,967,825개. 공식 `identifier` 기준 고유 3,961,077개, 격자 간 중복 6,748개. 중복의 X/Y 도형·`function`·`name1` 불일치 0개. [소유 격자 인덱스](../data/os_open_roads_2026_04_duplicate_owner.tsv) SHA-256 `2bb93f47da0b6e90630bc82035e0261ae7d00abcdd685a0902f392b9cd61b7b4`. |
| Rust 변환 | 52개 격자의 고유 RoadLink **3,961,077개 채택**, 격자 중복 **6,748개 제외**, 형식·좌표 변환·도형 거절 **0개**. OSTN15로 EPSG:27700 X/Y를 경위도로 변환했다. 중간 MappaGeoDB와 원본 ZIP은 로컬에만 둔다. 각 [원천 manifest](../data/os_open_roads_tq.toml)에 ZIP·중복 인덱스 SHA와 권리, 변환 버전이 고정된다. |
| 오프라인 팩 | [52개 팩 카탈로그](../assets/map/gb_regional_packs.toml), 합계 **376,893,577바이트**, 최대 단일 팩 TQ **34,221,508바이트**. [팩별 SHA·범위](../artifacts/world-roads/gb/pack-index.tsv). z10–15에서 비어 있지 않은 타일 **480,930개**, 확대 단계별 중복을 포함한 인코딩 선 도형 **19,701,267개**. [전수 타일 해독](../artifacts/world-roads/gb/tile-audit.log) 실패 **0개**. |
| 실제 화면 | 기본 세계 모드 Metal 캡처: [런던](../artifacts/world-roads/gb/london-world-z14.png), [에든버러](../artifacts/world-roads/gb/edinburgh-world-z14.png), [카디프](../artifacts/world-roads/gb/cardiff-world-z14.png), [TQ–TL 경계](../artifacts/world-roads/gb/tq-tl-border-world-z12.png). 네 화면의 타일 오류 0개. |

TQ–TL 경계는 중복 제거 전에 공유 도로 ID 231개와 확대 단계별 동일 타일 선 1,494개가 있었고, 두 팩을 중복 제거해 다시 만든 뒤 같은 타일 검사에서 동일 선 **0개**였다. 이는 그 두 팩의 정확히 같은 타일 선 검사이며, 전국 도로의 그래프 연결성이나 지상 위치 정확도 통과 판정은 아니다.

## 출처·좌표·권리

OS Open Roads는 [Open Government Licence v3.0](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/) 아래 무료로 재사용 가능한 OS OpenData다. 팩에는 원본 ZIP의 `/doc/licence.txt`에 있는 `Contains Ordnance Survey data © Crown copyright and database right 2026` 문구를 넣었다. 공식 [OS 표기 지침](https://www.ordnancesurvey.co.uk/documents/OS-Co-BrandingGuidelines-Accessible-PSGA.pdf)에 따라 스코틀랜드 관련 H·N 격자에는 `This product contains data created and maintained by Scottish Local Government.`도 표시한다. H·N 전체에 보수적으로 넣었으므로 일부 북잉글랜드 격자에도 이 추가 문구가 보일 수 있다. 앱은 인접 팩의 같은 문구를 한 번만 표시한다.

Rust 빌드 도구는 [`lonlat_bng`](https://github.com/urschrei/lonlat_bng)와 [`ostn15_phf`](https://github.com/urschrei/OSTN15_PHF)를 사용한다. 두 패키지의 [Blue Oak 1.0.0](https://blueoakcouncil.org/license/1.0.0) 및 격자 자료의 [OS·MOD 고지](https://github.com/urschrei/OSTN15_PHF/blob/master/OSTN15_license.txt)를 따른다. OS의 [좌표계 문서](https://docs.os.uk/os-downloads/products/transport-network-portfolio/os-open-roads/os-open-roads-overview/os-open-roads-data)는 Shapefile을 British National Grid EPSG:27700으로 지정한다. 현재 변환 결과의 ETRS89 경위도를 렌더링 경위도로 사용한다. 별도 OS 기준점·현장 관측으로 WGS84와의 차이를 측정하지 않았으므로 **미터급 절대 정확도를 주장하지 않는다.**

[OS 제품 설명](https://docs.os.uk/os-downloads/products/transport-network-portfolio/os-open-roads)은 이 도로망이 상세 차량 내비게이션 제약을 제공하지 않는다고 밝힌다. Mappa에서도 도로 연결성·일방통행·회전 제약을 아직 검증하지 않았다. GB의 상세 건물·수면·공원·역·공공기관 레이어도 현재 화면에는 없다. iPhone 실기기 성능과 위치 정확도도 아직 측정하지 않았다.

## 재현

공식 다운로드 URL은 **빌드 단계**에서만 사용한다. 앱 런타임은 커밋된 로컬 PMTiles를 읽고 지도 API를 호출하지 않는다. 원본 ZIP과 중간 GeoDB는 크기 때문에 Git에서 제외했다.

```bash
cargo run --release -p mappa-map-acquire -- --file \
  'https://api.os.uk/downloads/v1/products/OpenRoads/downloads?area=GB&format=ESRI%C2%AE+Shapefile&redirect' \
  data/local/uk_open_roads/oproad_essh_gb.zip \
  606145264 8c6e66a255f79e3e31845307cefee298

cargo run --release -p mappa-map-data --bin audit_os_open_roads_zip -- \
  data/local/uk_open_roads/oproad_essh_gb.zip \
  artifacts/world-roads/gb/source-index.tsv \
  artifacts/world-roads/gb/roadlink-index.tsv
cargo run --release -p mappa-map-data --bin audit_os_open_roads_grid_overlap -- \
  data/local/uk_open_roads/oproad_essh_gb.zip --all \
  data/os_open_roads_2026_04_duplicate_owner.tsv

cargo run --release -p mappa-map-data --features gb-roads \
  --bin build_os_open_roads_grid -- \
  data/local/uk_open_roads/oproad_essh_gb.zip TQ \
  data/os_open_roads_2026_04_product.json \
  data/os_open_roads_2026_04_downloads.json 2026-09-24 \
  data/os_open_roads_2026_04_duplicate_owner.tsv \
  artifacts/world-roads/gb/tq.mgeodb data/os_open_roads_tq.toml
cargo run --release -p mappa-map-data --bin build_canonical_tiles -- \
  data/os_open_roads_tq.toml artifacts/world-roads/gb/tq.mgeodb \
  artifacts/world-roads/gb/tq.pmtiles
cargo run --release -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-roads/gb/tq.pmtiles
cargo run --release -p mappa-map-data --bin make_os_open_roads_catalog -- \
  artifacts/world-roads/gb/roadlink-index.tsv . \
  assets/map/gb_regional_packs.toml artifacts/world-roads/gb/pack-index.tsv
```

각 격자는 같은 `build_os_open_roads_grid` → `build_canonical_tiles` 순서로 생성하며, 전체 [빌드 결과](../artifacts/world-roads/gb/build-all.log)는 TQ·TL 외 50개 격자의 실행 기록이다. 재실행 날짜는 실제 다운로드 날짜로 기록해야 한다.
