use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Result, Write};
use std::path::Path;
#[cfg(target_os = "windows")]
use std::process::Command;
use std::process::Output;
use std::time::SystemTime;
use tar::Archive;

pub const CACHE_FILE: &str = ".ffmpeg_build_cache";

pub fn log_info(message: &str, verbose: bool) {
    if verbose {
        println!("INFO {}", message);
    }
}

pub fn log_success(message: &str, verbose: bool) {
    if verbose {
        println!("OK {}", message);
    }
}

pub fn log_error(message: &str) {
    eprintln!("ERR {}", message);
}

pub fn mkdir(dir_name: &Path) -> Result<()> {
    if !dir_name.exists() {
        log_info(&format!("Creating directory: {}", dir_name.display()), true);
        fs::create_dir_all(dir_name)?;
    }
    Ok(())
}

/// Prepare shared output/build directories and propagate the desired parallelism to CMake.
pub fn prepare_cmake_build(output_dir: &Path, build_dir: &Path, job_count: usize) -> Result<()> {
    mkdir(output_dir)?;
    mkdir(build_dir)?;
    env::set_var("CMAKE_BUILD_PARALLEL_LEVEL", job_count.to_string());
    Ok(())
}

pub fn handle_command_output(output: Result<Output>, step: &str) -> Result<()> {
    match output {
        Ok(output) => {
            if !output.status.success() {
                io::stderr().write_all(&output.stderr)?;
                return Err(io::Error::other(format!(
                    "{} command failed with exit code {:?}",
                    step,
                    output.status.code()
                )));
            }
            Ok(())
        }
        Err(e) => {
            log_error(&format!("{} command failed to start: {}", step, e));
            Err(e)
        }
    }
}

#[cfg(target_os = "windows")]
pub fn run_windows_cmake_install(
    source_dir: &Path,
    build_dir: &Path,
    output_dir: &Path,
    definitions: &[String],
    config_name: &str,
    job_count: usize,
    step_name: &str,
) -> Result<()> {
    let mut configure_args = vec![
        "-S".to_string(),
        source_dir.to_string_lossy().into_owned(),
        "-B".to_string(),
        build_dir.to_string_lossy().into_owned(),
        "-G".to_string(),
        "Visual Studio 17 2022".to_string(),
        "-A".to_string(),
        "x64".to_string(),
        "-T".to_string(),
        "host=x64".to_string(),
        format!("-DCMAKE_INSTALL_PREFIX={}", output_dir.display()),
    ];
    configure_args.extend(definitions.iter().cloned());

    let configure_output = Command::new("cmake").args(&configure_args).output()?;
    handle_command_output(Ok(configure_output), &format!("Configure {}", step_name))?;

    let build_output = Command::new("cmake")
        .args([
            "--build",
            &build_dir.to_string_lossy(),
            "--config",
            config_name,
            "--parallel",
            &job_count.to_string(),
        ])
        .output()?;
    handle_command_output(Ok(build_output), &format!("Build {}", step_name))?;

    let install_output = Command::new("cmake")
        .args([
            "--install",
            &build_dir.to_string_lossy(),
            "--config",
            config_name,
        ])
        .output()?;
    handle_command_output(Ok(install_output), &format!("Install {}", step_name))?;

    Ok(())
}

#[cfg(feature = "build-from-source")]
pub fn clone_repository(
    url: &str,
    destination: &Path,
    branch: Option<&str>,
    commit_id: Option<&str>,
    verbose: bool,
) -> Result<()> {
    use gix::{
        hash::ObjectId, interrupt::IS_INTERRUPTED, prepare_clone, progress::Discard,
        refs::transaction::PreviousValue,
    };

    let url = gix::url::parse(gix::bstr::BStr::new(url.as_bytes())).unwrap();

    let (mut repo, _) = prepare_clone(url, destination)
        .unwrap()
        .with_ref_name(branch)
        .unwrap()
        .fetch_then_checkout(Discard, &IS_INTERRUPTED)
        .unwrap();

    if let Some(id) = commit_id {
        let oid = ObjectId::from_hex(id.as_bytes()).unwrap();

        repo.repo()
            .reference("HEAD", oid, PreviousValue::Any, "checkout specific commit")
            .unwrap();
    };

    repo.main_worktree(Discard, &IS_INTERRUPTED).unwrap();

    if verbose {
        println!(
            "Repository cloned to {:?}, {}",
            destination,
            if let Some(c) = commit_id {
                format!("detached at {}", c)
            } else if let Some(b) = branch {
                format!("on branch {}", b)
            } else {
                "no checkout".into()
            }
        );
    }
    Ok(())
}

pub fn download_file(url: &str, destination: &Path, verbose: bool) -> Result<()> {
    log_info(
        &format!("Downloading file: {} -> {}", url, destination.display()),
        verbose,
    );

    let client = reqwest::blocking::Client::new();
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| io::Error::other(format!("Download failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(io::Error::other(format!(
            "Download failed, HTTP status: {}",
            response.status()
        )));
    }

    let mut file = fs::File::create(destination)?;
    io::copy(&mut response, &mut file)?;

    log_success(
        &format!("Download completed: {}", destination.display()),
        verbose,
    );
    Ok(())
}

pub fn extract_tar_gz(source_path: &Path, extract_dir: &Path, verbose: bool) -> Result<()> {
    log_info(
        &format!(
            "Extracting tar.gz with Rust: {} -> {}",
            source_path.display(),
            extract_dir.display()
        ),
        verbose,
    );

    let tar_gz = fs::File::open(source_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive.unpack(extract_dir)?;

    log_success(
        &format!("Extraction completed: {}", extract_dir.display()),
        verbose,
    );
    Ok(())
}

pub fn verify_sha256(file_path: &Path, expected_hash: &str, verbose: bool) -> Result<bool> {
    log_info(
        &format!("Verifying file hash: {}", file_path.display()),
        verbose,
    );

    let mut file = fs::File::open(file_path)?;
    let mut hasher = Sha256::new();

    io::copy(&mut file, &mut hasher)?;

    let hash = hasher.finalize();
    let hash_string = format!("{:x}", hash);

    let verified = hash_string == expected_hash;

    if verified {
        log_success("Hash verification passed", verbose);
    } else {
        log_error(&format!(
            "Hash verification failed: expected {}, got {}",
            expected_hash, hash_string
        ));
    }

    Ok(verified)
}

pub fn check_cache(build_path: &Path, sources_path: &Path) -> bool {
    let cache_file = build_path.join(CACHE_FILE);

    if !cache_file.exists() {
        return false;
    }

    if let Ok(cache_content) = fs::read_to_string(&cache_file) {
        if let Ok(cache_time) = cache_content.trim().parse::<u64>() {
            let source_files = vec![
                sources_path.join("aom"),
                sources_path.join("opus-1.5.2"),
                sources_path.join("ffmpeg"),
            ];

            for file in source_files {
                if file.exists() {
                    if let Ok(metadata) = fs::metadata(&file) {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(modified_secs) = modified
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                            {
                                if modified_secs > cache_time {
                                    return false;
                                }
                            }
                        }
                    }
                } else {
                    return false;
                }
            }

            return true;
        }
    }

    false
}

pub fn update_cache(build_path: &Path) -> Result<()> {
    let cache_file = build_path.join(CACHE_FILE);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();

    fs::write(cache_file, now.to_string())?;
    Ok(())
}
