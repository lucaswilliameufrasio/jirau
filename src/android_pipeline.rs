//! Pipeline Android compartilhado por `jirau run`, `dev`, `build`, `test` e `bindings`.
//!
//! O núcleo Rust (cargo-ndk → jniLibs) e os bindings Kotlin (UniFFI) são
//! produzidos por `scripts/build-android-ffi.sh`, que o próprio Gradle chama no
//! `preBuild`. Assim `jirau`, `./gradlew`, Android Studio e CI seguem o mesmo fluxo.

use crate::config::JirauConfig;
use crate::process::run_quiet;
use crate::run::AndroidDevice;
use crate::ui;
use anyhow::{Result, bail};
use colored::*;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

pub const FFI_SCRIPT: &str = "scripts/build-android-ffi.sh";
const DEFAULT_ABIS: &[&str] = &["arm64-v8a", "x86_64"];

fn upgrade_hint() -> String {
    t!("android_pipeline.upgrade_hint").into_owned()
}

fn script_missing() -> String {
    t!(
        "android_pipeline.script_missing",
        script = FFI_SCRIPT,
        hint = upgrade_hint()
    )
    .into_owned()
}

/// SDK do Android: respeita ANDROID_HOME/ANDROID_SDK_ROOT e cai para os caminhos padrão.
pub fn android_sdk_dir() -> Option<PathBuf> {
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .chain(std::env::var_os("HOME").into_iter().flat_map(|home| {
            let home = PathBuf::from(home);
            [home.join("Android/Sdk"), home.join("Library/Android/sdk")]
        }))
        .find(|p| p.is_dir())
}

/// `adb` do PATH ou do SDK.
pub fn adb() -> PathBuf {
    if let Ok(path) = which::which("adb") {
        return path;
    }
    android_sdk_dir()
        .map(|sdk| sdk.join("platform-tools/adb"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("adb"))
}

fn gradlew(root_dir: &Path) -> Result<Command> {
    let android_dir = root_dir.join("android");
    let wrapper = if cfg!(windows) {
        android_dir.join("gradlew.bat")
    } else {
        android_dir.join("gradlew")
    };
    if !wrapper.exists() {
        bail!(
            "{}",
            t!("android_pipeline.gradlew_missing", hint = upgrade_hint())
        );
    }
    if !root_dir.join(FFI_SCRIPT).exists() {
        bail!("{}", script_missing());
    }
    let mut cmd = Command::new(wrapper);
    cmd.current_dir(&android_dir);
    // O Gradle precisa do SDK; sem local.properties usa ANDROID_HOME.
    if std::env::var_os("ANDROID_HOME").is_none()
        && !android_dir.join("local.properties").exists()
        && let Some(sdk) = android_sdk_dir()
    {
        cmd.env("ANDROID_HOME", sdk);
    }
    Ok(cmd)
}

fn abis_property(abis: &[&str]) -> String {
    format!("-Pjirau.rustAbis={}", abis.join(","))
}

/// ABI principal do dispositivo (`ro.product.cpu.abi`).
pub fn device_abi(serial: &str) -> Result<String> {
    let out = Command::new(adb())
        .args(["-s", serial, "shell", "getprop", "ro.product.cpu.abi"])
        .output()?;
    let abi = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || abi.is_empty() {
        bail!("{}", t!("android_pipeline.abi_unreadable", serial = serial));
    }
    Ok(abi)
}

fn with_spinner<T>(start: &str, ok: &str, fail: &str, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let spinner = ui::create_spinner(start);
    match f() {
        Ok(v) => {
            ui::finish_spinner_success(&spinner, ok);
            Ok(v)
        }
        Err(e) => {
            ui::finish_spinner_error(&spinner, fail);
            Err(e)
        }
    }
}

/// Compila o núcleo Rust (.so por ABI) e gera os bindings Kotlin.
pub fn build_ffi(root_dir: &Path, release: bool, abis: &[&str]) -> Result<()> {
    if !root_dir.join(FFI_SCRIPT).exists() {
        bail!("{}", script_missing());
    }
    let profile = if release { "release" } else { "debug" };
    let abis = if abis.is_empty() { DEFAULT_ABIS } else { abis };
    with_spinner(
        &t!(
            "android_pipeline.ffi_building",
            abis = abis.join(", "),
            profile = profile
        ),
        &t!("android_pipeline.ffi_ready"),
        &t!("android_pipeline.ffi_failed"),
        || {
            run_quiet(
                Command::new("bash")
                    .arg(root_dir.join(FFI_SCRIPT))
                    .arg(profile)
                    .args(abis)
                    .current_dir(root_dir),
                &t!("android_pipeline.ffi_step"),
            )
            .map(|_| ())
        },
    )
}

/// Gera o APK (debug ou release) para as ABIs pedidas.
pub fn assemble(root_dir: &Path, release: bool, abis: &[&str]) -> Result<PathBuf> {
    let (task, variant) = if release {
        ("assembleRelease", "release")
    } else {
        ("assembleDebug", "debug")
    };
    let abis = if abis.is_empty() { DEFAULT_ABIS } else { abis };
    with_spinner(
        &t!("android_pipeline.apk_building", variant = variant),
        &t!("android_pipeline.apk_built"),
        &t!("android_pipeline.apk_failed"),
        || {
            run_quiet(
                gradlew(root_dir)?.arg(task).arg(abis_property(abis)),
                "Gradle",
            )
            .map(|_| ())
        },
    )?;
    Ok(root_dir.join(format!("android/app/build/outputs/apk/{variant}")))
}

/// Compila (núcleo Rust só para a ABI do aparelho), instala e abre o app.
pub fn deploy_to_device(root_dir: &Path, config: &JirauConfig, dev: &AndroidDevice) -> Result<()> {
    let abi = device_abi(&dev.serial)?;
    ui::print_step(&t!(
        "android_pipeline.target_device",
        model = dev.model.cyan().bold(),
        serial = dev.serial.yellow(),
        abi = abi.green()
    ));

    with_spinner(
        &t!("android_pipeline.installing"),
        &t!("android_pipeline.installed"),
        &t!("android_pipeline.install_failed"),
        || {
            run_quiet(
                gradlew(root_dir)?
                    .env("ANDROID_SERIAL", &dev.serial)
                    .arg("installDebug")
                    .arg(abis_property(&[abi.as_str()])),
                "Gradle installDebug",
            )
            .map(|_| ())
        },
    )?;

    launch(&dev.serial, &config.app.package_id)
}

pub fn launch(serial: &str, package_id: &str) -> Result<()> {
    let component = format!("{}/.MainActivity", package_id);
    let out = run_quiet(
        Command::new(adb()).args([
            "-s", serial, "shell", "am", "start", "-W", "-S", "-n", &component,
        ]),
        "adb am start",
    )?;
    // `am start` devolve 0 mesmo quando a Activity não existe.
    let text = String::from_utf8_lossy(&out.stdout);
    if text.contains("Error:") {
        bail!(
            "{}",
            t!(
                "android_pipeline.launch_failed",
                component = component,
                output = text.trim()
            )
        );
    }
    ui::print_success(&t!("android_pipeline.app_running", package = package_id));
    Ok(())
}

/// Testes instrumentados (ViewModel/UI contra o núcleo Rust real) no aparelho.
pub fn run_instrumented_tests(root_dir: &Path, dev: &AndroidDevice) -> Result<String> {
    let abi = device_abi(&dev.serial)?;
    run_quiet(
        gradlew(root_dir)?
            .env("ANDROID_SERIAL", &dev.serial)
            .arg("connectedDebugAndroidTest")
            .arg(abis_property(&[abi.as_str()])),
        "Gradle connectedDebugAndroidTest",
    )?;
    let reports = root_dir.join("android/app/build/outputs/androidTest-results/connected/debug");
    let (tests, failures) = count_junit_results(&reports);
    if tests == 0 {
        bail!(
            "{}",
            t!(
                "android_pipeline.no_instrumented_tests",
                path = reports.display()
            )
        );
    }
    Ok(t!(
        "android_pipeline.tests_summary",
        tests = tests,
        failures = failures,
        model = dev.model
    )
    .into_owned())
}

/// Soma `tests`/`failures` dos relatórios JUnit (TEST-*.xml) do Gradle.
fn count_junit_results(dir: &Path) -> (u32, u32) {
    let attr = |tag: &str, name: &str| -> u32 {
        tag.split(&format!(" {name}=\""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };
    let mut totals = (0, 0);
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "xml") {
            continue;
        }
        let Ok(xml) = std::fs::read_to_string(&path) else {
            continue;
        };
        for tag in xml.split("<testsuite ").skip(1) {
            let tag = tag.split('>').next().unwrap_or_default();
            let tag = format!(" {tag}");
            totals.0 += attr(&tag, "tests");
            totals.1 += attr(&tag, "failures") + attr(&tag, "errors");
        }
    }
    totals
}

/// Observa `crates/` e `android/app/src` e reimplanta o app a cada alteração.
pub fn watch_and_redeploy(
    root_dir: &Path,
    config: &JirauConfig,
    dev: &AndroidDevice,
) -> Result<()> {
    ui::print_info(&t!("android_pipeline.watching"));

    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(&root_dir.join("crates"), RecursiveMode::Recursive)?;
    watcher.watch(&root_dir.join("android/app/src"), RecursiveMode::Recursive)?;

    let mut last_build = Instant::now();
    loop {
        let event = match rx.recv() {
            Ok(Ok(event)) => event,
            Ok(Err(e)) => {
                ui::print_warning(&t!(
                    "android_pipeline.watcher_error",
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
            // jniLibs é saída do próprio build.
            !p.components().any(|c| c.as_os_str() == "jniLibs")
                && p.extension()
                    .is_some_and(|ext| ext == "rs" || ext == "kt" || ext == "toml" || ext == "xml")
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
                "android_pipeline.change_detected",
                paths = changed.join(", ").yellow()
            )
        );

        let start = Instant::now();
        match deploy_to_device(root_dir, config, dev) {
            Ok(()) => ui::print_success(&t!(
                "android_pipeline.app_updated",
                secs = format!("{:.1}", start.elapsed().as_secs_f32())
            )),
            Err(e) => ui::print_error(&format!("{:#}\n", e)),
        }
        while rx.try_recv().is_ok() {}
        last_build = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_tests_from_gradle_junit_reports() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("TEST-emulator.xml"),
            r#"<?xml version="1.0"?><testsuite name="x.NativeLifecycleTest" tests="6" failures="1" errors="0" skipped="0"><testcase name="a"/></testsuite>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.txt"), "tests=\"99\"").unwrap();
        assert_eq!(count_junit_results(dir.path()), (6, 1));
        assert_eq!(count_junit_results(&dir.path().join("missing")), (0, 0));
    }
}
