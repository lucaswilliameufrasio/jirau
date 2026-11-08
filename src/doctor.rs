use colored::*;
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

impl DoctorEngine {
    pub fn run_all() -> DoctorReport {
        let mut items = Vec::new();

        // 1. Rust Ecosystem
        items.extend(Self::check_rust());

        // 2. Android Ecosystem
        items.extend(Self::check_android());

        // 3. iOS Ecosystem
        items.extend(Self::check_ios());

        // 4. Build Acceleration
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
                    title: "Compilador Rust (rustc)".to_string(),
                    status: CheckStatus::Ok,
                    detail: v,
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "Rust".to_string(),
                    title: "Compilador Rust (rustc)".to_string(),
                    status: CheckStatus::Error,
                    detail: "rustc não encontrado no PATH".to_string(),
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
                    title: "Gerenciador de Pacotes Cargo".to_string(),
                    status: CheckStatus::Ok,
                    detail: v,
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "Rust".to_string(),
                    title: "Gerenciador de Pacotes Cargo".to_string(),
                    status: CheckStatus::Error,
                    detail: "cargo não encontrado no PATH".to_string(),
                    fix_hint: Some("Instale o rustup para obter o cargo".to_string()),
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
                title: "Targets Rust Android".to_string(),
                status: CheckStatus::Ok,
                detail: format!("Instalados: {}", required_android_targets.join(", ")),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: "Targets Rust Android".to_string(),
                status: CheckStatus::Error,
                detail: format!("Faltando: {}", missing_android.join(", ")),
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
                title: "Targets Rust iOS".to_string(),
                status: CheckStatus::Ok,
                detail: format!("Instalados: {}", required_ios_targets.join(", ")),
                fix_hint: None,
            });
        } else {
            let is_macos = cfg!(target_os = "macos");
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: "Targets Rust iOS".to_string(),
                status: if is_macos {
                    CheckStatus::Error
                } else {
                    CheckStatus::Warning
                },
                detail: format!("Faltando: {}", missing_ios.join(", ")),
                fix_hint: Some(format!("rustup target add {}", missing_ios.join(" "))),
            });
        }

        // cargo-ndk
        if which::which("cargo-ndk").is_ok() {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: "cargo-ndk (compilação Android)".to_string(),
                status: CheckStatus::Ok,
                detail: "Disponível no PATH".to_string(),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "Rust".to_string(),
                title: "cargo-ndk (compilação Android)".to_string(),
                status: CheckStatus::Error,
                detail: "Utilitário cargo-ndk não encontrado".to_string(),
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
                let first_line = v.lines().next().unwrap_or("Java instalado");
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
                    detail: "JDK não encontrado no PATH".to_string(),
                    fix_hint: Some(
                        "Instale o OpenJDK 17 ou 21 (ex: sudo apt install openjdk-17-jdk)"
                            .to_string(),
                    ),
                });
            }
        }

        // ANDROID_HOME / SDK
        let sdk_dir = env::var("ANDROID_HOME")
            .or_else(|_| env::var("ANDROID_SDK_ROOT"))
            .map(PathBuf::from);

        match sdk_dir {
            Ok(ref p) if p.is_dir() => {
                items.push(DoctorItem {
                    category: "Android".to_string(),
                    title: "Android SDK".to_string(),
                    status: CheckStatus::Ok,
                    detail: format!("Localizado em {}", p.display()),
                    fix_hint: None,
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
                        detail: "NDK localizado".to_string(),
                        fix_hint: None,
                    });
                } else {
                    items.push(DoctorItem {
                        category: "Android".to_string(),
                        title: "Android NDK".to_string(),
                        status: CheckStatus::Error,
                        detail: "NDK não encontrado em $ANDROID_HOME/ndk".to_string(),
                        fix_hint: Some("Instale o NDK via Android Studio SDK Manager ou 'sdkmanager \"ndk;26.1.10909125\"'".to_string()),
                    });
                }
            }
            _ => {
                items.push(DoctorItem {
                    category: "Android".to_string(),
                    title: "Android SDK".to_string(),
                    status: CheckStatus::Error,
                    detail: "Variável $ANDROID_HOME ou $ANDROID_SDK_ROOT não definida ou diretório não existe".to_string(),
                    fix_hint: Some("Defina export ANDROID_HOME=$HOME/Android/Sdk no seu ~/.bashrc ou ~/.zshrc".to_string()),
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
                "adb disponível (nenhum dispositivo conectado)".to_string()
            } else {
                format!(
                    "adb disponível ({} dispositivo(s): {})",
                    dev_list.len(),
                    dev_list.join(", ")
                )
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
                detail: "adb não encontrado no PATH".to_string(),
                fix_hint: Some("Adicione $ANDROID_HOME/platform-tools ao seu PATH".to_string()),
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
                title: "Ambiente iOS (macOS)".to_string(),
                status: CheckStatus::Warning,
                detail: "Executando em Linux. Projetos iOS com XcodeGen podem ser gerados, mas a compilação final requer macOS + Xcode.".to_string(),
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
                    detail: format!("Configurado em {}", path),
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "iOS".to_string(),
                    title: "Xcode & Command Line Tools".to_string(),
                    status: CheckStatus::Error,
                    detail: "Xcode não selecionado".to_string(),
                    fix_hint: Some(
                        "xcode-select --install ou instale o Xcode completo pela App Store"
                            .to_string(),
                    ),
                });
            }
        }

        // XcodeGen
        if which::which("xcodegen").is_ok() {
            items.push(DoctorItem {
                category: "iOS".to_string(),
                title: "XcodeGen".to_string(),
                status: CheckStatus::Ok,
                detail: "Disponível no PATH".to_string(),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "iOS".to_string(),
                title: "XcodeGen".to_string(),
                status: CheckStatus::Warning,
                detail:
                    "XcodeGen não encontrado. Necessário para gerar o .xcodeproj sem conflitos."
                        .to_string(),
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
                    title: "Simulador iOS (simctl)".to_string(),
                    status: CheckStatus::Ok,
                    detail: format!(
                        "simctl operacional ({} simulador(es) em execução)",
                        booted_count
                    ),
                    fix_hint: None,
                });
            }
            _ => {
                items.push(DoctorItem {
                    category: "iOS".to_string(),
                    title: "Simulador iOS (simctl)".to_string(),
                    status: CheckStatus::Warning,
                    detail: "Não foi possível listar simuladores via xcrun simctl".to_string(),
                    fix_hint: None,
                });
            }
        }

        items
    }

    fn check_tooling() -> Vec<DoctorItem> {
        let mut items = Vec::new();

        // sccache
        if which::which("sccache").is_ok() {
            items.push(DoctorItem {
                category: "Otimização".to_string(),
                title: "sccache (Cache de Compilação C/Rust)".to_string(),
                status: CheckStatus::Ok,
                detail: "Disponível no PATH (acelera compilações repetidas)".to_string(),
                fix_hint: None,
            });
        } else {
            items.push(DoctorItem {
                category: "Otimização".to_string(),
                title: "sccache (Cache de Compilação C/Rust)".to_string(),
                status: CheckStatus::Warning,
                detail: "Não instalado (opcional, porém altamente recomendado)".to_string(),
                fix_hint: Some("cargo install sccache".to_string()),
            });
        }

        items
    }

    pub fn print_report(report: &DoctorReport) {
        println!("{}", "Diagnóstico do Ambiente jirau doctor".bold());
        println!(
            "{}",
            "══════════════════════════════════════════════════════════".cyan()
        );

        let mut current_cat = "";
        for item in &report.items {
            if item.category != current_cat {
                println!(
                    "\n{}",
                    format!("── {} ──────────────────────────────", item.category).bright_black()
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
                        println!("    {} {}", "Sugestão:".cyan(), fix.bold());
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
                            "    {} Execute: {}",
                            "Solução:".cyan().bold(),
                            fix.green().bold()
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
            println!(
                "{} Tudo pronto! Seu ambiente possui todas as ferramentas necessárias.",
                "✔".green().bold()
            );
        } else if report.error_count == 0 {
            println!(
                "{} Ambiente pronto para a maioria dos fluxos, com {} aviso(s) opcionais.",
                "!".yellow().bold(),
                report.warning_count
            );
        } else {
            println!(
                "{} Foram identificadas {} pendência(s) obrigatória(s). Execute os comandos sugeridos acima para resolver.",
                "✖".red().bold(),
                report.error_count
            );
        }
        println!();
    }
}
