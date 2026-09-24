use mappa_map_data::builder::first_party::{build, inspect};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

type DynError = Box<dyn Error + Send + Sync>;

fn number(raw: &str) -> Result<f64, DynError> {
    let value: f64 = raw.parse()?;
    if !value.is_finite() {
        return Err("number must be finite".into());
    }
    Ok(value)
}

fn position(raw: &str) -> Result<Vec<f64>, DynError> {
    let (lon, lat) = raw.split_once(',').ok_or("position must be LON,LAT")?;
    Ok(vec![number(lon)?, number(lat)?])
}

fn append(source: &Path, feature: Value) -> Result<(), DynError> {
    inspect(source)?;
    let mut data: Value = serde_json::from_slice(&std::fs::read(source)?)?;
    data["features"]
        .as_array_mut()
        .ok_or("survey features must be an array")?
        .push(feature);
    let temporary = source.with_extension("geojson.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&data)?)?;
    if let Err(error) = inspect(&temporary) {
        std::fs::remove_file(&temporary)?;
        return Err(error);
    }
    std::fs::rename(temporary, source)?;
    Ok(())
}

fn run() -> Result<(), DynError> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build") if args.len() == 4 => {
            let stats = build(Path::new(&args[2]), Path::new(&args[3]))?;
            println!("recorded_areas={} recorded_boundaries={} recorded_roads={} recorded_places={} generated_tiles={}", stats.areas, stats.boundaries, stats.roads, stats.places, stats.tiles);
        }
        Some("inspect") if args.len() == 3 => {
            let stats = inspect(Path::new(&args[2]))?;
            println!("recorded_areas={} recorded_boundaries={} recorded_roads={} recorded_places={}", stats.areas, stats.boundaries, stats.roads, stats.places);
        }
        Some("add-place") if args.len() == 9 => {
            let coordinates = vec![number(&args[5])?, number(&args[6])?];
            append(Path::new(&args[2]), json!({
                "type": "Feature",
                "properties": {
                    "origin": "mappa-field-survey",
                    "method": "field-note",
                    "kind": &args[3],
                    "name": &args[4],
                    "accuracy_m": number(&args[7])?,
                    "observed_at": &args[8]
                },
                "geometry": {"type": "Point", "coordinates": coordinates}
            }))?;
            println!("record appended");
        }
        Some("add-road" | "add-line") if args.len() >= 9 => {
            let coordinates = args[7..].iter().map(|p| position(p)).collect::<Result<Vec<_>, _>>()?;
            append(Path::new(&args[2]), json!({
                "type": "Feature",
                "properties": {
                    "origin": "mappa-field-survey",
                    "method": "field-note",
                    "kind": &args[3],
                    "name": &args[4],
                    "accuracy_m": number(&args[5])?,
                    "observed_at": &args[6]
                },
                "geometry": {"type": "LineString", "coordinates": coordinates}
            }))?;
            println!("record appended");
        }
        Some("add-area") if args.len() >= 11 => {
            let ring = args[7..].iter().map(|p| position(p)).collect::<Result<Vec<_>, _>>()?;
            append(Path::new(&args[2]), json!({
                "type": "Feature",
                "properties": {
                    "origin": "mappa-field-survey",
                    "method": "field-note",
                    "kind": &args[3],
                    "name": &args[4],
                    "accuracy_m": number(&args[5])?,
                    "observed_at": &args[6]
                },
                "geometry": {"type": "Polygon", "coordinates": [ring]}
            }))?;
            println!("record appended");
        }
        _ => return Err("usage: first_party_map inspect SURVEY.geojson | build SURVEY.geojson OUTPUT.pmtiles | add-place SURVEY.geojson KIND NAME LON LAT ACCURACY_M UTC_TIME | add-line SURVEY.geojson KIND NAME ACCURACY_M UTC_TIME LON,LAT LON,LAT ... | add-area SURVEY.geojson KIND NAME ACCURACY_M UTC_TIME LON,LAT LON,LAT LON,LAT LON,LAT ...".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("first-party map: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_area_does_not_replace_survey() {
        let path = std::env::temp_dir().join(format!(
            "mappa-first-party-cli-{}-{}.geojson",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let original = br#"{"type":"FeatureCollection","features":[]}"#;
        std::fs::write(&path, original).unwrap();
        let result = append(
            &path,
            json!({
                "type": "Feature",
                "properties": {
                    "origin": "mappa-field-survey",
                    "method": "field-note",
                    "kind": "land",
                    "name": "Unclosed",
                    "accuracy_m": 5,
                    "observed_at": "2026-09-24T00:00:00Z"
                },
                "geometry": {"type": "Polygon", "coordinates": [[[127.0,37.0],[127.001,37.0],[127.001,37.001],[127.0,37.001]]]}
            }),
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}
