#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: scripts/build_au_tas_roads.sh SNAPSHOT.toml DOWNLOAD_DATE" >&2
  exit 2
fi

snapshot=$1
download_date=$2
pack_dir=artifacts/world-roads/au-tas

cargo build --release -p mappa-map-data \
  --bin build_au_tas_roads \
  --bin build_canonical_tiles \
  --bin audit_canonical_tiles \
  --bin make_au_tas_catalog

target/release/build_au_tas_roads "$snapshot" "$download_date" "$pack_dir" data

for manifest in data/au_tas_roads_*.toml; do
  region=${manifest#data/au_tas_roads_}
  region=${region%.toml}
  target/release/build_canonical_tiles \
    "$manifest" "$pack_dir/$region.mgeodb" "$pack_dir/$region.pmtiles"
  target/release/audit_canonical_tiles \
    "$manifest" "$pack_dir/$region.pmtiles" > "$pack_dir/$region-tile-audit.log"
done

target/release/make_au_tas_catalog \
  data "$pack_dir" assets/map/au_tas_regional_packs.toml
