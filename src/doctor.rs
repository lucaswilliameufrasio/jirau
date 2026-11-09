use colored::*;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorItem {
    pub category: String,
    pub title: String,
    pub status: CheckStatus,
    pub detail: String,
    pub fix_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub items: Vec<DoctorItem>,
    pub ok_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
}

pub struct DoctorEngine;

/// Categoria do `--json` (estável) → rótulo exibido no relatório.
const CATEGORY_TOOLING: &str = "Tooling";

fn category_label(category: &str) -> String {
    match category {
        CATEGORY_TOOLING => t!("doctor.category_tooling").into_owned(),
        other => other.to_string(),
    }
}

impl DoctorEngine {
    pub fn run_all() -> DoctorReport {
        let mut items = Vec::new();

        // 1. Rust Ecosystem
        items.extend(Self::check_rust());

        // 2. Android Ecosystem
        items.extend(Self::check_android());

        // 3. iOS Ecosystem
        items.extend(Self::check_ios());

        // 4. Desktop Ecosystem (Rune UI)
        items.extend(Self::check_desktop());

        // 5. Build Acceleration
        items.extend(Self::check_tooling());

        let ok_count = items.iter().filter(|i| i.status == CheckStatus::Ok).count();
        let warning_count = items
            .iter()
            .filter(|i| i.status == CheckStatus::Warning)
            .count();
        let error_count = items
            .iter()
            .filter(|i| i.status == CheckStatus::Error)
            .count();

        DoctorReport {
            items,
            ok_count,
            warning_count,
            error_count,
        }
    }

    fn check_rust() -> Vec<DoctorItem> {
        let mut items = Vec::new();

        // rustc
        match Command::new("rustc").arg("--version").output() {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                items.push(DoctorItem {
                    category: "Rust".to_string(),
                    title: t!("doctor.rustc_title").into_owned(),
                    status: CheckStatus::Ok,
                    detail: v,
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "Rust".to_string(),
                    title: t!("doctor.rustc_title").into_owned(),
                    status: CheckStatus::Error,
                    detail: t!("doctor.rustc_missing").into_owned(),
                    fix_hint: Some(
                        "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
                            .to_string(),
                    ),
                });
            }
        }

        // cargo
        match Command::new("cargo").arg("--version").output() {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                items.push(DoctorItem {
                    category: "Rust".to_string(),
                    title: t!("doctor.cargo_title").into_owned(),
                    status: CheckStatus::Ok,
                    detail: v,
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "Rust".to_string(),
                    title: t!("doctor.cargo_title").into_owned(),
                    status: CheckStatus::Error,
                    detail: t!("doctor.cargo_missing").into_owned(),
                    fix_hint: Some(t!("doctor.cargo_hint").into_owned()),
                });
            }
        }

        // rustup installed targets
        let installed_targets = match Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
        {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };

        // Android targets
        let required_android_targets = ["aarch64-linux-android", "x86_64-linux-android"];
        let missing_android: Vec<_> = required_android_targets
            .iter()
            .filter(|t| !installed_targets.iter().any(|it| it == *t))
            .copied()
            .collect();

        if missing_android.is_empty() {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: t!("doctor.android_targets_title").into_owned(),
                status: CheckStatus::Ok,
                detail: t!(
                    "doctor.targets_installed",
                    targets = required_android_targets.join(", ")
                )
                .into_owned(),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: t!("doctor.android_targets_title").into_owned(),
                status: CheckStatus::Error,
                detail: t!(
                    "doctor.targets_missing",
                    targets = missing_android.join(", ")
                )
                .into_owned(),
                fix_hint: Some(format!("rustup target add {}", missing_android.join(" "))),
            });
        }

        // iOS targets
        let required_ios_targets = ["aarch64-apple-ios", "aarch64-apple-ios-sim"];
        let missing_ios: Vec<_> = required_ios_targets
            .iter()
            .filter(|t| !installed_targets.iter().any(|it| it == *t))
            .copied()
            .collect();

        if missing_ios.is_empty() {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: t!("doctor.ios_targets_title").into_owned(),
                status: CheckStatus::Ok,
                detail: t!(
                    "doctor.targets_installed",
                    targets = required_ios_targets.join(", ")
                )
                .into_owned(),
                fix_hint: None,
            });
        } else {
            let is_macos = cfg!(target_os = "macos");
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: t!("doctor.ios_targets_title").into_owned(),
                status: if is_macos {
                    CheckStatus::Error
                } else {
                    CheckStatus::Warning
                },
                detail: t!("doctor.targets_missing", targets = missing_ios.join(", ")).into_owned(),
                fix_hint: Some(format!("rustup target add {}", missing_ios.join(" "))),
            });
        }

        // cargo-ndk
        if which::which("cargo-ndk").is_ok() {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: t!("doctor.cargo_ndk_title").into_owned(),
                status: CheckStatus::Ok,
                detail: t!("doctor.available_in_path").into_owned(),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: t!("doctor.cargo_ndk_title").into_owned(),
                status: CheckStatus::Error,
                detail: t!("doctor.cargo_ndk_missing").into_owned(),
                fix_hint: Some("cargo install cargo-ndk".to_string()),
            });
        }

        items
    }

    fn check_android() -> Vec<DoctorItem> {
        let mut items = Vec::new();

        // JDK / Java
        match Command::new("java").arg("-version").output() {
            Ok(out) => {
                let v = String::from_utf8_lossy(if out.stderr.is_empty() {
                    &out.stdout
                } else {
                    &out.stderr
                });
                let java_installed = t!("doctor.java_installed");
                let first_line = v.lines().next().unwrap_or(&java_installed);
                items.push(DoctorItem {
                    category: "Android".to_string(),
                    title: "Java Development Kit (JDK)".to_string(),
                    status: CheckStatus::Ok,
                    detail: first_line.to_string(),
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "Android".to_string(),
                    title: "Java Development Kit (JDK)".to_string(),
                    status: CheckStatus::Error,
                    detail: t!("doctor.jdk_missing").into_owned(),
                    fix_hint: Some(t!("doctor.jdk_hint").into_owned()),
                });
            }
        }

        // ANDROID_HOME / SDK
        let explicit_sdk = env::var("ANDROID_HOME")
            .or_else(|_| env::var("ANDROID_SDK_ROOT"))
            .map(PathBuf::from);

        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from);

        let default_sdk_path = if cfg!(target_os = "macos") {
            home_dir.as_ref().map(|h| h.join("Library/Android/sdk"))
        } else if cfg!(target_os = "windows") {
            env::var("LOCALAPPDATA")
                .ok()
                .map(|l| PathBuf::from(l).join("Android/Sdk"))
                .or_else(|| {
                    home_dir
                        .as_ref()
                        .map(|h| h.join("AppData/Local/Android/Sdk"))
                })
        } else {
            home_dir.as_ref().map(|h| h.join("Android/Sdk"))
        };

        let sdk_detected = match explicit_sdk {
            Ok(ref p) if p.is_dir() => Some((p.clone(), true)),
            _ => default_sdk_path.filter(|p| p.is_dir()).map(|p| (p, false)),
        };

        match sdk_detected {
            Some((ref p, is_explicit)) => {
                let detail = if is_explicit {
                    t!("doctor.sdk_located", path = p.display()).into_owned()
                } else {
                    t!("doctor.sdk_located_default", path = p.display()).into_owned()
                };
                let fix_hint = if !is_explicit {
                    Some(t!("doctor.sdk_export_hint", path = p.display()).into_owned())
                } else {
                    None
                };
                items.push(DoctorItem {
                    category: "Android".to_string(),
                    title: "Android SDK".to_string(),
                    status: CheckStatus::Ok,
                    detail,
                    fix_hint,
                });

                // NDK check
                let ndk_dir = p.join("ndk");
                let explicit_ndk = env::var("ANDROID_NDK_HOME").map(PathBuf::from);
                let has_ndk = explicit_ndk.map(|d| d.is_dir()).unwrap_or(false)
                    || (ndk_dir.is_dir()
                        && ndk_dir
                            .read_dir()
                            .map(|mut r| r.next().is_some())
                            .unwrap_or(false));

                if has_ndk {
                    items.push(DoctorItem {
                        category: "Android".to_string(),
                        title: "Android NDK".to_string(),
                        status: CheckStatus::Ok,
                        detail: t!("doctor.ndk_located").into_owned(),
                        fix_hint: None,
                    });
                } else {
                    items.push(DoctorItem {
                        category: "Android".to_string(),
                        title: "Android NDK".to_string(),
                        status: CheckStatus::Error,
                        detail: t!("doctor.ndk_missing", path = p.display()).into_owned(),
                        fix_hint: Some(t!("doctor.ndk_hint").into_owned()),
                    });
                }
            }
            None => {
                items.push(DoctorItem {
                    category: "Android".to_string(),
                    title: "Android SDK".to_string(),
                    status: CheckStatus::Error,
                    detail: t!("doctor.sdk_missing").into_owned(),
                    fix_hint: Some(t!("doctor.sdk_hint").into_owned()),
                });
            }
        }

        // adb & devices
        if which::which("adb").is_ok() {
            let devices = Command::new("adb").arg("devices").output();
            let mut dev_list = Vec::new();
            if let Ok(out) = devices {
                for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1] == "device" {
                        dev_list.push(parts[0].to_string());
                    }
                }
            }

            let detail = if dev_list.is_empty() {
                t!("doctor.adb_no_devices").into_owned()
            } else {
                t!(
                    "doctor.adb_devices",
                    count = dev_list.len(),
                    devices = dev_list.join(", ")
                )
                .into_owned()
            };

            items.push(DoctorItem {
                category: "Android".to_string(),
                title: "Android Debug Bridge (adb)".to_string(),
                status: CheckStatus::Ok,
                detail,
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "Android".to_string(),
                title: "Android Debug Bridge (adb)".to_string(),
                status: CheckStatus::Warning,
                detail: t!("doctor.adb_missing").into_owned(),
                fix_hint: Some(t!("doctor.adb_hint").into_owned()),
            });
        }

        items
    }

    fn check_ios() -> Vec<DoctorItem> {
        let mut items = Vec::new();
        let is_macos = cfg!(target_os = "macos");

        if !is_macos {
            items.push(DoctorItem {
                category: "iOS".to_string(),
                title: t!("doctor.ios_env_title").into_owned(),
                status: CheckStatus::Warning,
                detail: t!("doctor.ios_env_linux").into_owned(),
                fix_hint: None,
            });
            return items;
        }

        // Xcode check
        match Command::new("xcode-select").arg("-p").output() {
            Ok(out) if out.status.success() => {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                items.push(DoctorItem {
                    category: "iOS".to_string(),
                    title: "Xcode & Command Line Tools".to_string(),
                    status: CheckStatus::Ok,
                    detail: t!("doctor.xcode_configured", path = path).into_owned(),
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "iOS".to_string(),
                    title: "Xcode & Command Line Tools".to_string(),
                    status: CheckStatus::Error,
                    detail: t!("doctor.xcode_missing").into_owned(),
                    fix_hint: Some(t!("doctor.xcode_hint").into_owned()),
                });
            }
        }

        // XcodeGen
        if which::which("xcodegen").is_ok() {
            items.push(DoctorItem {
                category: "iOS".to_string(),
                title: "XcodeGen".to_string(),
                status: CheckStatus::Ok,
                detail: t!("doctor.available_in_path").into_owned(),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "iOS".to_string(),
                title: "XcodeGen".to_string(),
                status: CheckStatus::Warning,
                detail: t!("doctor.xcodegen_missing").into_owned(),
                fix_hint: Some("brew install xcodegen".to_string()),
            });
        }

        // simctl
        match Command::new("xcrun")
            .args(["simctl", "list", "devices", "booted"])
            .output()
        {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout);
                let booted_count = v.lines().filter(|l| l.contains("(Booted)")).count();
                items.push(DoctorItem {
                    category: "iOS".to_string(),
                    title: t!("doctor.simctl_title").into_owned(),
                    status: CheckStatus::Ok,
                    detail: t!("doctor.simctl_ok", count = booted_count).into_owned(),
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "iOS".to_string(),
                    title: t!("doctor.simctl_title").into_owned(),
                    status: CheckStatus::Warning,
                    detail: t!("doctor.simctl_failed").into_owned(),
                    fix_hint: None,
                });
            }
        }

        items
    }

    fn check_desktop() -> Vec<DoctorItem> {
        let mut items = Vec::new();

        // Check platform and Apple Silicon constraint
        if cfg!(target_os = "macos") {
            let arch = Command::new("uname").arg("-m").output();
            let is_arm64 = arch
                .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "arm64")
                .unwrap_or(false);

            if is_arm64 {
                items.push(DoctorItem {
                    category: "Desktop".to_string(),
                    title: t!("doctor.macos_arch_title").into_owned(),
                    status: CheckStatus::Ok,
                    detail: t!("doctor.macos_arm64_ok").into_owned(),
                    fix_hint: None,
                });
            } else {
                items.push(DoctorItem {
                    category: "Desktop".to_string(),
                    title: t!("doctor.macos_arch_title").into_owned(),
                    status: CheckStatus::Error,
                    detail: t!("doctor.macos_intel").into_owned(),
                    fix_hint: Some(t!("doctor.macos_intel_hint").into_owned()),
                });
            }
        } else if cfg!(target_os = "linux") {
            let has_pkg_config = which::which("pkg-config").is_ok();
            items.push(DoctorItem {
                category: "Desktop".to_string(),
                title: t!("doctor.linux_desktop_title").into_owned(),
                status: if has_pkg_config {
                    CheckStatus::Ok
                } else {
                    CheckStatus::Warning
                },
                detail: if has_pkg_config {
                    t!("doctor.pkg_config_ok").into_owned()
                } else {
                    t!("doctor.pkg_config_missing").into_owned()
                },
                fix_hint: if has_pkg_config {
                    None
                } else {
                    Some(
                        "sudo apt install pkg-config libxkbcommon-dev libfontconfig1-dev"
                            .to_string(),
                    )
                },
            });
        } else if cfg!(target_os = "windows") {
            items.push(DoctorItem {
                category: "Desktop".to_string(),
                title: t!("doctor.windows_desktop_title").into_owned(),
                status: CheckStatus::Ok,
                detail: t!("doctor.windows_desktop_ok").into_owned(),
                fix_hint: None,
            });
        }

        items
    }

    fn check_tooling() -> Vec<DoctorItem> {
        let mut items = Vec::new();

        // sccache
        if which::which("sccache").is_ok() {
            items.push(DoctorItem {
                category: CATEGORY_TOOLING.to_string(),
                title: t!("doctor.sccache_title").into_owned(),
                status: CheckStatus::Ok,
                detail: t!("doctor.sccache_ok").into_owned(),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: CATEGORY_TOOLING.to_string(),
                title: t!("doctor.sccache_title").into_owned(),
                status: CheckStatus::Warning,
                detail: t!("doctor.sccache_missing").into_owned(),
                fix_hint: Some("cargo install sccache".to_string()),
            });
        }

        items
    }

    pub fn print_report(report: &DoctorReport) {
        println!("{}", t!("doctor.report_title").bold());
        println!(
            "{}",
            "══════════════════════════════════════════════════════════".cyan()
        );

        let mut current_cat = "";
        for item in &report.items {
            if item.category != current_cat {
                println!(
                    "\n{}",
                    format!(
                        "── {} ──────────────────────────────",
                        category_label(&item.category)
                    )
                    .bright_black()
                );
                current_cat = &item.category;
            }

            match item.status {
                CheckStatus::Ok => {
                    println!(
                        "  {} {} {}",
                        "✔".green().bold(),
                        item.title.bold(),
                        format!("({})", item.detail).bright_black()
                    );
                }
                CheckStatus::Warning => {
                    println!(
                        "  {} {} {}",
                        "!".yellow().bold(),
                        item.title.bold(),
                        format!("— {}", item.detail).yellow()
                    );
                    if let Some(ref fix) = item.fix_hint {
                        println!("    {} {}", t!("doctor.suggestion").cyan(), fix.bold());
                    }
                }
                CheckStatus::Error => {
                    println!(
                        "  {} {} {}",
                        "✖".red().bold(),
                        item.title.bold(),
                        format!("— {}", item.detail).red()
                    );
                    if let Some(ref fix) = item.fix_hint {
                        println!(
                            "    {} {}",
                            t!("doctor.solution").cyan().bold(),
                            t!("doctor.run_command", command = fix.green().bold())
                        );
                    }
                }
            }
        }

        println!(
            "\n{}",
            "══════════════════════════════════════════════════════════".cyan()
        );
        if report.error_count == 0 && report.warning_count == 0 {
            println!("{} {}", "✔".green().bold(), t!("doctor.all_ready"));
        } else if report.error_count == 0 {
            println!(
                "{} {}",
                "!".yellow().bold(),
                t!("doctor.ready_with_warnings", count = report.warning_count)
            );
        } else {
            println!(
                "{} {}",
                "✖".red().bold(),
                t!("doctor.has_errors", count = report.error_count)
            );
        }
        println!();
    }
}
