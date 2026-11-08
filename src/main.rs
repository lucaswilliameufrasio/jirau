mod bindings;
mod build;
mod cli;
mod config;
mod dev;
mod doctor;
mod scaffold;
mod templates;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, DevArgs, DoctorArgs, NewArgs};
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
        Some(Commands::Upgrade) => handle_upgrade(),
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

    if platforms.is_empty() {
        if args.non_interactive {
            platforms = vec!["android".to_string(), "ios".to_string()];
        } else {
            let options = vec!["android", "ios"];
            let defaults = vec![0, 1];
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

    println!();
    ui::print_step("Configuração do projeto:");
    ui::print_substep(&format!("Nome: {}", name.cyan().bold()));
    ui::print_substep(&format!("Package ID: {}", package_id.cyan()));
    ui::print_substep(&format!("Plataformas: {}", platforms.join(", ").cyan()));
    println!();

    let ctx = TemplateContext::new(&name, &package_id, platforms.clone());
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

fn handle_upgrade() -> Result<()> {
    ui::print_banner();
    let (root_dir, config) = JirauConfig::find_in_current_or_parent()?;
    ui::print_step(&format!("Projeto: {}", config.app.name.bold()));
    ui::print_substep(&format!("Localização: {}", root_dir.display()));
    ui::print_substep(&format!(
        "Versão do Template: {}",
        config.meta.template_version.cyan()
    ));
    ui::print_substep(&format!(
        "Versão da CLI jirau: {}",
        env!("CARGO_PKG_VERSION").green().bold()
    ));

    ui::print_success("A estrutura do projeto está alinhada com a versão atual da CLI.");
    Ok(())
}

fn handle_interactive_menu() -> Result<()> {
    ui::print_banner();

    let options = vec![
        "🪵  Criar um novo projeto (new)",
        "🩺  Diagnosticar ambiente e ferramentas (doctor)",
        "🚀  Iniciar ciclo de desenvolvimento (dev)",
        "🔗  Regerar bindings UniFFI (bindings)",
        "📦  Compilar binários para release (build)",
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
                    non_interactive: false,
                })
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
            } else {
                println!("Até logo!");
                Ok(())
            }
        }
        Err(_) => Ok(()),
    }
}
