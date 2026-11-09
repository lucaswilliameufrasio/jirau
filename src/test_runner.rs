use crate::config::JirauConfig;
use crate::ui;
use anyhow::Result;
use colored::*;
use rust_i18n::t;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub struct TestRunner;

#[derive(Debug)]
pub struct TestResultItem {
    pub name: String,
    pub duration_secs: f32,
    pub passed: bool,
    pub details: String,
}

impl TestRunner {
    pub fn run(
        root_dir: &Path,
        config: &JirauConfig,
        target: Option<String>,
        all: bool,
    ) -> Result<()> {
        ui::print_banner();
        println!("{}", t!("test_runner.title").cyan().bold());
        println!("{}", "═".repeat(60));

        let mut results = Vec::new();
        let target = target.as_deref().unwrap_or("core");

        // 1. Rust Workspace / Core Unit Tests
        if target == "core" || target == "rust" || target == "all" || all {
            results.push(Self::run_rust_core_tests(root_dir)?);
        }

        // 2. FFI Integration / Lifecycle Tests
        if target == "ffi" || target == "all" || all {
            results.push(Self::run_ffi_tests(root_dir, config)?);
        }

        // 3. Android Unit Tests
        if (target == "android" || target == "all" || all)
            && config.app.platforms.contains(&"android".to_string())
        {
            results.push(Self::run_android_tests(root_dir)?);
        }

        // 4. iOS Unit Tests
        if (target == "ios" || target == "all" || all)
            && config.app.platforms.contains(&"ios".to_string())
        {
            results.push(Self::run_ios_tests(root_dir, config)?);
        }

        // 5. Desktop Tests
        if (target == "desktop" || target == "all" || all)
            && config.app.platforms.contains(&"desktop".to_string())
        {
            results.push(Self::run_desktop_tests(root_dir, config)?);
        }

        // Summary report
        println!("\n{}", "═".repeat(60));
        println!("{}", t!("test_runner.summary_title").bold());
        let mut all_passed = true;

        for r in &results {
            if r.passed {
                println!(
                    "  {} {} ({:.2}s) — {}",
                    "✔".green().bold(),
                    r.name.bold(),
                    r.duration_secs,
                    r.details.dimmed()
                );
            } else {
                all_passed = false;
                println!(
                    "  {} {} ({:.2}s) — {}",
                    "✖".red().bold(),
                    r.name.bold(),
                    r.duration_secs,
                    r.details.red()
                );
            }
        }
        println!("{}", "═".repeat(60));

        if all_passed {
            println!("{}", t!("test_runner.all_passed").green().bold());
            Ok(())
        } else {
            anyhow::bail!("{}", t!("test_runner.some_failed"));
        }
    }

    fn run_rust_core_tests(root_dir: &Path) -> Result<TestResultItem> {
        let spinner = ui::create_spinner(&t!("test_runner.core_running"));
        let start = Instant::now();

        let out = Command::new("cargo")
            .current_dir(root_dir)
            .args(["test", "--workspace"])
            .output();

        let duration = start.elapsed().as_secs_f32();

        match out {
            Ok(o) if o.status.success() => {
                ui::finish_spinner_success(
                    &spinner,
                    &t!("test_runner.core_done", secs = format!("{:.2}", duration)),
                );
                Ok(TestResultItem {
                    name: "Rust Core Workspace".to_string(),
                    duration_secs: duration,
                    passed: true,
                    details: t!("test_runner.core_details").into_owned(),
                })
            }
            Ok(o) => {
                let err_msg = String::from_utf8_lossy(&o.stderr);
                ui::finish_spinner_error(&spinner, &t!("test_runner.core_failed"));
                Ok(TestResultItem {
                    name: "Rust Core Workspace".to_string(),
                    duration_secs: duration,
                    passed: false,
                    details: err_msg.lines().take(3).collect::<Vec<_>>().join(" "),
                })
            }
            Err(e) => {
                ui::finish_spinner_error(&spinner, &t!("test_runner.core_spawn_failed"));
                Ok(TestResultItem {
                    name: "Rust Core Workspace".to_string(),
                    duration_secs: duration,
                    passed: false,
                    details: e.to_string(),
                })
            }
        }
    }

    fn run_ffi_tests(root_dir: &Path, config: &JirauConfig) -> Result<TestResultItem> {
        let ffi_pkg = format!(
            "{}-ffi",
            config.app.name.to_lowercase().replace(['_', ' '], "-")
        );
        let spinner = ui::create_spinner(&t!("test_runner.ffi_running", pkg = ffi_pkg));
        let start = Instant::now();

        let out = Command::new("cargo")
            .current_dir(root_dir)
            .args(["test", "-p", &ffi_pkg])
            .output();

        let duration = start.elapsed().as_secs_f32();

        match out {
            Ok(o) if o.status.success() => {
                ui::finish_spinner_success(
                    &spinner,
                    &t!("test_runner.ffi_done", secs = format!("{:.2}", duration)),
                );
                Ok(TestResultItem {
                    name: t!("test_runner.ffi_name").into_owned(),
                    duration_secs: duration,
                    passed: true,
                    details: t!("test_runner.ffi_details").into_owned(),
                })
            }
            Ok(o) => {
                let err_msg = String::from_utf8_lossy(&o.stderr);
                ui::finish_spinner_error(&spinner, &t!("test_runner.ffi_failed"));
                Ok(TestResultItem {
                    name: t!("test_runner.ffi_name").into_owned(),
                    duration_secs: duration,
                    passed: false,
                    details: err_msg.lines().take(3).collect::<Vec<_>>().join(" "),
                })
            }
            Err(e) => {
                ui::finish_spinner_error(&spinner, &t!("test_runner.ffi_spawn_failed"));
                Ok(TestResultItem {
                    name: t!("test_runner.ffi_name").into_owned(),
                    duration_secs: duration,
                    passed: false,
                    details: e.to_string(),
                })
            }
        }
    }

    /// Testes instrumentados no dispositivo/emulador conectado: ViewModel,
    /// Keystore e UI Compose rodando contra o núcleo Rust real.
    fn run_android_tests(root_dir: &Path) -> Result<TestResultItem> {
        const NAME: &str = "Android Instrumented Tests (Compose + Rust)";
        let start = Instant::now();
        let failed = |details: String| TestResultItem {
            name: NAME.to_string(),
            duration_secs: start.elapsed().as_secs_f32(),
            passed: false,
            details,
        };

        let devices = crate::run::RunEngine::list_android_devices();
        let Some(dev) = devices.first() else {
            return Ok(failed(t!("test_runner.android_no_device").into_owned()));
        };

        let spinner = ui::create_spinner(&t!(
            "test_runner.android_running",
            model = dev.model,
            serial = dev.serial
        ));
        match crate::android_pipeline::run_instrumented_tests(root_dir, dev) {
            Ok(summary) => {
                ui::finish_spinner_success(&spinner, &t!("test_runner.android_done"));
                Ok(TestResultItem {
                    name: NAME.to_string(),
                    duration_secs: start.elapsed().as_secs_f32(),
                    passed: true,
                    details: summary,
                })
            }
            Err(e) => {
                ui::finish_spinner_error(&spinner, &t!("test_runner.android_failed"));
                Ok(failed(format!("{:#}", e)))
            }
        }
    }

    fn run_ios_tests(root_dir: &Path, config: &JirauConfig) -> Result<TestResultItem> {
        const NAME: &str = "iOS Unit Tests (XCTest)";
        if !cfg!(target_os = "macos") {
            return Ok(TestResultItem {
                name: NAME.to_string(),
                duration_secs: 0.0,
                passed: true,
                details: t!("test_runner.ios_skipped").into_owned(),
            });
        }

        let start = Instant::now();
        let failed = |details: String, start: Instant| TestResultItem {
            name: NAME.to_string(),
            duration_secs: start.elapsed().as_secs_f32(),
            passed: false,
            details,
        };

        // Usa o simulador ligado (ou o primeiro disponível) em vez de um modelo fixo.
        let sims = crate::run::RunEngine::list_ios_simulators();
        let sim = match sims
            .iter()
            .find(|s| s.state.eq_ignore_ascii_case("booted"))
            .or_else(|| sims.iter().find(|s| s.name.starts_with("iPhone")))
        {
            Some(sim) => sim.clone(),
            None => {
                return Ok(failed(
                    t!("test_runner.ios_no_simulator").into_owned(),
                    start,
                ));
            }
        };

        let prepared = crate::ios_pipeline::ensure_booted(&sim)
            .and_then(|_| crate::ios_pipeline::build_ffi(root_dir, false))
            .and_then(|_| crate::ios_pipeline::generate_xcode_project(root_dir));
        if let Err(e) = prepared {
            return Ok(failed(format!("{:#}", e), start));
        }

        let names = crate::ios_pipeline::IosNames::from_config(config);
        let ios_dir = root_dir.join("ios");
        let spinner = ui::create_spinner(&t!("test_runner.ios_running", name = sim.name));

        let out = Command::new("xcodebuild")
            .current_dir(&ios_dir)
            .arg("test")
            .args(["-project", &format!("{}.xcodeproj", names.pascal_name)])
            .args(["-scheme", &names.pascal_name])
            .args([
                "-destination",
                &format!("platform=iOS Simulator,id={}", sim.udid),
            ])
            .arg("-derivedDataPath")
            .arg(ios_dir.join("DerivedData"))
            .output();

        let duration = start.elapsed().as_secs_f32();
        match out {
            Ok(o) => {
                let log = String::from_utf8_lossy(&o.stdout);
                let summary = log
                    .lines()
                    .rev()
                    .find(|l| l.trim_start().starts_with("Executed "))
                    .map(|l| l.trim().to_string())
                    .unwrap_or_default();
                if o.status.success() {
                    ui::finish_spinner_success(
                        &spinner,
                        &t!("test_runner.ios_done", secs = format!("{:.2}", duration)),
                    );
                    Ok(TestResultItem {
                        name: NAME.to_string(),
                        duration_secs: duration,
                        passed: true,
                        details: summary,
                    })
                } else {
                    ui::finish_spinner_error(&spinner, &t!("test_runner.ios_failed"));
                    let failures: Vec<&str> = log
                        .lines()
                        .filter(|l| l.contains("error:") || l.contains("' failed ("))
                        .take(10)
                        .collect();
                    Ok(TestResultItem {
                        name: NAME.to_string(),
                        duration_secs: duration,
                        passed: false,
                        details: if failures.is_empty() {
                            summary
                        } else {
                            failures.join("\n")
                        },
                    })
                }
            }
            Err(e) => {
                ui::finish_spinner_error(&spinner, &t!("test_runner.ios_spawn_failed"));
                Ok(failed(e.to_string(), start))
            }
        }
    }

    fn run_desktop_tests(root_dir: &Path, config: &JirauConfig) -> Result<TestResultItem> {
        let pkg_name = format!(
            "{}-desktop",
            config.app.name.to_lowercase().replace(['_', ' '], "-")
        );
        let spinner = ui::create_spinner(&t!("test_runner.desktop_running", pkg = pkg_name));
        let start = Instant::now();

        let out = Command::new("cargo")
            .current_dir(root_dir)
            .args(["check", "--package", &pkg_name])
            .output();

        let duration = start.elapsed().as_secs_f32();

        match out {
            Ok(o) if o.status.success() => {
                ui::finish_spinner_success(
                    &spinner,
                    &t!(
                        "test_runner.desktop_done",
                        secs = format!("{:.2}", duration)
                    ),
                );
                Ok(TestResultItem {
                    name: "Desktop (Rune UI)".to_string(),
                    duration_secs: duration,
                    passed: true,
                    details: t!("test_runner.desktop_details").into_owned(),
                })
            }
            Ok(o) => {
                let err_msg = String::from_utf8_lossy(&o.stderr);
                ui::finish_spinner_error(&spinner, &t!("test_runner.desktop_failed"));
                Ok(TestResultItem {
                    name: "Desktop (Rune UI)".to_string(),
                    duration_secs: duration,
                    passed: false,
                    details: err_msg.lines().take(3).collect::<Vec<_>>().join(" "),
                })
            }
            Err(e) => {
                ui::finish_spinner_error(&spinner, &t!("test_runner.desktop_spawn_failed"));
                Ok(TestResultItem {
                    name: "Desktop (Rune UI)".to_string(),
                    duration_secs: duration,
                    passed: false,
                    details: e.to_string(),
                })
            }
        }
    }
}
