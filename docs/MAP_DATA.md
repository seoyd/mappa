# v0.3A offline world fixture

Source: Natural Earth 1:110m GeoJSON from [`nvkelso/natural-earth-vector`](https://github.com/nvkelso/natural-earth-vector), pinned to commit `ca96624a56bd078437bca8184e78163e5039ad19` (downloaded 2026-09-24 KST). [Natural Earth terms](https://www.naturalearthdata.com/about/terms-of-use/) place this vector data in the public domain; attribution is optional. The world PMTiles itself contains no OSM data.

| Source file | SHA-256 | Bytes | Output layer |
|---|---|---:|---|
| `ne_110m_land.geojson` | `9e0729ee253ca7d7a5c4ae9395fb1902264c5377c52e224d13dd85010e2835d9` | 138,160 | land |
| `ne_110m_lakes.geojson` | `eb02ecc86c82004fccbf979058bfabbbd6c2d07968c7844d38eb1c9152d2ffc9` | 36,648 | water |
| `ne_110m_admin_0_boundary_lines_land.geojson` | `d42479fd79552cca4eec7f85fcdca717a790d29ff06be7676f1af0568c6d3f7c` | 340,010 | boundary |

The Rust builder projects to Web Mercator, clips to z0–z4 tiles, encodes MVT extent 4096, and writes `assets/map/world_110m.pmtiles` with PMTiles v3 and gzip tile compression. It produced 266 nonempty tiles, 14,716 encoded features, and 209,232 bytes. Fixture SHA-256: `4dbbeb056a41ff682d61c1ff44bf419e45307bcda301fe23c39e2162647f4dd7`. Empty ocean tiles are absent and render as background. The fixture is committed because it is small and enables a one-command offline demo.

Rebuild (when no reader has the file mapped):

```sh
cargo run -p mappa-map-data --bin build_fixture
```

The world fixture is a renderer feasibility asset, not a full detailed basemap. At Korea-scale zoom it lacks coastline detail, roads, and city labels; the bounded detail fixture below supplies those layers in East Asia. Other regions and street-level precision still need their own detailed sources.

## Bounded East Asia detail fixture

An additional offline PMTiles, `assets/map/east_asia_10m.pmtiles`, uses the same pinned Natural Earth commit and its 1:10m GeoJSON. These are generalized cartographic data at 1:10 million scale, not 10-metre positional measurements. The build took the z5 tile range `[24,30) × [9,15)` (90°E–157.5°E, approximately 11.18°N–61.61°N), generated z5–z7 tiles, and selected roads with source `scalerank ≤ 5` and populated places with `SCALERANK ≤ 6`. z5 contains land/lakes/country boundaries. z6–z7 add actual road lines and source-named city points. No source geometry was invented.

| Source GeoJSON | SHA-256 | Bytes |
|---|---|---:|
| `ne_10m_land.geojson` | `1ac90796408bc6ad6911d69448485d3c4dbf2190370080368a09976e1c9f7416` | 10,157,965 |
| `ne_10m_lakes.geojson` | `2d036f53dedec578001c5c30c2959ee7d4eebc1306900fa4367c49929ec8f2d9` | 5,043,554 |
| `ne_10m_admin_0_boundary_lines_land.geojson` | `74d9c16229c095fde65943a9919e337682f044bcebccb120764f38edf3b70f4a` | 2,284,669 |
| `ne_10m_roads.geojson` | `66a0c7b438e92fd124822cc5921cfa11042f48c294ade5e0f03f2c6640fd0248` | 50,474,096 |
| `ne_10m_populated_places.geojson` | `9b8e3de09048ef00dfc70357dbb9fa324493f214b5e0ae4daf1aa79a8d10116b` | 19,359,003 |

These five static sources came from the `geojson/` directory of [`nvkelso/natural-earth-vector` at commit `ca96624a56bd078437bca8184e78163e5039ad19`](https://github.com/nvkelso/natural-earth-vector/tree/ca96624a56bd078437bca8184e78163e5039ad19/geojson). To rebuild after obtaining the exact files into one local directory:

```sh
cargo run -p mappa-map-data --bin build_detail_fixture -- /path/to/10m-geojson assets/map/east_asia_10m.pmtiles 5 7 24 9 30 15 5 6
```

The resulting 589 nonempty tiles contain 115,469 encoded features and occupy 1,174,286 bytes. SHA-256: `c297f8acde44786103789f864c8f85c74c823faa4e16cb4a3aa407fdb452d733`. The full upstream GeoJSON files are not needed at runtime; the bundled PMTiles is. Outside this region, the demo chooses the world fixture. Above z7 it enlarges z7 detail rather than presenting street or building accuracy. Buildings are absent from these sources.

## Country labels and Korea OSM detail

Country labels come from `ne_110m_admin_0_countries.geojson` at the same pinned Natural Earth commit. The demo reads the source `LABEL_X`, `LABEL_Y`, `LABELRANK`, and Korean name fields; it does not invent label coordinates. The bundled source is 838,726 bytes, SHA-256 `6866c877d39cba9c357620878839b336d569f8c662ea6c3213c86d949a1dccc4`.

The two further offline archives use the dated [Geofabrik South Korea OSM extract](https://download.geofabrik.de/asia/south-korea.html), `south-korea-260922.osm.pbf`, containing OSM data through 2026-09-22T20:22:59Z. The downloaded source was 287,765,288 bytes, SHA-256 `32edd3a891ffd217b673c437af000f68a255f1b662ea6c3213c86d949a1dccc4`. Their coastlines come from the static [OSM land polygons](https://osmdata.openstreetmap.de/data/land-polygons.html) package, whose source date is 2026-09-23T00:00:00Z and ZIP SHA-256 is `60b2d5ada8a32e01dc6d0a17466086651056798f36eee324b73d0e6b46f3ffe3`. The package and large intermediate GeoJSON files are build inputs, not runtime requirements. Runtime map requests read only the bundled local PMTiles and country label file; there is no external map API.

| Archive | Source area and content | z range | Tiles | Features | Bytes | SHA-256 |
|---|---|---:|---:|---:|---:|---|
| `korea_osm_roads.pmtiles` | OSM coastline, water, parks/forest, ranked roads and cities/towns; Natural Earth national borders | 8–9 | 173 | 202,168 | 9,972,650 | `5c706a23cc4f3c57a9260bed2464e2220fcaad61981ae270f98c304d6eb02084` |
| `seoul_osm_streets.pmtiles` | Same OSM terrain, more local roads, stations and public institutions | 10–12 | 840 | 637,148 | 19,912,318 | `5cf71d570c2659685a95c1db0c3baa8b8fc647b1009aa71ea80693a7e125a68a` |

The z8 tile rectangle is `[216,223) × [96,105)`; the z10 street rectangle is `[870,877) × [394,400)`. The app uses each archive only when the whole viewport is inside its bounds, then falls back to the wider archive. OSM station points use `railway=station|halt`. Civic markers use named `amenity=townhall|police|fire_station|post_office|courthouse` or `office=government`. Points and named area centroids come from source geometries. The categories and zoom thresholds are rendering filters; they do not create map objects. At z8–z9, road links are omitted at overview scale. Small land, water and green polygons are filtered by projected pixel area per zoom; the original geometry remains the only source of displayed shapes. Green extraction reported a few GDAL topology warnings. Country and civic coverage outside the bounded slices remains incomplete. The archive audit decoded all 1,013 nonempty OSM tiles with 0 invalid geometries.

The two OSM-derived PMTiles databases are distributed under [ODbL 1.0](../assets/map/OSM_LICENSE.md), with visible `© OpenStreetMap contributors` attribution in the demo. The source's completeness and location precision vary by feature. These maps are for visual exploration, not authoritative street addressing.

Rebuild with GDAL's OSM driver and the checked-in [`osmconf.ini`](../assets/map/source/osmconf.ini). [Exact extraction commands](OSM_EXTRACT.md) and the two Rust builder commands are recorded for reproducibility:

```sh
cargo run -p mappa-map-data --bin build_street_fixture -- /path/to/10m-geojson /path/to/osm-land.geojson /path/to/korea-water.geojson /path/to/korea-green.geojson /path/to/korea-roads.geojson /path/to/korea-cities.geojson /path/to/empty-areas.geojson assets/map/korea_osm_roads.pmtiles 8 9 216 96 223 105
cargo run -p mappa-map-data --bin build_street_fixture -- /path/to/10m-geojson /path/to/osm-land.geojson /path/to/seoul-water.geojson /path/to/seoul-green.geojson /path/to/seoul-roads.geojson /path/to/seoul-points.geojson /path/to/seoul-areas.geojson assets/map/seoul_osm_streets.pmtiles 10 12 870 394 877 400
cargo run -p mappa-map-data --bin audit_fixture -- assets/map/korea_osm_roads.pmtiles
cargo run -p mappa-map-data --bin audit_fixture -- assets/map/seoul_osm_streets.pmtiles
```
