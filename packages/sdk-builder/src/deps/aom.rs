//! AOM build module.

#[cfg(not(target_os = "windows"))]
use cmake::Config;
#[cfg(target_os = "windows")]
use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};

use crate::utils;

const AOM_COMMIT: &str = "412efe2dacf00de33fa32633dff4cf53d3d05b4f";

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

    fn checkout_commit(&self) -> Option<&'static str> {
        if cfg!(target_os = "macos") {
            None
        } else {
            Some(AOM_COMMIT)
        }
    }

    fn build_aom_common(&self) -> Result<()> {
        utils::log_info("Compiling AOM (libaom)...", self.verbose);

        let aom_dir = self.source_checkout_dir();
        if !aom_dir.exists() {
            utils::clone_repository(
                "https://aomedia.googlesource.com/aom",
                &aom_dir,
                None,
                self.checkout_commit(),
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
            .out_dir(aom_build_dir);
        config
    }

    #[cfg(target_os = "windows")]
    fn install_windows_artifacts(&self, aom_dir: &Path, aom_build_dir: &Path) -> Result<()> {
        let include_dir = self.output_dir.join("include");
        let lib_dir = self.output_dir.join("lib");
        let pkgconfig_dir = lib_dir.join("pkgconfig");
        utils::mkdir(&include_dir)?;
        utils::mkdir(&lib_dir)?;
        utils::mkdir(&pkgconfig_dir)?;

        for header_dir in [
            "aom",
            "aom_dsp",
            "aom_mem",
            "aom_ports",
            "aom_scale",
            "aom_util",
        ] {
            let source = aom_dir.join(header_dir);
            if source.exists() {
                copy_dir_all(&source, &include_dir.join(header_dir))?;
            }
        }
        copy_dir_all(&aom_build_dir.join("config"), &include_dir.join("config"))?;

        fs::copy(
            aom_build_dir.join("RelWithDebInfo").join("aom.lib"),
            lib_dir.join("aom.lib"),
        )?;

        fs::write(
            pkgconfig_dir.join("aom.pc"),
            format!(
                "prefix={0}\nexec_prefix=${{prefix}}\nlibdir=${{exec_prefix}}/lib\nincludedir=${{prefix}}/include\n\nName: aom\nDescription: Alliance for Open Media AV1 codec library\nVersion: 3.12.0\nLibs: -L${{libdir}} -laom\nCflags: -I${{includedir}}\n",
                self.output_dir.display()
            ),
        )?;

        Ok(())
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

        // The Windows-pinned AOM commit does not generate INSTALL.vcxproj even with the
        // Visual Studio generator, so we cannot rely on cmake --build --target install here.
        // Configure/build can still be driven by CMake, then we stage headers/libs manually.
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
                "-DCMAKE_C_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_CXX_FLAGS_RELEASE=/MT /GL".to_string(),
                "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded$<$<CONFIG:Debug>:Debug>".to_string(),
            ],
            "RelWithDebInfo",
            self.job_count,
            "AOM",
        )?;
        self.install_windows_artifacts(&aom_dir, &aom_build_dir)?;

        utils::log_success("AOM build finished", self.verbose);
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let source = entry.path();
        let destination = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&source, &destination)?;
        } else {
            fs::copy(source, destination)?;
        }
    }
    Ok(())
}
