# Rebuilding the Korea offline OSM slices

These commands are build-time data preparation. The Mappa app reads only local files. Use GDAL 3.13.3's OSM driver with [`osmconf.ini`](../assets/map/source/osmconf.ini); that config exposes roads, railways, civic places, water, land cover, and Korean names as attributes. The source is the dated [Geofabrik South Korea extract](https://download.geofabrik.de/asia/south-korea.html), not the changing `latest` alias.

```sh
curl -L --fail -o /tmp/mappa-south-korea-260922.osm.pbf https://download.geofabrik.de/asia/south-korea-260922.osm.pbf
shasum -a 256 /tmp/mappa-south-korea-260922.osm.pbf

OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-korea-roads.geojson /tmp/mappa-south-korea-260922.osm.pbf lines -spat 123.75 31.9 133.6 41.0 -clipsrc 123.75 31.9 133.6 41.0 -where "highway IN ('motorway','motorway_link','trunk','trunk_link','primary','primary_link','secondary','secondary_link')" -lco COORDINATE_PRECISION=6
OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-korea-cities.geojson /tmp/mappa-south-korea-260922.osm.pbf points -spat 123.75 31.9 133.6 41.0 -where "place IN ('city','town')" -lco COORDINATE_PRECISION=6
printf '%s\n' '{"type":"FeatureCollection","features":[]}' > /tmp/mappa-osm-empty-areas.geojson

OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-roads.geojson /tmp/mappa-south-korea-260922.osm.pbf lines -spat 125.5 36.4 128.5 38.8 -clipsrc 125.5 36.4 128.5 38.8 -where "highway IN ('motorway','motorway_link','trunk','trunk_link','primary','primary_link','secondary','secondary_link','tertiary','tertiary_link','residential','unclassified','living_street','pedestrian')" -lco COORDINATE_PRECISION=6
OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-points.geojson /tmp/mappa-south-korea-260922.osm.pbf points -spat 125.5 36.4 128.5 38.8 -where "railway IN ('station','halt') OR amenity IN ('townhall','police','fire_station','post_office','courthouse') OR office='government' OR place IN ('city','town')" -lco COORDINATE_PRECISION=6
OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-areas.geojson /tmp/mappa-south-korea-260922.osm.pbf multipolygons -spat 125.5 36.4 128.5 38.8 -where "railway IN ('station','halt') OR amenity IN ('townhall','police','fire_station','post_office','courthouse') OR office='government'" -lco COORDINATE_PRECISION=6
```

The PBF SHA-256 must be `32edd3a891ffd217b673c437af000f68a255f1b662ea6c3213c86d949a1dccc4`.

For the street-scale coast, download the static [OSM land polygons](https://osmdata.openstreetmap.de/data/land-polygons.html) package. Its split polygons overlap, so the renderer fills them without outlining each chunk. The package used here contains coastline data dated 2026-09-23T00:00:00Z; its ZIP SHA-256 is `60b2d5ada8a32e01dc6d0a17466086651056798f36eee324b73d0e6b46f3ffe3`. This is a build input of about 928 MB, not bundled with the app.

```sh
curl -L --fail -o /tmp/mappa-land-polygons-split-4326.zip https://osmdata.openstreetmap.de/download/land-polygons-split-4326.zip
shasum -a 256 /tmp/mappa-land-polygons-split-4326.zip
unzip -q /tmp/mappa-land-polygons-split-4326.zip -d /tmp/mappa-coast-source
ogr2ogr -f GeoJSON /tmp/mappa-osm-land-korea.geojson /tmp/mappa-coast-source/land-polygons-split-4326/land_polygons.shp land_polygons -spat 123.75 31.9 133.6 41.0 -clipsrc 123.75 31.9 133.6 41.0 -lco COORDINATE_PRECISION=6

OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-korea-water.geojson /tmp/mappa-south-korea-260922.osm.pbf multipolygons -spat 123.75 31.9 133.6 41.0 -clipsrc 123.75 31.9 133.6 41.0 -where "natural='water' OR waterway='riverbank' OR landuse='reservoir'" -lco COORDINATE_PRECISION=6
OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-korea-green.geojson /tmp/mappa-south-korea-260922.osm.pbf multipolygons -spat 123.75 31.9 133.6 41.0 -clipsrc 123.75 31.9 133.6 41.0 -where "leisure='park' OR landuse IN ('forest','grass','meadow','recreation_ground') OR natural IN ('wood','scrub','heath','grassland')" -lco COORDINATE_PRECISION=6

OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-water.geojson /tmp/mappa-south-korea-260922.osm.pbf multipolygons -spat 125.5 36.4 128.5 38.8 -clipsrc 125.5 36.4 128.5 38.8 -where "natural='water' OR waterway='riverbank' OR landuse='reservoir'" -lco COORDINATE_PRECISION=6
OSM_CONFIG_FILE=assets/map/source/osmconf.ini ogr2ogr -f GeoJSON /tmp/mappa-osm-green.geojson /tmp/mappa-south-korea-260922.osm.pbf multipolygons -spat 125.5 36.4 128.5 38.8 -clipsrc 125.5 36.4 128.5 38.8 -where "leisure='park' OR landuse IN ('forest','grass','meadow','recreation_ground') OR natural IN ('wood','scrub','heath','grassland')" -lco COORDINATE_PRECISION=6
```

The resulting GeoJSON files feed the two Rust `build_street_fixture` commands in [MAP_DATA.md](MAP_DATA.md). Natural Earth 1:10m remains an input only for country boundaries in these OSM archives. GDAL reported several non-closed rings and topology conflicts while extracting green polygons; this preview therefore cannot claim complete land-cover coverage. The OSM PMTiles are [ODbL-licensed](../assets/map/OSM_LICENSE.md) and must retain attribution when displayed.
