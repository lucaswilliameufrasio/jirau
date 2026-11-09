use crate::config::JirauConfig;
use crate::ui;
use anyhow::{Context, Result};
use colored::*;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rust_i18n::t;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

pub struct DevRunner;

impl DevRunner {
    pub fn run(
        root_dir: &Path,
        config: &JirauConfig,
        requested_platform: Option<String>,
        watch: bool,
    ) -> Result<()> {
        ui::print_banner();

        // 1. Determine platform
        let platform = match requested_platform {
            Some(p) => p.to_lowercase(),
            None => Self::detect_or_select_platform(config)?,
        };

        ui::print_step(&t!(
            "dev.starting_flow",
            platform = platform.to_uppercase().cyan().bold()
        ));

        match platform.as_str() {
            "android" => Self::run_android_dev(root_dir, config, watch)?,
            "ios" => Self::run_ios_dev(root_dir, config, watch)?,
            "desktop" => Self::run_desktop_dev(root_dir, config, watch)?,
            other => anyhow::bail!("{}", t!("dev.unknown_platform", platform = other)),
        }

        Ok(())
    }

    fn detect_or_select_platform(config: &JirauConfig) -> Result<String> {
        // Test adb connected devices
        let has_adb_device = Command::new(crate::android_pipeline::adb())
            .arg("devices")
            .output()
            .map(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .skip(1)
                    .any(|l| l.split_whitespace().nth(1) == Some("device"))
            })
            .unwrap_or(false);

        let is_macos = cfg!(target_os = "macos");

        if has_adb_device && !is_macos {
            ui::print_info(&t!("dev.adb_device_detected"));
            return Ok("android".to_string());
        }

        // If interactive, prompt user
        let options = config.app.platforms.clone();
        if options.is_empty() {
            return Ok("android".to_string());
        }

        if options.len() == 1 {
            return Ok(options[0].clone());
        }

        let ans = inquire::Select::new(&t!("dev.select_platform"), options)
            .with_help_message(&t!("dev.select_help"))
            .prompt();

        match ans {
            Ok(choice) => Ok(choice),
            Err(_) => anyhow::bail!("{}", t!("dev.cancelled")),
        }
    }

    fn run_android_dev(root_dir: &Path, config: &JirauConfig, watch: bool) -> Result<()> {
        let dev = crate::run::RunEngine::select_or_detect_android_device(None)?;
        crate::android_pipeline::deploy_to_device(root_dir, config, &dev)?;
        if watch {
            crate::android_pipeline::watch_and_redeploy(root_dir, config, &dev)?;
        }
        Ok(())
    }

    fn run_ios_dev(root_dir: &Path, config: &JirauConfig, watch: bool) -> Result<()> {
        let sim = crate::run::RunEngine::select_or_detect_ios_simulator(None)?;
        ui::print_substep(&t!(
            "dev.target_simulator",
            name = sim.name.cyan().bold(),
            udid = sim.udid.yellow()
        ));

        crate::ios_pipeline::deploy_to_simulator(root_dir, config, &sim)?;

        if watch {
            crate::ios_pipeline::watch_and_redeploy(root_dir, config, &sim)?;
        }

        Ok(())
    }

    fn run_desktop_dev(root_dir: &Path, config: &JirauConfig, watch: bool) -> Result<()> {
        let slug = config.app.name.to_lowercase().replace(['_', ' '], "-");
        let pkg_name = format!("{}-desktop", slug);

        ui::print_info(&t!(
            "dev.desktop_starting",
            package = pkg_name.cyan().bold()
        ));

        if !watch {
            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args(["run", "--package", &pkg_name])
                .status()
                .with_context(|| t!("dev.exec_failed", package = pkg_name).into_owned())?;

            if !status.success() {
                anyhow::bail!("{}", t!("dev.desktop_failed"));
            }
            return Ok(());
        }

        ui::print_info(&t!("dev.watching_crates"));

        let mut child = Command::new("cargo")
            .current_dir(root_dir)
            .args(["run", "--package", &pkg_name])
            .spawn()
            .with_context(|| t!("dev.start_failed", package = pkg_name).into_owned())?;

        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        let crates_dir = root_dir.join("crates");

        watcher.watch(&crates_dir, RecursiveMode::Recursive)?;
        let mut last_build = Instant::now();

        loop {
            // Verifica se a janela foi fechada pelo usuário
            if let Ok(Some(_)) = child.try_wait() {
                ui::print_info(&t!("dev.desktop_closed"));
                break;
            }

            if let Ok(Ok(Event {
                kind: EventKind::Modify(_),
                paths,
                ..
            })) = rx.recv_timeout(Duration::from_millis(500))
            {
                if last_build.elapsed() < Duration::from_millis(1500) {
                    continue;
                }

                let changed_rs = paths
                    .iter()
                    .any(|p| p.extension().is_some_and(|ext| ext == "rs"));
                if !changed_rs {
                    continue;
                }

                let relative_paths: Vec<String> = paths
                    .iter()
                    .filter_map(|p| {
                        p.strip_prefix(root_dir)
                            .ok()
                            .map(|d| d.display().to_string())
                    })
                    .collect();

                println!(
                    "{} {}",
                    "➜".cyan().bold(),
                    t!(
                        "dev.change_detected",
                        paths = relative_paths.join(", ").yellow()
                    )
                );

                let _ = child.kill();
                let _ = child.wait();

                let spinner = ui::create_spinner(&t!("dev.restarting"));
                let start = Instant::now();

                match Command::new("cargo")
                    .current_dir(root_dir)
                    .args(["run", "--package", &pkg_name])
                    .spawn()
                {
                    Ok(new_child) => {
                        child = new_child;
                        ui::finish_spinner_success(
                            &spinner,
                            &t!(
                                "dev.restarted",
                                secs = format!("{:.2}", start.elapsed().as_secs_f32())
                            ),
                        );
                        println!();
                    }
                    Err(e) => {
                        ui::finish_spinner_error(&spinner, &t!("dev.restart_failed", error = e));
                        println!();
                    }
                }

                last_build = Instant::now();
            }
        }

        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    }
}
