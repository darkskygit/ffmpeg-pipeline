//! Zlib build module (Windows only).

use std::fs;
use std::io::{self, Result};
use std::path::PathBuf;

use crate::utils;

pub struct ZlibBuilder<'a> {
    source_dir: &'a PathBuf,
    output_dir: &'a PathBuf,
    job_count: usize,
    verbose: bool,
}

impl<'a> ZlibBuilder<'a> {
    pub fn new(
        source_dir: &'a PathBuf,
        output_dir: &'a PathBuf,
        job_count: usize,
        verbose: bool,
    ) -> Self {
        Self {
            source_dir,
            output_dir,
            job_count,
            verbose,
        }
    }

    fn source_checkout_dir(&self) -> PathBuf {
        self.source_dir.join("zlib-1.3.2")
    }

    fn build_dir(&self) -> PathBuf {
        self.output_dir.join("_build").join("zlib")
    }

    /// Build zlib on Windows and install it into the shared FFmpeg output directory.
    pub fn build(&self) -> Result<()> {
        utils::log_info("Compiling zlib...", self.verbose);

        let zlib_tarball = self.source_dir.join("zlib-1.3.2.tar.gz");
        if !zlib_tarball.exists() {
            utils::download_file(
                "https://zlib.net/zlib-1.3.2.tar.gz",
                &zlib_tarball,
                self.verbose,
            )?;

            const ZLIB_SHA256: &str =
                "bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16";

            if !utils::verify_sha256(&zlib_tarball, ZLIB_SHA256, self.verbose)? {
                fs::remove_file(&zlib_tarball)?;
                return Err(io::Error::other(
                    "Zlib source archive integrity check failed",
                ));
            }
        }

        let zlib_src_dir = self.source_checkout_dir();
        if !zlib_src_dir.exists() {
            utils::extract_tar_gz(&zlib_tarball, self.source_dir, self.verbose)?;
        }

        let cmake_build_dir = self.build_dir().join("build");
        utils::prepare_cmake_build(self.output_dir, &cmake_build_dir, self.job_count)?;

        utils::log_info(
            &format!(
                "Building zlib with CMake into {} using {} jobs...",
                self.output_dir.display(),
                self.job_count
            ),
            self.verbose,
        );

        utils::run_windows_cmake_install(
            &zlib_src_dir,
            &cmake_build_dir,
            self.output_dir,
            &[
                "-DCMAKE_BUILD_TYPE=Release".to_string(),
                "-DBUILD_SHARED_LIBS=OFF".to_string(),
                "-DZLIB_BUILD_EXAMPLES=OFF".to_string(),
                "-DCMAKE_C_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_CXX_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded$<$<CONFIG:Debug>:Debug>".to_string(),
            ],
            "RelWithDebInfo",
            self.job_count,
            "zlib",
        )?;

        let installed_lib_dir = self.output_dir.join("lib");
        let installed_zlib_lib = installed_lib_dir.join("zlib.lib");
        let installed_zlibstatic_lib = installed_lib_dir.join("zlibstatic.lib");
        if !installed_zlib_lib.exists() && installed_zlibstatic_lib.exists() {
            fs::copy(&installed_zlibstatic_lib, &installed_zlib_lib)?;
        }

        utils::log_success(
            &format!("Zlib build finished: {}", self.output_dir.display()),
            self.verbose,
        );
        Ok(())
    }
}
