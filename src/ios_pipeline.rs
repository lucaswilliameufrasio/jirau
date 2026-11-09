//! Pipeline iOS compartilhado por `jirau run`, `dev`, `build`, `test` e `bindings`.
//!
//! A compilação do núcleo Rust, a geração dos bindings Swift e a montagem do
//! XCFramework vivem no script `scripts/build-ios-ffi.sh` do projeto gerado,
//! para que `make`, CI e quem usa só o Xcode executem exatamente o mesmo fluxo.

use crate::config::JirauConfig;
use crate::run::IosSimulator;
use crate::templates::TemplateContext;
use crate::ui;
use anyhow::{Result, bail};
use colored::*;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

pub const FFI_SCRIPT: &str = "scripts/build-ios-ffi.sh";

/// Nomes derivados exatamente como no scaffold (`TemplateContext`).
pub struct IosNames {
    pub pascal_name: String,
    pub bundle_id: String,
}

impl IosNames {
    pub fn from_config(config: &JirauConfig) -> Self {
        let ctx = TemplateContext::new(&config.app.name, &config.app.package_id, Vec::new());
        Self {
            pascal_name: ctx.pascal_name,
            bundle_id: config.app.package_id.clone(),
        }
    }
}

fn require_macos() -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!("{}", t!("ios_pipeline.requires_macos"));
    }
    Ok(())
}

fn run_quiet(cmd: &mut Command, what: &str) -> Result<()> {
    crate::process::run_quiet(cmd, what).map(|_| ())
}

/// Compila o núcleo Rust para iOS, gera os bindings Swift e o XCFramework.
pub fn build_ffi(root_dir: &Path, release: bool) -> Result<()> {
    require_macos()?;
    let script = root_dir.join(FFI_SCRIPT);
    if !script.exists() {
        bail!("{}", t!("ios_pipeline.script_missing", script = FFI_SCRIPT));
    }

    let profile = if release { "release" } else { "debug" };
    let spinner = ui::create_spinner(&t!("ios_pipeline.ffi_building", profile = profile));
    let result = run_quiet(
        Command::new("bash")
            .arg(&script)
            .arg(profile)
            .current_dir(root_dir),
        &t!("ios_pipeline.ffi_step"),
    );
    match result {
        Ok(()) => {
            ui::finish_spinner_success(&spinner, &t!("ios_pipeline.ffi_ready"));
            Ok(())
        }
        Err(e) => {
            ui::finish_spinner_error(&spinner, &t!("ios_pipeline.ffi_failed"));
            Err(e)
        }
    }
}

/// Regera o .xcodeproj a partir de ios/project.yml.
pub fn generate_xcode_project(root_dir: &Path) -> Result<()> {
    let ios_dir = root_dir.join("ios");
    if !ios_dir.join("project.yml").exists() {
        bail!("{}", t!("ios_pipeline.project_yml_missing"));
    }
    if which::which("xcodegen").is_err() {
        bail!("{}", t!("ios_pipeline.xcodegen_missing"));
    }
    let spinner = ui::create_spinner(&t!("ios_pipeline.xcodegen_running"));
    match run_quiet(
        Command::new("xcodegen")
            .args(["generate", "--quiet"])
            .current_dir(&ios_dir),
        "xcodegen generate",
    ) {
        Ok(()) => {
            ui::finish_spinner_success(&spinner, &t!("ios_pipeline.xcodegen_done"));
            Ok(())
        }
        Err(e) => {
            ui::finish_spinner_error(&spinner, &t!("ios_pipeline.xcodegen_failed"));
            Err(e)
        }
    }
}

/// Compila o app para um simulador e devolve o caminho do `.app`.
pub fn build_app_for_simulator(root_dir: &Path, names: &IosNames, udid: &str) -> Result<PathBuf> {
    let ios_dir = root_dir.join("ios");
    let project = format!("{}.xcodeproj", names.pascal_name);
    let derived = ios_dir.join("DerivedData");

    let spinner = ui::create_spinner(&t!("ios_pipeline.app_building"));
    let result = run_quiet(
        Command::new("xcodebuild")
            .current_dir(&ios_dir)
            .args(["-project", &project, "-scheme", &names.pascal_name])
            .args(["-configuration", "Debug"])
            .args([
                "-destination",
                &format!("platform=iOS Simulator,id={}", udid),
            ])
            .arg("-derivedDataPath")
            .arg(&derived)
            // Assinatura ad-hoc (project.yml): o Keychain exige entitlements.
            .arg("build"),
        "xcodebuild",
    );
    if let Err(e) = result {
        ui::finish_spinner_error(&spinner, &t!("ios_pipeline.app_build_failed"));
        return Err(e);
    }

    let app = derived
        .join("Build/Products/Debug-iphonesimulator")
        .join(format!("{}.app", names.pascal_name));
    if !app.exists() {
        ui::finish_spinner_error(&spinner, &t!("ios_pipeline.app_missing_after_build"));
        bail!("{}", t!("ios_pipeline.app_not_found", path = app.display()));
    }
    ui::finish_spinner_success(&spinner, &t!("ios_pipeline.app_built"));
    Ok(app)
}

/// Garante que o simulador está ligado e com o app Simulator aberto.
pub fn ensure_booted(sim: &IosSimulator) -> Result<()> {
    if !sim.state.eq_ignore_ascii_case("booted") {
        ui::print_step(&t!(
            "ios_pipeline.booting_simulator",
            name = sim.name.cyan()
        ));
        // Ignora o erro "já está ligado"; bootstatus confirma o estado real.
        let _ = Command::new("xcrun")
            .args(["simctl", "boot", &sim.udid])
            .output();
    }
    run_quiet(
        Command::new("xcrun").args(["simctl", "bootstatus", &sim.udid, "-b"]),
        "simctl bootstatus",
    )?;
    open_simulator_app();
    Ok(())
}

/// Abre o Simulator.app do Xcode ativo (`open -a Simulator` falha quando o
/// LaunchServices não indexou o app, comum com múltiplos Xcodes instalados).
fn open_simulator_app() {
    let from_xcode = Command::new("xcode-select")
        .arg("-p")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            PathBuf::from(String::from_utf8_lossy(&o.stdout).trim())
                .join("Applications/Simulator.app")
        })
        .filter(|p| p.exists());

    let mut cmd = Command::new("open");
    match from_xcode {
        Some(app) => cmd.arg(app),
        None => cmd.args(["-a", "Simulator"]),
    };
    let _ = cmd.output();
}

pub fn install_and_launch(udid: &str, app: &Path, bundle_id: &str) -> Result<()> {
    let spinner = ui::create_spinner(&t!("ios_pipeline.installing"));
    let result = run_quiet(
        Command::new("xcrun")
            .args(["simctl", "install", udid])
            .arg(app),
        "simctl install",
    )
    .and_then(|_| {
        run_quiet(
            Command::new("xcrun").args([
                "simctl",
                "launch",
                "--terminate-running-process",
                udid,
                bundle_id,
            ]),
            "simctl launch",
        )
    });
    match result {
        Ok(()) => {
            ui::finish_spinner_success(
                &spinner,
                &t!("ios_pipeline.app_running", bundle_id = bundle_id),
            );
            Ok(())
        }
        Err(e) => {
            ui::finish_spinner_error(&spinner, &t!("ios_pipeline.install_failed"));
            Err(e)
        }
    }
}

/// Fluxo completo: FFI → XcodeGen → xcodebuild → install → launch.
pub fn deploy_to_simulator(
    root_dir: &Path,
    config: &JirauConfig,
    sim: &IosSimulator,
) -> Result<()> {
    require_macos()?;
    let names = IosNames::from_config(config);
    ensure_booted(sim)?;
    build_ffi(root_dir, false)?;
    generate_xcode_project(root_dir)?;
    let app = build_app_for_simulator(root_dir, &names, &sim.udid)?;
    install_and_launch(&sim.udid, &app, &names.bundle_id)
}

/// Observa `crates/` e `ios/App/` e reimplanta o app a cada alteração.
pub fn watch_and_redeploy(root_dir: &Path, config: &JirauConfig, sim: &IosSimulator) -> Result<()> {
    ui::print_info(&t!("ios_pipeline.watching"));

    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(&root_dir.join("crates"), RecursiveMode::Recursive)?;
    watcher.watch(&root_dir.join("ios/App"), RecursiveMode::Recursive)?;

    let mut last_build = Instant::now();
    loop {
        let event = match rx.recv() {
            Ok(Ok(event)) => event,
            Ok(Err(e)) => {
                ui::print_warning(&t!(
                    "ios_pipeline.watcher_error",
                    error = format!("{:?}", e)
                ));
                continue;
            }
            Err(_) => return Ok(()),
        };
        let Event { kind, paths, .. } = event;
        if !matches!(kind, EventKind::Modify(_) | EventKind::Create(_)) {
            continue;
        }
        if last_build.elapsed() < Duration::from_millis(1500) {
            continue;
        }
        let relevant = paths.iter().any(|p| {
            p.extension()
                .is_some_and(|ext| ext == "rs" || ext == "swift" || ext == "toml")
        });
        if !relevant {
            continue;
        }

        let changed: Vec<String> = paths
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
                "ios_pipeline.change_detected",
                paths = changed.join(", ").yellow()
            )
        );

        let start = Instant::now();
        match deploy_to_simulator(root_dir, config, sim) {
            Ok(()) => ui::print_success(&t!(
                "ios_pipeline.app_updated",
                secs = format!("{:.1}", start.elapsed().as_secs_f32())
            )),
            Err(e) => ui::print_error(&format!("{:#}\n", e)),
        }
        // Descarta eventos gerados pelo próprio build (bindings, xcodeproj).
        while rx.try_recv().is_ok() {}
        last_build = Instant::now();
    }
}
