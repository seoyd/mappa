use std::{env, error::Error, fs, path::PathBuf, process::Command};

fn variable(name: &str) -> Result<String, Box<dyn Error>> {
    Ok(env::var(name).map_err(|_| format!("missing Xcode build setting {name}"))?)
}

fn run(mut command: Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("build command exited with {status}").into())
    }
}

fn build() -> Result<(), Box<dyn Error>> {
    let sdk = variable("SDK_NAME")?;
    let target = if sdk.starts_with("iphonesimulator") {
        "aarch64-apple-ios-sim"
    } else if sdk.starts_with("iphoneos") {
        "aarch64-apple-ios"
    } else {
        return Err(format!("unsupported SDK_NAME: {sdk}").into());
    };
    if variable("ARCHS")?.trim() != "arm64" {
        return Err("only arm64 is configured".into());
    }
    let release = variable("CONFIGURATION")? != "Debug";
    let target_dir = PathBuf::from(variable("DERIVED_FILE_DIR")?).join("cargo");
    let mut cargo = Command::new("cargo");
    cargo.args([
        "build",
        "--package",
        "mappa-ios",
        "--bin",
        "mappa-ios",
        "--target",
        target,
    ]);
    if release {
        cargo.arg("--release");
    }
    cargo
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "1");
    run(cargo)?;
    let built = target_dir
        .join(target)
        .join(if release { "release" } else { "debug" })
        .join("mappa-ios");
    let destination =
        PathBuf::from(variable("TARGET_BUILD_DIR")?).join(variable("EXECUTABLE_PATH")?);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(built, &destination)?;
    if let (Ok(folder), Ok(name)) = (
        env::var("DWARF_DSYM_FOLDER_PATH"),
        env::var("DWARF_DSYM_FILE_NAME"),
    ) {
        fs::create_dir_all(&folder)?;
        let mut dsymutil = Command::new("dsymutil");
        dsymutil
            .arg(&destination)
            .arg("-o")
            .arg(PathBuf::from(folder).join(name));
        run(dsymutil)?;
    }
    if target == "aarch64-apple-ios"
        && env::var("CODE_SIGNING_ALLOWED").as_deref() != Ok("NO")
        && let Ok(identity) = env::var("EXPANDED_CODE_SIGN_IDENTITY")
        && !identity.is_empty()
    {
        let mut codesign = Command::new("codesign");
        codesign.args(["--force", "--sign", &identity]);
        if let (Ok(temp), Ok(product)) = (env::var("TARGET_TEMP_DIR"), env::var("PRODUCT_NAME")) {
            let entitlements = PathBuf::from(temp).join(format!("{product}.app.xcent"));
            if entitlements
                .metadata()
                .is_ok_and(|metadata| metadata.len() > 0)
            {
                codesign.arg("--entitlements").arg(entitlements);
            }
        }
        codesign.arg(&destination);
        run(codesign)?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = build() {
        eprintln!("iOS build failed: {error}");
        std::process::exit(1);
    }
}
