use std::env;

/// 检测运行平台
pub fn detect_platform() -> String {
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    if target.contains("darwin") {
        "macos".to_string()
    } else if target.contains("windows") {
        "windows".to_string()
    } else if target.contains("linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    }
}
