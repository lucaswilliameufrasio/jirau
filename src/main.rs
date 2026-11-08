mod bindings;
mod build;
mod cli;
mod config;
mod dev;
mod doctor;
mod scaffold;
mod templates;
mod ui;
pub mod upgrade;

use anyhow::Result;
use clap::Parser;
use cli::{AddArgs, Cli, Commands, DevArgs, DoctorArgs, NewArgs, UpgradeArgs};
use colored::*;
use config::JirauConfig;
use doctor::DoctorEngine;
use scaffold::Scaffolder;
use std::env;
use std::process::Command;
use templates::TemplateContext;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::New(args)) => handle_new(args),
        Some(Commands::Doctor(args)) => handle_doctor(args),
        Some(Commands::Dev(args)) => handle_dev(args),
        Some(Commands::Bindings(args)) => handle_bindings(args.platform),
        Some(Commands::Build(args)) => handle_build(args.platform, args.release),
        Some(Commands::Add(args)) => handle_add(args),
        Some(Commands::Upgrade(args)) => upgrade::run_upgrade(&args),
        None => handle_interactive_menu(),
    }
}

fn handle_new(args: NewArgs) -> Result<()> {
    ui::print_banner();

    let name = match args.name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ if args.non_interactive => {
            anyhow::bail!(
                "O nome do projeto é obrigatório em modo não interativo: jirau new <nome>"
            );
        }
        _ => {
            let validator = |input: &str| {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    Ok(inquire::validator::Validation::Invalid(
                        "O nome do projeto não pode ser vazio".into(),
                    ))
                } else if !trimmed
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    Ok(inquire::validator::Validation::Invalid(
                        "Use apenas letras, números, hífens ou underscores".into(),
                    ))
                } else {
                    Ok(inquire::validator::Validation::Valid)
                }
            };

            inquire::Text::new("Nome do aplicativo:")
                .with_placeholder("meu-app")
                .with_help_message("Identificador do projeto e diretório que será criado")
                .with_validator(validator)
                .prompt()?
        }
    };

    let slug = name.to_lowercase().replace(['_', ' '], "-");
    let default_package = format!("com.example.{}", slug.replace('-', ""));

    let package_id = match args.package {
        Some(pkg) if !pkg.trim().is_empty() => pkg.trim().to_string(),
        _ if args.non_interactive => default_package,
        _ => inquire::Text::new("Package ID / Bundle Identifier:")
            .with_default(&default_package)
            .with_help_message("Identificador reverso de domínio para Android e iOS")
            .prompt()?,
    };

    let mut platforms = Vec::new();
    if args.android {
        platforms.push("android".to_string());
    }
    if args.ios {
        platforms.push("ios".to_string());
    }
    if args.desktop {
        platforms.push("desktop".to_string());
    }

    if platforms.is_empty() {
        if args.non_interactive {
            platforms = vec![
                "android".to_string(),
                "ios".to_string(),
                "desktop".to_string(),
            ];
        } else {
            let options = vec!["android", "ios", "desktop"];
            let defaults = vec![0, 1, 2];
            let selected = inquire::MultiSelect::new("Plataformas suportadas:", options)
                .with_default(&defaults)
                .with_help_message("Pressione Espaço para marcar/desmarcar e Enter para confirmar")
                .prompt()?;

            if selected.is_empty() {
                anyhow::bail!("Pelo menos uma plataforma deve ser selecionada.");
            }
            platforms = selected.into_iter().map(String::from).collect();
        }
    }

    // Persistência local (SQLite / InMemory)
    let storage = match args.storage {
        Some(s) => s.to_lowercase(),
        None if args.sqlite => "sqlite".to_string(),
        None if args.non_interactive => "none".to_string(),
        None => {
            let options = vec![
                "Nenhuma (Estado puramente em memória)",
                "SQLite embutido (rusqlite no core Rust)",
            ];
            let choice = inquire::Select::new("Camada de persistência local:", options)
                .with_help_message("Escolha a camada de armazenamento de dados no core Rust")
                .prompt()?;
            if choice.contains("SQLite") {
                "sqlite".to_string()
            } else {
                "none".to_string()
            }
        }
    };

    // Sistema de build Android (Gradle / Amper)
    let android_build_system = match args.android_build_system {
        Some(s) => s.to_lowercase(),
        None if args.amper => "amper".to_string(),
        None if args.non_interactive => "gradle".to_string(),
        None if platforms.contains(&"android".to_string()) => {
            let options = vec![
                "Gradle (Padrão do ecossistema Android com Kotlin DSL)",
                "Amper (Configuração declarativa moderna da JetBrains)",
            ];
            let choice = inquire::Select::new("Sistema de build Android:", options)
                .with_help_message("Escolha entre o Gradle tradicional ou o JetBrains Amper")
                .prompt()?;
            if choice.contains("Amper") {
                "amper".to_string()
            } else {
                "gradle".to_string()
            }
        }
        None => "gradle".to_string(),
    };

    println!();
    ui::print_step("Configuração do projeto:");
    ui::print_substep(&format!("Nome: {}", name.cyan().bold()));
    ui::print_substep(&format!("Package ID: {}", package_id.cyan()));
    ui::print_substep(&format!("Plataformas: {}", platforms.join(", ").cyan()));
    ui::print_substep(&format!("Persistência: {}", storage.cyan()));
    if platforms.contains(&"android".to_string()) {
        ui::print_substep(&format!("Build Android: {}", android_build_system.cyan()));
    }
    println!();

    let ctx = TemplateContext::with_options(
        &name,
        &package_id,
        platforms.clone(),
        &storage,
        &android_build_system,
        upgrade::CURRENT_TEMPLATE_VERSION,
    );
    let current_dir = env::current_dir()?;
    Scaffolder::create_project(&ctx, &current_dir)?;

    println!();
    println!("{}", "🎉 Projeto configurado com sucesso!".green().bold());
    println!(
        "{}",
        "══════════════════════════════════════════════════════════".cyan()
    );
    println!("Para começar:");
    println!("  {} {}", "cd".cyan(), ctx.slug.bold());
    println!(
        "  {}        # Valida seu ambiente local",
        "jirau doctor".cyan()
    );
    if platforms.contains(&"android".to_string()) {
        println!(
            "  {}    # Inicia o desenvolvimento Android",
            "jirau dev android".cyan()
        );
    }
    if platforms.contains(&"ios".to_string()) {
        println!(
            "  {}        # Inicia o desenvolvimento iOS",
            "jirau dev ios".cyan()
        );
    }
    if platforms.contains(&"desktop".to_string()) {
        println!(
            "  {}    # Inicia o app Desktop (Rune UI)",
            "jirau dev desktop".cyan()
        );
    }
    println!(
        "  {}      # Roda testes unitários do core Rust no host",
        "cargo test".cyan()
    );
    println!(
        "{}",
        "══════════════════════════════════════════════════════════".cyan()
    );
    println!();

    Ok(())
}

fn handle_doctor(args: DoctorArgs) -> Result<()> {
    if !args.json {
        ui::print_banner();
    }

    let report = DoctorEngine::run_all();

    if args.json {
        let json = serde_json::to_string_pretty(&report)?;
        println!("{}", json);
        return Ok(());
    }

    DoctorEngine::print_report(&report);

    if args.profile {
        ui::print_step("Executando profiling de build do Gradle (--profile)...");
        let (root, _cfg) = JirauConfig::find_in_current_or_parent()?;
        let android_dir = root.join("android");

        if android_dir.exists() {
            let gradlew = if cfg!(windows) {
                android_dir.join("gradlew.bat")
            } else {
                android_dir.join("gradlew")
            };

            if gradlew.exists() {
                let status = Command::new(&gradlew)
                    .current_dir(&android_dir)
                    .args(["assembleDebug", "--profile"])
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        ui::print_success("Relatório gerado em: android/build/reports/profile/");
                    }
                    _ => {
                        ui::print_warning("Falha ao rodar profiling do Gradle.");
                    }
                }
            } else {
                ui::print_warning("gradlew não encontrado na pasta android/.");
            }
        } else {
            ui::print_warning("Pasta android/ não encontrada para profiling.");
        }
    }

    Ok(())
}

fn handle_dev(args: DevArgs) -> Result<()> {
    let (root_dir, config) = JirauConfig::find_in_current_or_parent()?;
    dev::DevRunner::run(&root_dir, &config, args.platform, !args.no_watch)
}

fn handle_bindings(platform: Option<String>) -> Result<()> {
    let (root_dir, config) = JirauConfig::find_in_current_or_parent()?;
    bindings::BindingsGenerator::generate(&root_dir, &config, platform)
}

fn handle_build(platform: Option<String>, release: bool) -> Result<()> {
    let (root_dir, config) = JirauConfig::find_in_current_or_parent()?;
    build::ReleaseBuilder::build(&root_dir, &config, platform, release)
}

fn handle_add(args: AddArgs) -> Result<()> {
    ui::print_banner();
    let (root_dir, mut config) = JirauConfig::find_in_current_or_parent()?;

    let platform = match args.platform {
        Some(p) => p.to_lowercase(),
        None => {
            let options = vec!["desktop", "android", "ios"];
            inquire::Select::new("Qual plataforma deseja adicionar?", options)
                .with_help_message("Escolha com as setas e pressione Enter")
                .prompt()?
                .to_string()
        }
    };

    if config.app.platforms.contains(&platform) {
        ui::print_warning(&format!(
            "A plataforma '{}' já faz parte deste projeto.",
            platform
        ));
        return Ok(());
    }

    let ctx = TemplateContext::with_options(
        &config.app.name,
        &config.app.package_id,
        vec![platform.clone()],
        &config.app.storage,
        &config.app.android_build_system,
        &config.meta.template_version,
    );
    let spinner = ui::create_spinner("Configurando nova plataforma...");

    match platform.as_str() {
        "desktop" => {
            let desktop_dir = root_dir
                .join("crates")
                .join(format!("{}-desktop", ctx.slug));
            std::fs::create_dir_all(desktop_dir.join("src"))?;
            std::fs::write(
                desktop_dir.join("Cargo.toml"),
                templates::desktop::desktop_cargo_toml(&ctx),
            )?;
            std::fs::write(
                desktop_dir.join("src/main.rs"),
                templates::desktop::desktop_main_rs(&ctx),
            )?;

            // Atualiza workspace members no Cargo.toml raiz
            let cargo_toml_path = root_dir.join("Cargo.toml");
            let cargo_content = std::fs::read_to_string(&cargo_toml_path)?;
            let desktop_member = format!("\"crates/{}-desktop\"", ctx.slug);
            if !cargo_content.contains(&desktop_member) {
                let updated = cargo_content.replace(
                    "members = [",
                    &format!("members = [\n    {},", desktop_member),
                );
                std::fs::write(&cargo_toml_path, updated)?;
            }
        }
        "android" => {
            let android_dir = root_dir.join("android");
            let app_dir = android_dir.join("app");
            let pkg_src_dir = app_dir.join("src/main/java").join(&ctx.package_path);
            let jni_libs_dir = app_dir.join("src/main/jniLibs");
            let gradle_toml_dir = android_dir.join("gradle");

            std::fs::create_dir_all(&pkg_src_dir)?;
            std::fs::create_dir_all(&jni_libs_dir)?;
            std::fs::create_dir_all(&gradle_toml_dir)?;

            std::fs::write(
                android_dir.join("settings.gradle.kts"),
                templates::android::settings_gradle_kts(&ctx),
            )?;
            std::fs::write(
                android_dir.join("gradle.properties"),
                templates::android::gradle_properties(),
            )?;
            std::fs::write(
                android_dir.join("build.gradle.kts"),
                templates::android::root_build_gradle_kts(),
            )?;
            std::fs::write(
                gradle_toml_dir.join("libs.versions.toml"),
                templates::android::libs_versions_toml(),
            )?;
            std::fs::write(
                app_dir.join("build.gradle.kts"),
                templates::android::app_build_gradle_kts(&ctx),
            )?;
            std::fs::write(
                app_dir.join("src/main/AndroidManifest.xml"),
                templates::android::app_android_manifest_xml(&ctx),
            )?;
            std::fs::write(
                pkg_src_dir.join("MainActivity.kt"),
                templates::android::app_main_activity_kt(&ctx),
            )?;

            if config.app.android_build_system == "amper" {
                std::fs::write(
                    android_dir.join("module.yaml"),
                    templates::android::amper_module_yaml(&ctx),
                )?;
                std::fs::write(
                    android_dir.join("project.yaml"),
                    templates::android::amper_project_yaml(),
                )?;
                let amper_script = android_dir.join("amper");
                std::fs::write(&amper_script, templates::android::amper_wrapper_sh())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        &amper_script,
                        std::fs::Permissions::from_mode(0o755),
                    );
                }
            }
        }
        "ios" => {
            let ios_dir = root_dir.join("ios");
            let sources_dir = ios_dir.join("App/Sources");
            let gen_dir = ios_dir.join("Generated");

            std::fs::create_dir_all(&sources_dir)?;
            std::fs::create_dir_all(&gen_dir)?;

            std::fs::write(
                ios_dir.join("project.yml"),
                templates::ios::project_yml(&ctx),
            )?;
            std::fs::write(
                sources_dir.join("ContentView.swift"),
                templates::ios::content_view_swift(&ctx),
            )?;
            std::fs::write(
                sources_dir.join(format!("{}App.swift", ctx.pascal_name)),
                templates::ios::app_swift(&ctx),
            )?;
        }
        other => anyhow::bail!(
            "Plataforma '{}' não suportada. Use 'desktop', 'android' ou 'ios'.",
            other
        ),
    }

    config.app.platforms.push(platform.clone());
    config.write_to_dir(&root_dir)?;

    ui::finish_spinner_success(
        &spinner,
        &format!("Plataforma '{}' adicionada com sucesso!", platform),
    );
    println!();
    ui::print_info(&format!(
        "Execute 'jirau dev {}' para testar a nova plataforma.",
        platform
    ));
    println!();

    Ok(())
}

fn handle_interactive_menu() -> Result<()> {
    ui::print_banner();

    let options = vec![
        "🪵  Criar um novo projeto (new)",
        "➕  Adicionar plataforma a um projeto existente (add)",
        "🩺  Diagnosticar ambiente e ferramentas (doctor)",
        "🚀  Iniciar ciclo de desenvolvimento (dev)",
        "🔗  Regerar bindings UniFFI (bindings)",
        "📦  Compilar binários para release (build)",
        "🔄  Sincronizar template via 3-way merge (upgrade)",
        "🚪  Sair",
    ];

    let choice = inquire::Select::new("O que você deseja fazer?", options)
        .with_help_message("Use as setas ↑/↓ e pressione Enter")
        .prompt();

    match choice {
        Ok(selected) => {
            if selected.contains("Criar um novo projeto") {
                handle_new(NewArgs {
                    name: None,
                    package: None,
                    android: false,
                    ios: false,
                    desktop: false,
                    storage: None,
                    sqlite: false,
                    android_build_system: None,
                    amper: false,
                    non_interactive: false,
                })
            } else if selected.contains("Adicionar plataforma") {
                handle_add(AddArgs { platform: None })
            } else if selected.contains("Diagnosticar ambiente") {
                handle_doctor(DoctorArgs {
                    json: false,
                    profile: false,
                })
            } else if selected.contains("Iniciar ciclo de desenvolvimento") {
                handle_dev(DevArgs {
                    platform: None,
                    no_watch: false,
                })
            } else if selected.contains("Regerar bindings") {
                handle_bindings(None)
            } else if selected.contains("Compilar binários") {
                handle_build(None, true)
            } else if selected.contains("Sincronizar template") {
                upgrade::run_upgrade(&UpgradeArgs::default())
            } else {
                println!("Até logo!");
                Ok(())
            }
        }
        Err(_) => Ok(()),
    }
}
