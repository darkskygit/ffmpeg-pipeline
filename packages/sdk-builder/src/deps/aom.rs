//! AOM build module.

#[cfg(not(target_os = "windows"))]
use cmake::Config;
use std::io::Result;
use std::path::{Path, PathBuf};

use crate::utils;

const AOM_VERSION: &str = "3.12.1";

pub struct AomBuilder<'a> {
    source_dir: &'a PathBuf,
    output_dir: &'a PathBuf,
    job_count: usize,
    verbose: bool,
}

impl<'a> AomBuilder<'a> {
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
        self.source_dir.join("aom")
    }

    fn build_dir(&self) -> PathBuf {
        self.output_dir.join("_build").join("aom")
    }

    fn build_aom_common(&self) -> Result<()> {
        utils::log_info("Compiling AOM (libaom)...", self.verbose);

        let aom_dir = self.source_checkout_dir();
        if !aom_dir.exists() {
            utils::clone_repository(
                "https://aomedia.googlesource.com/aom",
                &aom_dir,
                Some(&format!("v{AOM_VERSION}")),
                None,
                self.verbose,
            )?;
        }

        utils::prepare_cmake_build(self.output_dir, &self.build_dir(), self.job_count)?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn base_cmake_config(&self, aom_dir: &Path, aom_build_dir: &Path) -> Config {
        let mut config = Config::new(aom_dir);
        config
            .define("CMAKE_INSTALL_PREFIX", self.output_dir)
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("ENABLE_DOCS", "OFF")
            .define("ENABLE_EXAMPLES", "OFF")
            .define("ENABLE_TESTS", "OFF")
            .define("ENABLE_TESTDATA", "OFF")
            .define("ENABLE_TOOLS", "OFF")
            .define("ENABLE_NASM", "ON")
            .define("CONFIG_AV1_ENCODER", "0")
            .out_dir(aom_build_dir);
        config
    }

    #[cfg(target_os = "macos")]
    pub fn build(&self) -> Result<()> {
        self.build_aom_common()?;

        let aom_dir = self.source_checkout_dir();
        let aom_build_dir = self.build_dir();

        let mut config = self.base_cmake_config(&aom_dir, &aom_build_dir);
        config.build_target("install").build();

        utils::log_success("AOM build finished", self.verbose);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn build(&self) -> Result<()> {
        self.build_aom_common()?;

        let aom_dir = self.source_checkout_dir();
        let aom_build_dir = self.build_dir();

        let mut config = self.base_cmake_config(&aom_dir, &aom_build_dir);
        config.generator("Unix Makefiles");
        config.build_target("install").build();

        utils::log_success("AOM build finished", self.verbose);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn build(&self) -> Result<()> {
        self.build_aom_common()?;

        let aom_dir = self.source_checkout_dir();
        let aom_build_dir = self.build_dir().join("build");
        utils::prepare_cmake_build(self.output_dir, &aom_build_dir, self.job_count)?;

        utils::run_windows_cmake_install(
            &aom_dir,
            &aom_build_dir,
            self.output_dir,
            &[
                "-DBUILD_SHARED_LIBS=OFF".to_string(),
                "-DENABLE_DOCS=OFF".to_string(),
                "-DENABLE_EXAMPLES=OFF".to_string(),
                "-DENABLE_TESTS=OFF".to_string(),
                "-DENABLE_TESTDATA=OFF".to_string(),
                "-DENABLE_TOOLS=OFF".to_string(),
                "-DENABLE_NASM=on".to_string(),
                "-DCONFIG_AV1_ENCODER=0".to_string(),
                "-DCMAKE_C_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_CXX_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded$<$<CONFIG:Debug>:Debug>".to_string(),
            ],
            "Release",
            self.job_count,
            "AOM",
        )?;
        utils::log_success("AOM build finished", self.verbose);
        Ok(())
    }
}
