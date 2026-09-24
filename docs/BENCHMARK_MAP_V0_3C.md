# 나주 도로·행정 지명·수계·수목 proof 측정 (Mac, 2026-09-24)

| 단계 | 측정 |
|---|---:|
| 나주시 원본 ZIP | 2,052,408 B |
| 변환 도로 중심선 GeoJSON | 2,492,774 B |
| 변환 도로면 GeoJSON | 4,139,454 B |
| SGIS 공식 ZIP / 변환 읍면동 GeoJSON | 269,032,521 / 573,196 B |
| ESA 2021 나주 COG crop / 물 GeoJSON / 수목 GeoJSON | 1,170,362 / 181,888 / 2,333,633 B |
| MappaGeoDB v1 / provenance 포함 | 5,245,737 B |
| PMTiles / provenance 제외, z10–z15 | 742,590 B |
| 타일 수 / 인코딩 feature 수 | 149 / 38,534 (줌·타일 중복 포함) |
| z14.6 비동기 cold load | 34.789 ms |
| z14.6 이동 40프레임 중앙값 / p95 / p99 | 4.175 / 6.274 / 22.430 ms |
| 이동 중 미해결 프레임 / 실패 | 0 / 0 |

도로선을 타일 경계에서 바로 자른 초기 산출물은 PMTiles 550,216B, cold load 21.522ms, 이동 중앙값 2.287ms/p95 2.894ms/p99 27.113ms였다. 4px 여유 구간 뒤 도로 전용 PMTiles는 556,291B였고 허용 오차 초과 도로선 접점은 7개에서 0개로 줄었다. SGIS 지명까지 포함한 이전 PMTiles는 560,768B, 같은 카메라에서 cold load 24.701ms, 이동 중앙값/p95/p99 2.486/3.513/24.915ms였다. 수계·수목 추가 후 archive는 181,822B 증가했고 같은 카메라에서 이동 중앙값과 p95가 각각 4.175ms, 6.274ms로 늘었다. 각 화면 측정은 1회 40프레임이므로 반복 실험에 의한 확정 성능 판정은 아니다.

`audit_canonical_tiles`로 모든 지역 타일을 디코드한 줌별 값이다. `decoded_mvt_bytes`는 압축이 풀린 MVT 합계로, PMTiles 파일 내부 바이트 기여량과 다르다.

| 줌 | 타일 | 디코드 MVT 합계 B | 도로선 | 도로면 | 수면 | 수목 | 지명 | 경계 정확 일치 | 1단위 이내 | 미일치 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 10 | 1 | 16,581 | 460 | 0 | 16 | 0 | 0 | 0 | 0 | 0 |
| 11 | 4 | 37,539 | 1,661 | 0 | 37 | 0 | 0 | 7 | 0 | 0 |
| 12 | 4 | 80,224 | 3,966 | 0 | 55 | 0 | 14 | 22 | 0 | 0 |
| 13 | 11 | 238,718 | 4,008 | 4,509 | 92 | 0 | 14 | 43 | 0 | 0 |
| 14 | 30 | 411,128 | 4,087 | 4,674 | 99 | 2,777 | 14 | 98 | 10 | 0 |
| 15 | 99 | 449,760 | 4,124 | 4,820 | 117 | 2,988 | 14 | 160 | 25 | 0 |

Mac Metal의 1200×720 도로·읍면동 지명·ESA 수계·수목 화면이다. iPhone 성능이나 건물이 많은 화면의 값으로 해석할 수 없다. 건물 포함 여부에 따른 용량·속도 비교는 원본 미확보로 측정하지 않았다. 5개 manifest 입력(독립 제공기관 3개) GeoDB·PMTiles의 재현 해시는 아래 빌드 ID로 기록한다.

재현:

```sh
cargo run -p mappa-map-data --bin build_canonical_proof -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb
cargo run -p mappa-map-data --bin build_canonical_tiles -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb artifacts/map-v0.3c/naju-roads.pmtiles
cargo run -p mappa-map-data --bin audit_canonical_tiles -- data/sources.toml artifacts/map-v0.3c/naju-roads.pmtiles
cargo run -p mappa-map-data --bin audit_canonical_pipeline -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb artifacts/map-v0.3c/naju-roads.pmtiles
MAPPA_DATASET=canonical-proof cargo run -p mappa-map-demo -- --street-demo 126.715 35.025 15.2
MAPPA_DATASET=canonical-proof cargo run -p mappa-map-demo -- --async-street-benchmark 126.715 35.025 14.6
```

빌드 ID는 본 proof의 두 산출물 SHA-256으로 고정한다: GeoDB `282dda6611c3f90883249afdebe97b4415020e1345fd38c45536f06d8c161367`, PMTiles `e94dcd1e39cc970e40db39f96e65ade433db59d97fb002f5cbd56ca73c8fccd1`. 레이어는 도로선·도로면·읍면동 지명·영구수면·수목 피복이다. 압축 archive 내부의 레이어별 byte accounting과 MLT 비교, peak 메모리, iPhone 성능은 아직 측정하지 않았다.
