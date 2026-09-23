# Benchmark v0.1 baseline

측정일: 2026-09-23. 환경: macOS 27.0 arm64, Rust 1.98.1, Cargo dev profile, PostgreSQL 17.11, PostGIS 3.6.4. 로컬 loopback DB. `cargo run -p mappa-harness` 결과이며 성능 합격선은 설정하지 않았다. 단일 실행의 wall-clock 측정이므로 다른 환경과 직접 비교하지 않는다.

| 항목 | 반복 | 측정 |
|---|---:|---:|
| coordinate → cell | 100,000 | 5,808 µs total |
| CreatePostRequest encode | 10,000 | 4,696 µs total |
| CreatePostRequest decode | 10,000 | 9,913 µs total |
| cell cache lookup | 100,000 | 783 µs total |
| DB cell snapshot query | 100 | 33,550 µs total |

| Posts | Binary frame bytes | 비교용 JSON array bytes | Binary 감소율 |
|---:|---:|---:|---:|
| 100 | 6,636 | 23,101 | 71.3% |
| 1,000 | 66,036 | 231,001 | 71.4% |

JSON baseline은 harness에서만 생성한다. 같은 필드를 담지만 binary는 frame/cell header를 포함하고 JSON은 posts array만 포함하므로 정확히 같은 envelope 비교는 아니다.

E2E 마지막 실행: 생성 요청 93 bytes, 생성 응답 56 bytes, 3×3 셀 조회 응답 980 bytes. 기준 셀 revision 7 → 글 3개 생성 후 10, 기준 글 수 7 → 10. 다른 셀 글은 제외되었고 서버 재시작 뒤 생성 글 재조회가 성공했다. 이 DB는 반복 실행된 테스트 글을 포함한다.
