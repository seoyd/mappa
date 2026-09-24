//! List published archive members before selecting canonical source classes.

use std::{error::Error, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: list_7z PUBLISHED.7z".into());
    }
    let archive = sevenz_rust2::Archive::open(Path::new(&args[1]))?;
    for entry in archive.files {
        if !entry.is_directory() {
            println!("{}\t{}", entry.size(), entry.name());
        }
    }
    Ok(())
}
