# 나주 도로·행정 지명 proof 측정 (Mac, 2026-09-24)

| 단계 | 측정 |
|---|---:|
| 나주시 원본 ZIP | 2,052,408 B |
| 변환 도로 중심선 GeoJSON | 2,492,774 B |
| 변환 도로면 GeoJSON | 4,139,454 B |
| SGIS 공식 ZIP / 변환 읍면동 GeoJSON | 269,032,521 / 573,196 B |
| MappaGeoDB v1 / provenance 포함 | 3,305,015 B |
| PMTiles / provenance 제외, z10–z15 | 560,768 B |
| 타일 수 / 인코딩 도로선·도로면·지명 수 | 79 / 32,353 (줌·타일 중복 포함) |
| z14.6 비동기 cold load | 24.701 ms |
| z14.6 이동 40프레임 중앙값 / p95 / p99 | 2.486 / 3.513 / 24.915 ms |
| 이동 중 미해결 프레임 / 실패 | 0 / 0 |

도로선을 타일 경계에서 바로 자른 초기 산출물은 PMTiles 550,216B, cold load 21.522ms, 이동 중앙값 2.287ms/p95 2.894ms/p99 27.113ms였다. 4px 여유 구간 뒤 도로 전용 PMTiles는 556,291B였고 허용 오차 초과 도로선 접점은 7개에서 0개로 줄었다. SGIS 지명 14개를 각 z12–z15에 더한 현재 PMTiles는 그 도로 전용 파일보다 4,477B 증가했다. 각 화면 측정은 1회 40프레임이므로 성능 개선/악화 판정용 반복 실험은 아니다.

`audit_canonical_tiles`로 모든 지역 타일을 디코드한 줌별 값이다. `decoded_mvt_bytes`는 압축이 풀린 MVT 합계로, PMTiles 파일 내부 바이트 기여량과 다르다.

| 줌 | 타일 | 디코드 MVT 합계 B | 도로선 | 도로면 | 지명 | 경계 정확 일치 | 1단위 이내 | 미일치 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 10 | 1 | 7,516 | 460 | 0 | 0 | 0 | 0 | 0 |
| 11 | 4 | 27,224 | 1,661 | 0 | 0 | 7 | 0 | 0 |
| 12 | 4 | 69,402 | 3,966 | 0 | 14 | 22 | 0 | 0 |
| 13 | 8 | 226,671 | 4,008 | 4,509 | 14 | 43 | 0 | 0 |
| 14 | 19 | 240,999 | 4,087 | 4,674 | 14 | 98 | 10 | 0 |
| 15 | 43 | 257,337 | 4,124 | 4,820 | 14 | 160 | 25 | 0 |

Mac Metal의 1200×720 도로·읍면동 지명 화면이다. iPhone 성능이나 건물이 많은 화면의 값으로 해석할 수 없다. 건물 포함 여부에 따른 용량·속도 비교는 원본 미확보로 측정하지 않았다. 3원천 GeoDB·PMTiles의 재현 해시는 아래 빌드 ID로 기록한다.

재현:

```sh
cargo run -p mappa-map-data --bin build_canonical_proof -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb
cargo run -p mappa-map-data --bin build_canonical_tiles -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb artifacts/map-v0.3c/naju-roads.pmtiles
cargo run -p mappa-map-data --bin audit_canonical_tiles -- data/sources.toml artifacts/map-v0.3c/naju-roads.pmtiles
MAPPA_DATASET=canonical-proof cargo run -p mappa-map-demo -- --street-demo 126.715 35.025 15.2
MAPPA_DATASET=canonical-proof cargo run -p mappa-map-demo -- --async-street-benchmark 126.715 35.025 14.6
```

빌드 ID는 본 proof의 두 산출물 SHA-256으로 고정한다: GeoDB `e7e9ca4a13af9ae93b3cd64f0cf1a6d632c1e1c1b15c4627bffe40e4f354591c`, PMTiles `6a0306adb6834d040e1a128cc9ce0134131b275f2d0cc173c3cde6570e2efb77`. 레이어는 도로선·도로면·읍면동 지명이며 압축 archive 내부의 줌별 byte accounting과 MLT 비교는 추가 계측 전에는 비워 둔다.
