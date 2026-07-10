use std::env;

/// 检测运行平台
pub fn detect_platform() -> String {
    classify_platform(env::var("TARGET").ok().as_deref(), env::consts::OS).to_string()
}

fn classify_platform(target: Option<&str>, host_os: &str) -> &'static str {
    let target = target.unwrap_or(host_os);
    if target.contains("darwin") {
        "macos"
    } else if target.contains("windows") {
        "windows"
    } else if target.contains("linux") {
        "linux"
    } else if target == "macos" {
        "macos"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::classify_platform;

    #[test]
    fn detects_cargo_target_triples() {
        assert_eq!(
            classify_platform(Some("aarch64-apple-darwin"), "unknown"),
            "macos"
        );
        assert_eq!(
            classify_platform(Some("x86_64-pc-windows-msvc"), "unknown"),
            "windows"
        );
    }

    #[test]
    fn falls_back_to_binary_host_os() {
        assert_eq!(classify_platform(None, "macos"), "macos");
        assert_eq!(classify_platform(None, "windows"), "windows");
        assert_eq!(classify_platform(None, "linux"), "linux");
    }
}
