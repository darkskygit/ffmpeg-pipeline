use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use ffmpeg_sdk_builder::{is_valid_sdk, pipeline_sdk_builder};

struct Options {
    output_dir: PathBuf,
    work_dir: PathBuf,
    job_count: Option<usize>,
}

fn main() -> io::Result<()> {
    let Some(options) = parse_args(env::args_os().skip(1))? else {
        return Ok(());
    };
    if env::var_os("FFMPEG_DIR").is_some() {
        return Err(io::Error::other(
            "FFMPEG_DIR must be unset when producing a new SDK",
        ));
    }

    let output_dir = absolute_path(&options.output_dir)?;
    let work_dir = absolute_path(&options.work_dir)?;
    if output_dir.starts_with(&work_dir) || work_dir.starts_with(&output_dir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--output and --work-dir must not contain one another",
        ));
    }

    let source_dir = work_dir.join("sources");
    let mut builder = pipeline_sdk_builder(&source_dir, &work_dir).verbose(true);
    if let Some(job_count) = options.job_count {
        builder = builder.job_count(job_count);
    }
    builder.build().compile()?;

    let built_sdk = work_dir.join("ffmpeg_build");
    stage_sdk(&built_sdk, &output_dir)?;
    if !is_valid_sdk(&output_dir) {
        return Err(io::Error::other(format!(
            "generated SDK is incomplete: {}",
            output_dir.display()
        )));
    }
    println!("FFmpeg SDK written to {}", output_dir.display());
    Ok(())
}

fn parse_args(args: impl Iterator<Item = OsString>) -> io::Result<Option<Options>> {
    let mut output_dir = PathBuf::from("ffmpeg-sdk");
    let mut work_dir = PathBuf::from(".build/ffmpeg-sdk");
    let mut job_count = None;
    let mut args = args;

    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--output") => output_dir = PathBuf::from(required_value(&mut args, "--output")?),
            Some("--work-dir") => {
                work_dir = PathBuf::from(required_value(&mut args, "--work-dir")?)
            }
            Some("--jobs") => {
                let value = required_value(&mut args, "--jobs")?;
                let parsed = value.to_string_lossy().parse::<usize>().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--jobs must be an integer")
                })?;
                if parsed == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--jobs must be greater than zero",
                    ));
                }
                job_count = Some(parsed);
            }
            Some("-h" | "--help") => {
                print_help();
                return Ok(None);
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {}", argument.to_string_lossy()),
                ));
            }
        }
    }

    Ok(Some(Options {
        output_dir,
        work_dir,
        job_count,
    }))
}

fn required_value(
    args: &mut impl Iterator<Item = OsString>,
    argument: &str,
) -> io::Result<OsString> {
    args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{argument} requires a value"),
        )
    })
}

fn stage_sdk(source: &Path, destination: &Path) -> io::Result<()> {
    if destination.exists() {
        if !is_valid_sdk(destination) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "refusing to replace a directory that is not an FFmpeg SDK: {}",
                    destination.display()
                ),
            ));
        }
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination)?;
    for directory in ["include", "lib", "share"] {
        let source = source.join(directory);
        if source.exists() {
            copy_dir(&source, &destination.join(directory))?;
        }
    }
    make_pkg_config_relocatable(&destination.join("lib/pkgconfig"))?;
    Ok(())
}

fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

fn make_pkg_config_relocatable(directory: &Path) -> io::Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("pc") {
            continue;
        }
        let contents = fs::read_to_string(&path)?;
        let mut lines = contents.lines();
        let Some(first_line) = lines.next() else {
            continue;
        };
        if first_line.starts_with("prefix=") {
            let mut relocated = String::from("prefix=${pcfiledir}/../..\n");
            relocated.push_str(&lines.collect::<Vec<_>>().join("\n"));
            relocated.push('\n');
            fs::write(path, relocated)?;
        }
    }
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        "Build the static FFmpeg SDK used by ffmpeg-pipeline.\n\n\
         Usage: build-ffmpeg-sdk [--output DIR] [--work-dir DIR] [--jobs N]\n\n\
         Defaults:\n  --output ffmpeg-sdk\n  --work-dir .build/ffmpeg-sdk"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_build_options() {
        let options = parse_args(
            ["--output", "sdk", "--work-dir", "work", "--jobs", "4"]
                .into_iter()
                .map(OsString::from),
        )
        .unwrap()
        .unwrap();
        assert_eq!(options.output_dir, PathBuf::from("sdk"));
        assert_eq!(options.work_dir, PathBuf::from("work"));
        assert_eq!(options.job_count, Some(4));
    }

    #[test]
    fn stages_only_sdk_artifacts() {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let destination = temporary.path().join("destination");
        fs::create_dir_all(source.join("include/libavutil")).unwrap();
        fs::create_dir_all(source.join("lib")).unwrap();
        fs::create_dir_all(source.join("lib/pkgconfig")).unwrap();
        fs::create_dir_all(source.join("_build")).unwrap();
        fs::write(source.join("include/libavutil/avutil.h"), []).unwrap();
        for library in ["avutil", "avcodec", "avformat"] {
            fs::write(source.join("lib").join(format!("lib{library}.a")), []).unwrap();
        }
        fs::write(source.join("_build/intermediate"), []).unwrap();
        fs::write(
            source.join("lib/pkgconfig/libavutil.pc"),
            "prefix=/temporary/build\nlibdir=${prefix}/lib\n",
        )
        .unwrap();

        stage_sdk(&source, &destination).unwrap();

        assert!(is_valid_sdk(&destination));
        assert!(!destination.join("_build").exists());
        assert_eq!(
            fs::read_to_string(destination.join("lib/pkgconfig/libavutil.pc")).unwrap(),
            "prefix=${pcfiledir}/../..\nlibdir=${prefix}/lib\n"
        );
    }

    #[test]
    fn refuses_to_replace_unrelated_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let destination = temporary.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("unrelated"), []).unwrap();

        assert_eq!(
            stage_sdk(&source, &destination).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(destination.join("unrelated").exists());
    }
}
