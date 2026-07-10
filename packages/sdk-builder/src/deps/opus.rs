//! Opus build module.

use std::fs;
use std::io::{self, Result};
use std::path::PathBuf;

use crate::utils;

pub struct OpusBuilder<'a> {
    source_dir: &'a PathBuf,
    output_dir: &'a PathBuf,
    job_count: usize,
    verbose: bool,
}

impl<'a> OpusBuilder<'a> {
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
        self.source_dir.join("opus-1.5.2")
    }

    fn build_dir(&self) -> PathBuf {
        self.output_dir.join("_build").join("opus")
    }

    fn build_opus_common(&self) -> Result<()> {
        utils::log_info("Compiling Opus...", self.verbose);

        let opus_tarball = self.source_dir.join("opus-1.5.2.tar.gz");
        if !opus_tarball.exists() {
            utils::download_file(
                "https://downloads.xiph.org/releases/opus/opus-1.5.2.tar.gz",
                &opus_tarball,
                self.verbose,
            )?;

            const OPUS_SHA256: &str =
                "65c1d2f78b9f2fb20082c38cbe47c951ad5839345876e46941612ee87f9a7ce1";

            if !utils::verify_sha256(&opus_tarball, OPUS_SHA256, self.verbose)? {
                fs::remove_file(&opus_tarball)?;
                return Err(io::Error::other(
                    "Opus source archive integrity check failed",
                ));
            }
        }

        let opus_src_dir = self.source_checkout_dir();
        if !opus_src_dir.exists() {
            utils::extract_tar_gz(&opus_tarball, self.source_dir, self.verbose)?;
        }

        utils::prepare_cmake_build(self.output_dir, &self.build_dir(), self.job_count)?;
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn build(&self) -> Result<()> {
        self.build_opus_common()?;

        let opus_src_dir = self.source_checkout_dir();
        let opus_build_dir = self.build_dir();

        utils::log_info("Building Opus with CMake...", self.verbose);

        utils::run_cmake_install(
            &opus_src_dir,
            &opus_build_dir,
            self.output_dir,
            &[
                "-DBUILD_SHARED_LIBS=OFF".to_string(),
                "-DOPUS_BUILD_SHARED_LIBRARY=OFF".to_string(),
                "-DOPUS_BUILD_TESTING=OFF".to_string(),
                "-DOPUS_BUILD_PROGRAMS=OFF".to_string(),
                "-DOPUS_OSCE=ON".to_string(),
                "-DOPUS_STATIC_RUNTIME=ON".to_string(),
            ],
            self.job_count,
            "Opus",
        )?;

        utils::log_success("Opus build finished", self.verbose);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn build(&self) -> Result<()> {
        self.build_opus_common()?;

        let opus_src_dir = self.source_checkout_dir();
        let opus_build_dir = self.build_dir().join("build");
        utils::prepare_cmake_build(self.output_dir, &opus_build_dir, self.job_count)?;

        utils::log_info("Building Opus with CMake...", self.verbose);

        utils::run_windows_cmake_install(
            &opus_src_dir,
            &opus_build_dir,
            self.output_dir,
            &[
                "-DCMAKE_BUILD_TYPE=Release".to_string(),
                "-DBUILD_SHARED_LIBS=OFF".to_string(),
                "-DOPUS_BUILD_SHARED_LIBRARY=OFF".to_string(),
                "-DOPUS_BUILD_TESTING=OFF".to_string(),
                "-DOPUS_BUILD_PROGRAMS=OFF".to_string(),
                "-DOPUS_OSCE=ON".to_string(),
                "-DOPUS_STATIC_RUNTIME=ON".to_string(),
                "-DCMAKE_C_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_CXX_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded$<$<CONFIG:Debug>:Debug>".to_string(),
            ],
            "Release",
            self.job_count,
            "Opus",
        )?;

        utils::log_success("Opus build finished", self.verbose);
        Ok(())
    }
}
