# Map v0.3A measurements — 2026-09-24 KST

Status: **PARTIAL feasibility gate**. The renderer and four styles run through local PMTiles → MVT → Rust geometry → wgpu → **Apple M4 Metal**. Interaction and high-zoom cartography still need deeper verification before M11 PASS.

Environment: Mac mini Apple M4, macOS 27.0, `rustc 1.98.1`, `wgpu 30.0.1`, `pmtiles 0.24.0`, `mvt-reader 2.5.0`, `mlt-core 0.14.5` evaluation with default v1/tag-01 mode. PMTiles fixture: 209,232 bytes, z0–z4, 266 nonempty tiles. All timings below come from local commands and are not mobile results.

The initial `/usr/bin/time -l target/debug/mappa-map-demo --benchmark` run, before the hypercasual coastline layer, used a 1200×720 offscreen sRGB target and waited for GPU completion after each frame:

| Measurement | Result |
|---|---:|
| PMTiles open | 0.799 ms |
| Cold visible tile load (4 canonical tiles) | 16.651 ms |
| Cold first frame, open + load + render/GPU wait | 22.714 ms |
| Tile lookup total (cold + pan/zoom) | 1.748 ms |
| MVT decode total (cold + pan/zoom) | 4.374 ms |
| Polygon/line preparation total (cold + pan/zoom) | 9.216 ms |
| GPU upload total (cold + pan/zoom) | 1.244 ms |
| Warm offscreen frame median / p95 / p99 (120 frames) | 0.848 / 1.294 / 1.953 ms |
| Simulated pan offscreen median / p95 / p99 (40 frames) | 1.184 / 3.649 / 5.133 ms |
| Simulated zoom offscreen median / p95 / p99 (24 frames) | 1.138 / 4.192 / 8.648 ms |
| Tile requests / missing / failures | 71 / 2 / 0 |
| Raw/decode cache hits / GPU cache hits | 0 / 0 / 1009 |
| CPU estimated payload / GPU buffers | 798,393 / 1,220,716 bytes |
| CPU/GPU evictions | 0 / 0 |
| macOS maximum resident set size / peak memory footprint | 36,372,480 / 127,828,640 bytes |

The pan/zoom loops include loading and preparation but do **not** include a presented window frame or human gesture timing. They cannot be called interactive FPS. A live window run logged approximately 91 FPS for its first two seconds and approximately 100 FPS subsequently at idle, but controlled drag/zoom FPS and p95/p99 presentation times were not recorded. The benchmark exercises only a small z0–z4 fixture, so the 16 MiB CPU and 32 MiB GPU cache eviction paths did not activate. The CPU number is a geometry payload estimate rather than a process memory ceiling; the process figures above are measured separately.

## Hypercasual coastline iteration

After adding coast geometry and round caps, the same Apple M4 Metal offscreen command returned:

| Measurement | Result |
|---|---:|
| PMTiles open / cold visible tile load | 0.334 / 18.606 ms |
| Cold first frame, open + load + render/GPU wait | 24.418 ms |
| Polygon/line/coast preparation total (cold + pan/zoom) | 12.381 ms |
| Warm offscreen frame median / p95 / p99 | 1.037 / 1.420 / 1.679 ms |
| Simulated pan offscreen median / p95 / p99 | 1.516 / 4.637 / 7.329 ms |
| Simulated zoom offscreen median / p95 / p99 | 1.484 / 5.160 / 11.259 ms |
| CPU estimated payload / GPU buffers | 798,393 / 6,405,812 bytes |
| macOS maximum resident set size / peak memory footprint | 50,249,728 / 135,840,440 bytes |

The round-cap mesh raised GPU buffer use from 1,220,716 to 6,405,812 bytes on this small fixture. It remains below the 32 MiB demo budget here; larger high-zoom data needs its own cache and frame-time measurements. `--variants` rendered four real Metal PNGs at the world and East Asia cameras. The reference image's roads and decorative nodes are absent because the fixture has no corresponding geographic layers.

`cargo run -p mappa-map-data --features mlt-eval --bin compare_formats` converts three identical local MVT tiles to MLT v1/tag-01 and times 100 full decodes per format. Raw MVT bytes come from decompressed PMTiles. Gzip sizes are computed with the same `flate2` default compressor for both formats. MLT v2 features are disabled.

| Tile | MVT raw / gzip B | MLT v1 raw / gzip B | MVT decode µs | MLT decode µs | Features |
|---|---:|---:|---:|---:|---:|
| 0/0/0 | 49,769 / 25,907 | 16,605 / 15,187 | 3,087.41 | 2,802.72 | 2,806 |
| 3/6/3 | 3,609 / 2,511 | 1,674 / 1,681 | 216.07 | 236.73 | 178 |
| 4/13/6 | 610 / 565 | 527 / 504 | 35.79 | 44.50 | 12 |

MLT v1 is smaller for these samples, but the small tiles decode more slowly through the tested owned-row conversion. MVT remains the runnable baseline for v0.3A; v0.3B should compare the full target layer schema and direct GPU preparation before selecting a production format. Decoded allocations were not instrumented.

`target/debug/mappa-map-demo --variants` produced four Metal PNGs in `artifacts/map-v0.3a/`. `--views` produced world, East Asia, Korea, Europe, North America, antimeridian, and 2400×1440 2× Retina PNGs. Visual inspection found recognizable world orientation and joined antimeridian views. Korea zoom exposes the expected 1:110m detail limit. The map source has no HTTP tile implementation, so map tile requests are zero by code path; packet-level network tracing was not performed.

`cargo run -p mappa-map-demo --features text-rnd -- --text` produced `artifacts/map-v0.3a/text-rnd.png`. Visual inspection confirmed system-font glyphs for Latin (`Seoul`), Korean (`서울`), Japanese (`東京`), Chinese (`北京`), Cyrillic (`Москва`), and Arabic (`القاهرة`) on the Metal target. Arabic was shaped right-to-left and positioned at the right edge of its text run. This does not validate label collisions, font licensing for bundling, or iOS fallback.

Open checks: asynchronous worker/generation cancellation, bounded-cache eviction under larger data, controlled drag and scroll measurements on the actual window, and iOS Simulator integration. Physical iPhone installation remains blocked by the unchanged Personal Team device registration limit.
