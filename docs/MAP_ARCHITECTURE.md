# Mappa v0.3A map architecture (historical)

현재 기본 세계지도와 나주 canonical 실증의 레이어·타일 선택·렌더 순서는 [최신 지도 레이어 구조](MAP_LAYER_ARCHITECTURE.md)에 정리했다. 아래 내용은 초기 외부 데이터 비교 시안의 설계 기록이며 현재 기본 세계지도의 레이어 범위를 설명하지 않는다.

This document describes the earlier external-data comparison modes. The desktop demo now defaults to the [offline world map](WORLD_MAP.md), built by Mappa's Rust pipeline from public-domain Natural Earth source data. Select `MAPPA_DATASET=first-party` for only [Mappa field records](FIRST_PARTY_MAP.md), or `MAPPA_DATASET=legacy-osm` / `MAPPA_DATASET=public-naju` for the older comparison modes described below.

The map is an independent Rust branch of the existing iOS client. Its bundled data path has no tile URL, HTTP client, DNS lookup, or server dependency.

```text
MapCamera (Web Mercator f64, unwrapped x)
  → visible TileKey placements (+ one tile margin)
  → one of four LocalPmTiles archives (immutable mmap files, gzip MVT)
  → DecodedTile (land/green/water polygons, ranked roads, boundaries, named points)
  → PreparedTile (lyon triangle fills, line quads, city/station/civic markers)
  → bounded GPU tile cache
  → wgpu Metal (camera-relative per-tile uniforms)
```

Crates: `mappa-map-core` owns camera/projection/keys; `mappa-map-data` owns local source, decoder, and fixture builder; `mappa-map-render` owns geometry preparation, typed styles, GPU buffers, and WGSL; `mappa-map-demo` owns the macOS window, gestures, preview capture, and measurements. The map crates do not depend on PostgreSQL, Axum, or Mappa post queries. A future post overlay can call `MapCamera::world_to_screen`; map picking uses `screen_to_coordinate`. Post query policy remains in `mappa-client-core`.

Coordinates use f64 on CPU. Tile geometry remains local in 0..4096; each draw receives a camera-relative screen origin and scale in a 256-byte aligned uniform slot. X repeats across ±180° and Y clamps at the Web Mercator pole. Fractional zoom changes the uniform scale continuously while tile selection uses an integer zoom capped by the active archive. Above that archive's maximum zoom, vector geometry remains sharp but gains no detail.

The zoom demonstration opens four local archives on the same projection. The global Natural Earth 1:110m archive supplies z0–z4, and the bounded East Asia 1:10m archive supplies z5–z7. A Korea OSM archive supplies z8–z9 OSM coastline, lakes, vegetation, major roads, and cities; a capital-region OSM archive supplies z10–z12 the same terrain with smaller roads, stations, and civic points. Road classes are separate MVT layers and use different stroke widths. Small polygons and road links are filtered by display zoom to reduce overview clutter. Each regional archive is selected only when the full viewport fits inside its PMTiles header bounds; otherwise the next wider source supplies its parent tiles. Natural Earth country names come from the source label coordinates. OSM point names prefer the source Korean name. Screen-box collision filtering limits visible text. Above z12 the z12 geometry is enlarged; buildings and full-country street coverage are absent.

The fixture is an immutable file while mapped. Regenerate it only when the demo is stopped; replacing or truncating the mapped file could cause a process fault. Ocean is the background clear color. The writer clips polygons per tile with `geo` and tessellates holes with lyon's even-odd fill rule. It removes rings that collapse during MVT coordinate rounding. At overview zoom, coast strokes derive from Natural Earth land rings and omit tile clipping edges. The OSM coast source arrives as overlapping polygon chunks, so the renderer fills those without outlining chunk boundaries. Country boundaries use thinner screen-width line quads. Road segment outer wedges are closed with shared triangle indices, and line edges use a 0.5px shader coverage ramp.

The demo keeps decoded tiles under a 64 MiB estimated CPU payload limit and GPU meshes under a 128 MiB buffer limit; oldest entries are evicted first. These limits were raised after the z9 OSM screenshot exposed a missing visible tile under the prior 16/32 MiB limits. The offscreen zoom comparison rejects a frame if a visible tile remains unresolved or has been evicted; intentionally empty ocean tiles are excluded. GPU buffer bytes are counted; CPU estimates include decoded geometry payload, not allocator overhead or total process memory. A single live worker reads, decodes, and prepares tiles selected near the screen center; the UI uploads at most one completed tile per frame. A bounded request queue and desired-tile set skip stale requests before loading, but work already in progress is not interrupted. A missing-tile set avoids repeated misses. Redraws occur on input, resize, or tile completion rather than a continuous idle loop. There is no remote source implementation.

Typed styles in `mappa-map-render` share the same geometry: Arcade, Lagoon, Candy, Sunset. The desktop demo changes style with keys 1–4, pans with left drag, zooms around the pointer with wheel/trackpad scroll, and handles physical-pixel resize and scale-factor events. `--variants` outputs world and East Asia comparisons; `--views`, `--zoom-compare`, and `--benchmark` use the same GPU pipeline offscreen. `--street-demo` opens the live view near the center of the capital-region archive, and optional longitude/latitude/zoom arguments fix a comparison camera. `--capture` writes the same view to a PNG. The glyphon overlay draws actual country, city, station, and civic names and OSM attribution, with source rank priority and screen-box collision. A post marker overlay should be drawn after the basemap and kept separate from tile decoding. The global fixture has no road, park, or building data; the regional OSM fixtures have no building or game-node layer.

An optional `text-rnd` feature in the desktop demo uses glyphon/cosmic-text and system font fallback to render six writing-system samples on the same Metal target. This is a feasibility probe, not map labels or a collision engine.

Scope at the time of v0.3A: low-zoom world silhouettes and borders, bounded East Asia z5–z7 detail, Korea z8–z9 road network, and a bounded Seoul z10–z12 street/POI slice. There were no buildings, complete Korea street coverage, rotation, iOS map integration, or physical-device map results. The existing Apple Personal Team device registration gate remained blocked.
