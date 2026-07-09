use std::fs;
use std::io::{self, Result};
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::utils::{download_file, extract_tar_gz, verify_sha256};

const SUPPORTED_TARGETS: &[&str] = &[
    "x86_64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

pub fn ensure_prebuilt_sdk(
    output_dir: &Path,
    target: &str,
    tag: &str,
    base_url: &str,
    verbose: bool,
) -> Result<PathBuf> {
    if !SUPPORTED_TARGETS.contains(&target) {
        return Err(io::Error::other(format!(
            "no prebuilt FFmpeg SDK is published for target {target}; set FFMPEG_DIR or enable the build-from-source feature"
        )));
    }

    let sdk_dir = output_dir.join(format!("ffmpeg-sdk-{tag}-{target}"));
    if is_valid_sdk(&sdk_dir) {
        return Ok(sdk_dir);
    }

    fs::create_dir_all(output_dir)?;
    let temporary = TempDir::new_in(output_dir)?;
    let asset = format!("ffmpeg-sdk-{target}.tar.gz");
    let archive = temporary.path().join(&asset);
    let checksum_file = temporary.path().join(format!("{asset}.sha256"));
    let release_url = format!("{}/{tag}", base_url.trim_end_matches('/'));

    download_file(&format!("{release_url}/{asset}"), &archive, verbose)?;
    download_file(
        &format!("{release_url}/{asset}.sha256"),
        &checksum_file,
        verbose,
    )?;

    let checksum_contents = fs::read_to_string(&checksum_file)?;
    let checksum = parse_checksum(&checksum_contents)?;
    if !verify_sha256(&archive, checksum, verbose)? {
        return Err(io::Error::other(format!(
            "checksum verification failed for {asset}"
        )));
    }

    let extracted = temporary.path().join("extracted");
    fs::create_dir_all(&extracted)?;
    extract_tar_gz(&archive, &extracted, verbose)?;
    if !is_valid_sdk(&extracted) {
        return Err(io::Error::other(format!(
            "release asset {asset} does not contain a valid FFmpeg SDK"
        )));
    }

    if sdk_dir.exists() {
        fs::remove_dir_all(&sdk_dir)?;
    }
    fs::rename(extracted, &sdk_dir)?;
    Ok(sdk_dir)
}

pub fn is_valid_sdk(path: &Path) -> bool {
    path.join("include/libavutil/avutil.h").is_file()
        && library_exists(&path.join("lib"), "avutil")
        && library_exists(&path.join("lib"), "avcodec")
        && library_exists(&path.join("lib"), "avformat")
}

fn library_exists(directory: &Path, name: &str) -> bool {
    [
        format!("lib{name}.a"),
        format!("{name}.lib"),
        format!("lib{name}.lib"),
    ]
    .iter()
    .any(|file| directory.join(file).is_file())
}

fn parse_checksum(contents: &str) -> Result<&str> {
    let checksum = contents
        .split_whitespace()
        .next()
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| io::Error::other("release checksum file is invalid"))?;
    Ok(checksum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_sha256_file() {
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            parse_checksum(&format!("{hash}  sdk.tar.gz\n")).unwrap(),
            hash
        );
    }

    #[test]
    fn rejects_invalid_checksum() {
        assert!(parse_checksum("not-a-checksum sdk.tar.gz").is_err());
    }
}
