# 프랑스 IGN 파리 D075 상세 수면 실증 — 2026-09-25

## 원천과 판정

[IGN BD TOPO 3.5 공식 명세](https://data.geopf.fr/annexes/ressources/documentation/DC_BDTOPO_3-5.pdf)는 `Surface hydrographique`를 내륙 수면의 3D 멀티폴리곤으로 정의한다. 바다·해양과 작은 수면 일부는 이 클래스에 포함되지 않는다. [공식 파리 D075 2026-06-15 배포본](https://data.geopf.fr/telechargement/resource/BDTOPO/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15)의 같은 7z 원본을 사용했다. 원본 SHA-256은 `dc1cdeaef2b220520faf2a05edea11af0ff2c7cc54961cb6cefd0af6e50c2414`다. 이 원천은 [Licence Ouverte 2.0](https://www.data.gouv.fr/pages/legal/licences/etalab-2.0)으로 게시되며 출처 표기가 필요하다. 권리·버전·파일 해시는 [수면 manifest](../data/ign_bdtopo_paris_water.toml)에 고정했다.

Rust 추출기는 7z 전체 CRC를 확인하면서 `HYDROGRAPHIE/SURFACE_HYDROGRAPHIQUE`의 다섯 Shapefile 구성 파일만 선택했다. 사용하지 않는 날짜 필드를 제외하고 DBF의 `ID`·`NATURE`·`ETAT`·`PERSISTANC`·수면 이름을 레코드 순서대로 보존했다. 파생 ZIP SHA-256은 `a391a9d7c7c665e3e40690a10bc36add0a3b4166869a4608e63dc7ee805cdce0`이다. 원본과 파생 ZIP은 로컬 빌드 입력으로 Git에는 포함하지 않는다.

| 단계 | 관측 사실 |
|---|---|
| 원본 | DBF 537개 레코드, RGF93 / Lambert-93 좌표계. |
| 정규화 | `ETAT=En service`이고 `PERSISTANC=Permanent`인 **239개 수면**과 피처별 출처를 채택. `En construction` 11개, `Intermittent` 284개, `Inconnue` 3개를 [거절 기록](../artifacts/world-water/fr-paris/water.rejected.json.gz)에 남김. 각 폴리곤의 링과 위상 검사에 통과. |
| 좌표 | Rust Lambert-93 역투영을 적용한 채택 형상 범위 `[2.1371806340501998, 48.75239315631629, 2.769379530255967, 48.95243084204839]`. 이는 파리 행정 경계가 아니다. |
| GeoDB | 로컬 474,391바이트, SHA-256 `d53f4f1415376b438513c63376a033ed6822400e24a0a3b4703d6711b23aedc2`. |
| 지도 팩 | [수면 PMTiles](../artifacts/world-water/fr-paris/water.pmtiles) 337,880바이트, SHA-256 `3d8081a07c17d47d3f670dab40ba75650f4d613afd376fc0bb7bb7db9acb34e4`. 실제 타일 594개 전수 해독, 실패 0. 타일별 중복 포함 수면 1,797개. |
| 화면 | [파리 도로·수면 합성 z14](../artifacts/world-water/fr-paris/paris-road-water-z14.png) Mac Metal 캡처에서 타일 실패 0. 센강 일부와 작은 수면이 표시된다. |

일시 수면을 상시 파란 수면처럼 표시하지 않기 위해 `Permanent`만 채택했다. 현재 판정은 파일·타일·화면의 표시 검증이다. 수면 경계의 현장 위치 정확도, RGF93에서 WGS84로의 독립 기준점 비교, 원본의 모든 수면 완전성, iPhone 성능은 검증하지 않았다. 여객역은 [별도 지역 팩](FR_PARIS_STATIONS_PROGRESS.md)에 있으며 파리 건물·공공시설은 아직 없다.

## 재현

```bash
cargo run --release --offline -p mappa-map-acquire --bin extract_ign_roads -- \
  data/local/ign_bdtopo_paris/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z \
  data/local/ign_bdtopo_paris/water.zip --water
cargo run --release --offline -p mappa-map-data --bin make_ign_bdtopo_roads_manifest -- \
  'https://data.geopf.fr/telechargement/download/BDTOPO/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z' \
  data/local/ign_bdtopo_paris/BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D075_2026-06-15.7z \
  data/local/ign_bdtopo_paris/water.zip data/ign_bdtopo_paris_water.toml 2026-09-25 --water
cargo run --release --offline -p mappa-map-data --bin build_canonical_proof -- \
  data/ign_bdtopo_paris_water.toml artifacts/world-water/fr-paris/water.mgeodb --gzip-rejections
cargo run --release --offline -p mappa-map-data --bin build_canonical_tiles -- \
  data/ign_bdtopo_paris_water.toml artifacts/world-water/fr-paris/water.mgeodb \
  artifacts/world-water/fr-paris/water.pmtiles
cargo run --release --offline -p mappa-map-data --bin audit_fixture -- \
  artifacts/world-water/fr-paris/water.pmtiles
```
