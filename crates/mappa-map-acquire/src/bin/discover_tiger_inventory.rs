//! Pin the county files actually published in one official Census directory.
//! This is a build-time tool; the map runtime never requests the directory.

use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fs, path::Path};

fn publisher_files(html: &str, state: &str, class: &str) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    let prefix = format!("tl_2025_{state}");
    let suffix = format!("_{class}.zip");
    for rest in html.split("href=\"").skip(1) {
        let Some((href, after)) = rest.split_once('"') else {
            return Err("unterminated publisher link".into());
        };
        if !href.starts_with(&prefix) {
            continue;
        }
        let Some(county) = href
            .strip_prefix(&prefix)
            .and_then(|name| name.strip_suffix(&suffix))
        else {
            return Err(format!("unexpected publisher file name: {href}"));
        };
        if county.len() != 3 || !county.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("invalid county FIPS in publisher link: {href}"));
        }
        if !after.starts_with(&format!(">{href}</a>")) {
            return Err(format!("publisher link text differs from href: {href}"));
        }
        if !files.insert(href.to_owned()) {
            return Err(format!("duplicate publisher link: {href}"));
        }
    }
    if files.is_empty() || files.len() > 300 {
        return Err(format!(
            "unexpected file count for state {state}: {}",
            files.len()
        ));
    }
    Ok(files)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let (class, directory) = match args.get(1).map(String::as_str) {
        Some("--roads") if args.len() == 5 => ("roads", "ROADS"),
        Some("--areawater") if args.len() == 5 => ("areawater", "AREAWATER"),
        _ => {
            return Err(
                "usage: discover_tiger_inventory [--roads|--areawater] STATE_FIPS OBSERVED_DATE OUTPUT.txt"
                    .into(),
            );
        }
    };
    let state = &args[2];
    if state.len() != 2 || !state.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("state FIPS must be exactly two digits".into());
    }
    let date = &args[3];
    if date.len() != 10
        || !date.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return Err("observed date must be YYYY-MM-DD".into());
    }
    let url = format!("https://www2.census.gov/geo/tiger/TIGER2025/{directory}/");
    let client = reqwest::Client::builder()
        .user_agent("Mappa build-time official source inventory")
        .build()?;
    let mut response = client.get(&url).send().await?.error_for_status()?;
    if response.url().as_str() != url {
        return Err("publisher directory redirected".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err("publisher directory exceeds 4 MiB".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let html = std::str::from_utf8(&bytes)?;
    if !html.contains(&format!("Index of /geo/tiger/TIGER2025/{directory}")) {
        return Err("unexpected publisher directory title".into());
    }
    let files = publisher_files(html, state, class)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let output = Path::new(&args[4]);
    fs::create_dir_all(output.parent().ok_or("output needs a parent directory")?)?;
    let temporary = output.with_extension("part");
    let mut inventory = format!(
        "# Official Census TIGER2025/{directory} directory; observed {date}; listing SHA-256 {digest}.\n# {url}\n"
    );
    for file in &files {
        inventory.push_str(file);
        inventory.push('\n');
    }
    fs::write(&temporary, inventory)?;
    fs::rename(&temporary, output)?;
    println!(
        "inventory={} state={} class={} files={} listing_sha256={digest}",
        output.display(),
        state,
        class,
        files.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::publisher_files;

    #[test]
    fn reads_visible_publisher_links_and_rejects_changed_text() {
        let html = r#"<a href="tl_2025_23001_roads.zip">tl_2025_23001_roads.zip</a><a href="tl_2025_24001_roads.zip">tl_2025_24001_roads.zip</a>"#;
        let files = publisher_files(html, "23", "roads").unwrap();
        assert_eq!(
            files.into_iter().collect::<Vec<_>>(),
            ["tl_2025_23001_roads.zip"]
        );
        let changed = r#"<a href="tl_2025_23001_roads.zip">different.zip</a>"#;
        assert!(publisher_files(changed, "23", "roads").is_err());
    }
}
