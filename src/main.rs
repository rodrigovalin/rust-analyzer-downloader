use anyhow::Result;
use reqwest;
use reqwest::header::{ACCEPT, CONTENT_TYPE, USER_AGENT};
use serde_derive::{Deserialize, Serialize};

use std::fs;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::{env, io, str};

#[derive(Serialize, Deserialize, Debug, Default)]
struct ReleaseAsset {
    url: String,
    name: String,
    browser_download_url: String,
    content_type: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Release {
    url: String,
    name: String,
    assets: Vec<ReleaseAsset>,
}

fn get_client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder().build()?)
}

fn get_os_name<'a>() -> &'a str {
    // In rust-analyzer releases, mac assets are suffixed with "-mac" but
    // the Rust's `env::consts::OS` would be "macos" instead.
    if env::consts::OS == "macos" {
        "mac"
    } else {
        env::consts::OS
    }
}

fn get_asset_for_os(assets: Vec<ReleaseAsset>) -> Option<ReleaseAsset> {
    let os = format!("-{}", get_os_name());
    for asset in assets {
        if asset.name.ends_with(&os) {
            return Some(asset);
        }
    }

    None
}

fn download_url_to_location(url: String, content_type: String, location: &str) -> Result<()> {
    let mut resp = get_client()?
        .get(&url)
        .header(CONTENT_TYPE, content_type)
        .send()?;

    println!("Creating {}", location);
    let mut out = File::create(location)?;
    io::copy(&mut resp, &mut out)?;

    Ok(())
}

#[allow(dead_code)]
fn get_available_assets_os(assets: Vec<ReleaseAsset>) -> Vec<String> {
    let oses: Vec<String> = Vec::new();
    for _asset in assets {
        // get suffix split after . and return
    }

    oses
}

fn download_asset(asset: ReleaseAsset, location: &str) -> Result<()> {
    download_url_to_location(asset.browser_download_url, asset.content_type, location)
}

fn rust_analyzer_version() -> Result<String> {
    let output = Command::new("rust-analyzer")
        .arg("--version")
        .output()?
        .stdout;

    let sss = String::from(str::from_utf8(&output)?);
    let sum: Vec<&str> = sss.split(' ').collect();

    let version = String::from(
        sum.get(1)
            .expect("some error")
            .strip_suffix('\n')
            .expect("could not convert"),
    );

    Ok(version)
}

fn set_file_exec(filename: &str) -> Result<(), std::io::Error> {
    let mut perms = fs::metadata(filename)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(filename, perms)
}

fn get_rust_analyzer_latest_release() -> Result<Release> {
    let resp = get_client()?
        .get("https://api.github.com/repos/rust-analyzer/rust-analyzer/releases/latest")
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, "rust-analyzer-downloader")
        .send()?;

    let release: Release = resp.json()?;

    Ok(release)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rust_analyzer_base_path = dirs::home_dir().unwrap();
    rust_analyzer_base_path.push("bin/rust-analyzer");

    let rust_analyzer_base_path = rust_analyzer_base_path.to_str().unwrap();

    // First we move the existing rust_binary if we had some
    if let Ok(version) = rust_analyzer_version() {
        let new_name = format!("{}-{}", rust_analyzer_base_path, version);
        fs::rename(&rust_analyzer_base_path, new_name)?;
    }

    let release = get_rust_analyzer_latest_release()?;
    let asset = get_asset_for_os(release.assets).expect("No assets for this OS");

    println!("Url to download {}", asset.browser_download_url);
    download_asset(asset, &rust_analyzer_base_path)?;
    set_file_exec(&rust_analyzer_base_path)?;

    Ok(())
}
