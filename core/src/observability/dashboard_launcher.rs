use std::env;
/// Dashboard launcher for spawning stats dashboard in separate terminal window
/// Platform-aware: uses appropriate terminal for macOS/Linux
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy)]
pub enum Platform {
    MacOS,
    Linux,
    Other,
}

impl Platform {
    pub fn detect() -> Self {
        let os = env::consts::OS;
        match os {
            "macos" => Platform::MacOS,
            "linux" => Platform::Linux,
            _ => Platform::Other,
        }
    }
}

/// Configuration for dashboard launcher
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub server_url: String,
    pub refresh_interval_ms: u64,
    pub history_size: usize,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:9999".to_string(),
            refresh_interval_ms: 1000,
            history_size: 300,
        }
    }
}

/// Launches the stats dashboard in a separate terminal window
pub fn launch_dashboard(config: DashboardConfig) -> crate::Result<std::process::Child> {
    let platform = Platform::detect();

    match platform {
        Platform::MacOS => launch_dashboard_macos(config),
        Platform::Linux => launch_dashboard_linux(config),
        Platform::Other => Err(crate::Error::ConfigError(
            "Unsupported platform for dashboard launcher".to_string(),
        )),
    }
}

/// Launch dashboard on macOS using Terminal.app
fn launch_dashboard_macos(config: DashboardConfig) -> crate::Result<std::process::Child> {
    let script = format!(
        r#"
open -a Terminal <<'EOF'
cd {}
cargo run --bin pyreverseetl-dashboard -- \
    --server-url {} \
    --refresh-interval {} \
    --history-size {}
EOF
"#,
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .display(),
        config.server_url,
        config.refresh_interval_ms,
        config.history_size
    );

    let child = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            crate::Error::ConfigError(format!("Failed to launch dashboard on macOS: {}", e))
        })?;

    Ok(child)
}

/// Launch dashboard on Linux using terminator (with fallback to xterm)
fn launch_dashboard_linux(config: DashboardConfig) -> crate::Result<std::process::Child> {
    let cmd = format!(
        "cargo run --bin pyreverseetl-dashboard -- --server-url {} --refresh-interval {} --history-size {}",
        config.server_url,
        config.refresh_interval_ms,
        config.history_size
    );

    // Try terminator first, fall back to xterm
    let child = if is_command_available("terminator") {
        Command::new("terminator")
            .arg("-e")
            .arg(&cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                crate::Error::ConfigError(format!(
                    "Failed to launch dashboard with terminator: {}",
                    e
                ))
            })?
    } else if is_command_available("xterm") {
        Command::new("xterm")
            .arg("-hold")
            .arg("-e")
            .arg(&cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                crate::Error::ConfigError(format!("Failed to launch dashboard with xterm: {}", e))
            })?
    } else if is_command_available("gnome-terminal") {
        Command::new("gnome-terminal")
            .arg("--")
            .arg("bash")
            .arg("-c")
            .arg(&cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                crate::Error::ConfigError(format!(
                    "Failed to launch dashboard with gnome-terminal: {}",
                    e
                ))
            })?
    } else {
        return Err(crate::Error::ConfigError(
            "No terminal emulator found (terminator, xterm, or gnome-terminal required)"
                .to_string(),
        ));
    };

    Ok(child)
}

/// Check if a command is available in PATH
fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::detect();
        // Just ensure we can detect a platform
        match platform {
            Platform::MacOS | Platform::Linux | Platform::Other => {
                // All platforms are valid
            }
        }
    }

    #[test]
    fn test_dashboard_config_default() {
        let config = DashboardConfig::default();
        assert_eq!(config.server_url, "http://localhost:9999");
        assert_eq!(config.refresh_interval_ms, 1000);
        assert_eq!(config.history_size, 300);
    }

    #[test]
    fn test_dashboard_config_custom() {
        let config = DashboardConfig {
            server_url: "http://127.0.0.1:8080".to_string(),
            refresh_interval_ms: 500,
            history_size: 100,
        };
        assert_eq!(config.server_url, "http://127.0.0.1:8080");
        assert_eq!(config.refresh_interval_ms, 500);
        assert_eq!(config.history_size, 100);
    }
}
