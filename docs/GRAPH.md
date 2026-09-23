# Implementation graph

Dependency: `G0 → G1 → {G2,G3} → G4 → G5 → G6 → G7 → G8`. Status는 해당 gate 결과가 확인된 뒤 변경한다.

| Node | Input | Output | Dependency | Invariant | Test / Gate | Status |
|---|---|---|---|---|---|---|
| G0 Workspace | empty repository | Cargo workspace | none | stable Rust, all members build | fmt/check/clippy/test | PASS |
| G1 Domain | E7 coordinates, text | typed Post | G0 | WGS84 bounds, original text | unit tests | PASS |
| G2 Spatial | Coordinate | z14 CellId, neighbors | G1 | reversible ID, x wrap, y bound | unit tests | PASS |
| G3 Protocol | typed messages | v1 binary frames | G1,G2 | strict decode, no panic | roundtrip/corrupt tests | PASS |
| G4 Storage | create/query | PostGIS rows, revision | G2,G3 | insert + revision atomic | migration and DB harness | PASS |
| G5 Axum | binary HTTP | create/query response | G4 | input bound, binary errors | process harness | PASS |
| G6 Client core | camera idle, cache | one batch request | G5 contract | moving = zero request | scheduler tests | PASS |
| G7 E2E harness | local DB/server | A create, B query | G5,G6 | same post, persistence | run harness | PASS |
| G8 Benchmark | running harness | baseline numbers | G7 | measured, not estimated | run and record | PASS |

2026-09-23: `cargo fmt --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace` 통과. 로컬 PostgreSQL 17.11/PostGIS 3.6.4에서 `cargo run -p mappa-harness` 통과. 별도 `mappa-server` 프로세스의 `/health` HTTP 200 확인. 상세 수치는 [BENCHMARK_V0_1.md](BENCHMARK_V0_1.md)에 기록.
