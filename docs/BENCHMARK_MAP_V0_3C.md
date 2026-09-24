# 나주 도로 전용 proof 측정 (Mac, 2026-09-24)

| 단계 | 측정 |
|---|---:|
| 나주시 원본 ZIP | 2,052,408 B |
| 변환 도로 중심선 GeoJSON | 2,492,774 B |
| 변환 도로면 GeoJSON | 4,139,454 B |
| MappaGeoDB v1 / provenance 포함 | 3,301,071 B |
| PMTiles / provenance 제외, z10–z15 | 556,291 B |
| 타일 수 / 인코딩 road line+surface 수 | 65 / 32,297 (줌·타일 중복 포함) |
| z14.6 비동기 cold load | 22.458 ms |
| z14.6 이동 40프레임 중앙값 / p95 / p99 | 2.214 / 3.508 / 25.537 ms |
| 이동 중 미해결 프레임 / 실패 | 0 / 0 |

도로선을 타일 경계에서 바로 자른 직전 산출물은 PMTiles 550,216B, cold load 21.522ms, 이동 중앙값 2.287ms/p95 2.894ms/p99 27.113ms였다. 4px 여유 구간을 둔 뒤 PMTiles는 6,075B(1.10%) 증가했고 타일 경계의 허용 오차 초과 도로선 접점은 7개에서 0개로 줄었다. 각 화면 측정은 1회 40프레임이므로 성능 개선/악화 판정용 반복 실험은 아니다. 도로 중심선만 사용한 더 이른 산출물은 GeoDB 1,174,105B, PMTiles 198,821B, cold load 17.121ms, 이동 중앙값 1.831ms/p95 2.132ms/p99 21.745ms였다.

`audit_canonical_tiles`로 모든 지역 타일을 디코드한 줌별 값이다. `decoded_mvt_bytes`는 압축이 풀린 MVT 합계로, PMTiles 파일 내부 바이트 기여량과 다르다.

| 줌 | 타일 | 디코드 MVT 합계 B | 도로선 | 도로면 | 경계 정확 일치 | 1단위 이내 | 미일치 |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 10 | 1 | 7,516 | 460 | 0 | 0 | 0 | 0 |
| 11 | 4 | 27,224 | 1,661 | 0 | 7 | 0 | 0 |
| 12 | 4 | 68,755 | 3,966 | 0 | 22 | 0 | 0 |
| 13 | 6 | 225,718 | 4,008 | 4,509 | 43 | 0 | 0 |
| 14 | 14 | 239,562 | 4,087 | 4,674 | 98 | 10 | 0 |
| 15 | 36 | 255,640 | 4,124 | 4,820 | 160 | 25 | 0 |

Mac Metal의 1200×720 도로 전용 화면이다. iPhone 성능이나 건물이 많은 화면의 값으로 해석할 수 없다. 건물 포함 여부에 따른 용량·속도 비교는 원본 미확보로 측정하지 않았다. GeoDB·PMTiles를 같은 입력으로 두 번 빌드한 SHA-256이 각각 일치했다.

재현:

```sh
cargo run -p mappa-map-data --bin build_canonical_proof -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb
cargo run -p mappa-map-data --bin build_canonical_tiles -- data/sources.toml artifacts/map-v0.3c/naju-roads.mgeodb artifacts/map-v0.3c/naju-roads.pmtiles
cargo run -p mappa-map-data --bin audit_canonical_tiles -- data/sources.toml artifacts/map-v0.3c/naju-roads.pmtiles
MAPPA_DATASET=canonical-proof cargo run -p mappa-map-demo -- --street-demo 126.715 35.025 15.2
MAPPA_DATASET=canonical-proof cargo run -p mappa-map-demo -- --async-street-benchmark 126.715 35.025 14.6
```

빌드 ID는 본 proof의 두 산출물 SHA-256으로 고정한다: GeoDB `4c6d6efa4b275354c8e846a68947a7164bd90992c6849311137aaef6f69ee4a9`, PMTiles `1f1fe3be8d8672a2a971d60b07bb0c8c2c85585a6ea35747fc0785cf74f20c06`. 레이어는 도로뿐이며 압축 archive 내부의 줌별 byte accounting과 MLT 비교는 추가 계측 전에는 비워 둔다.
