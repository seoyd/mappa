# Norway NVDB V4 source assessment — 2026-09-25

## Confirmed source and rights

The Norwegian Public Roads Administration's [NVDB API Les V4](https://nvdb-docs.atlas.vegvesen.no/nvdbapil/v4/introduksjon/Oversikt/) publishes national road-network data under [NLOD](https://data.norge.no/nlod/no/1.0) with source attribution and an indication when data is changed. The terms permit copying, modifying and redistribution; they do not impose ODbL-style database share-alike. Its [road-network endpoint](https://nvdb-docs.atlas.vegvesen.no/nvdbapil/v4/Vegnett/) exposes paged road-link sequences and geometries. This is a candidate **build-time** source. Mappa's runtime must continue to use local files only.

The V3 API is being retired; the [official migration guide](https://nvdb-docs.atlas.vegvesen.no/nvdbapil/Migrering/) points to V4. The V4 request requires an identifying `X-Client` header per [official authentication guidance](https://nvdb-docs.atlas.vegvesen.no/nvdbapil/v4/Autentisering/).

## Observed access probe

On 2026-09-25, a GET to `https://nvdbapiles.atlas.vegvesen.no/vegnett/api/v4/veglenkesekvenser?kommune=5001&antall=1&inkluderAntall=true` with `X-Client: MappaOfflineMapBuilder` returned HTTP 200 and 2,176 JSON bytes. The stored [one-object response](../assets/map/source/public/no_nvdb_v4_kommune5001_one.json) has SHA-256 `105bf4de8671708a6e6aa85f3a9c5b4dac71ca5c7847ef284745c05ceafbf701`. Its response metadata says `antall=28932` for this municipality query; this is a road-link-sequence count in a live response, **not** a verified road-line count, length, national coverage, or fixed inventory. The returned geometry is `LINESTRING Z`, `srid=5973`, with link and node identifiers and a `typeVeg` attribute. The [NVDB CRS documentation](https://nvdb-docs.atlas.vegvesen.no/datafangst/Begrensninger/) identifies 5973 as nationwide UTM33N.

The V4 documentation's example still includes `sortert=false`; the live API returned HTTP 400 with `Ukjente parametre: sortert`. The successful probe omitted that parameter. A full harvester must follow the response's `metadata.neste` cursor, and verify counts, duplicated IDs, checksum and schema per retrieved page.

## Gate before use in the canonical map

No Norwegian road has yet been added to Mappa. Required: enumerate stable national or regional pages; preserve byte-level source provenance; transform EPSG:5973 with independent control points; classify vehicle-accessible roads from source attributes without guessing; audit accepted and rejected link records; build and decode regional tiles; inspect national and boundary coverage; record NLOD attribution. Availability and license alone do not establish coverage or positional accuracy.
