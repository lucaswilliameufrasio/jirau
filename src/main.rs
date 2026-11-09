mod android_pipeline;
mod bindings;
mod build;
mod cli;
mod config;
mod dev;
mod doctor;
mod i18n;
mod ios_pipeline;
mod process;
mod run;
mod scaffold;
mod templates;
mod test_runner;
mod ui;
pub mod upgrade;

use anyhow::Result;
use cli::{AddArgs, Commands, DevArgs, DoctorArgs, NewArgs, RunArgs, TestArgs, UpgradeArgs};
use colored::*;
use config::JirauConfig;
use doctor::DoctorEngine;
use rust_i18n::t;
use scaffold::{ProjectWriter, Scaffolder};
use std::env;
use std::process::Command;
use templates::TemplateContext;

// Traduções em locales/*.yml (en + pt-BR lado a lado); `en` é o fallback.
rust_i18n::i18n!("locales", fallback = "en");

fn main() -> Result<()> {
    i18n::init_from_args(std::env::args());
    let cli = i18n::parse_localized_cli()?;

    match cli.command {
        Some(Commands::New(args)) => handle_new(args),
        Some(Commands::Run(args)) => handle_run(args),
        Some(Commands::Test(args)) => handle_test(args),
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
            anyhow::bail!("{}", t!("main.name_required_non_interactive"));
        }
        _ => {
            let validator = |input: &str| {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    Ok(inquire::validator::Validation::Invalid(
                        t!("main.name_empty").into_owned().into(),
                    ))
                } else if !trimmed
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    Ok(inquire::validator::Validation::Invalid(
                        t!("main.name_invalid_chars").into_owned().into(),
                    ))
                } else {
                    Ok(inquire::validator::Validation::Valid)
                }
            };

            inquire::Text::new(&t!("main.prompt_app_name"))
                .with_placeholder(&t!("main.prompt_app_name_placeholder"))
                .with_help_message(&t!("main.prompt_app_name_help"))
                .with_validator(validator)
                .prompt()?
        }
    };

    let slug = name.to_lowercase().replace(['_', ' '], "-");
    let default_package = format!("com.example.{}", slug.replace('-', ""));

    let package_id = match args.package {
        Some(pkg) if !pkg.trim().is_empty() => pkg.trim().to_string(),
        _ if args.non_interactive => default_package,
        _ => inquire::Text::new(&t!("main.prompt_package"))
            .with_default(&default_package)
            .with_help_message(&t!("main.prompt_package_help"))
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
            let selected = inquire::MultiSelect::new(&t!("main.prompt_platforms"), options)
                .with_default(&defaults)
                .with_help_message(&t!("main.prompt_platforms_help"))
                .prompt()?;

            if selected.is_empty() {
                anyhow::bail!("{}", t!("main.platform_required"));
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
            // Cada opção traz o valor interno ao lado do texto traduzido.
            let choices = [
                ("none", t!("main.storage_option_none")),
                ("sqlite", t!("main.storage_option_sqlite")),
            ];
            let labels: Vec<&str> = choices.iter().map(|(_, label)| label.as_ref()).collect();
            let choice = inquire::Select::new(&t!("main.prompt_storage"), labels)
                .with_help_message(&t!("main.prompt_storage_help"))
                .raw_prompt()?;
            choices[choice.index].0.to_string()
        }
    };

    // Sistema de build Android: Gradle (Kotlin DSL + version catalog).
    // O JetBrains Amper saiu do jirau: o modo plugin do Gradle foi descontinuado
    // e o Amper standalone ainda não tem versão estável (1.0).
    let requested_build_system = match args.android_build_system {
        Some(s) => s.to_lowercase(),
        None if args.amper => "amper".to_string(),
        None => "gradle".to_string(),
    };
    if requested_build_system != "gradle" {
        anyhow::bail!(
            "{}",
            t!(
                "main.build_system_unsupported",
                name = requested_build_system
            )
        );
    }
    let android_build_system = requested_build_system;

    // Tema de UI
    let ui_theme = match args.theme {
        Some(t) => t.to_lowercase(),
        None if args.non_interactive => "default".to_string(),
        None => {
            let choices = [
                ("default", t!("main.theme_option_default")),
                (
                    "material3-extended",
                    t!("main.theme_option_material3_extended"),
                ),
                ("minimal", t!("main.theme_option_minimal")),
            ];
            let labels: Vec<&str> = choices.iter().map(|(_, label)| label.as_ref()).collect();
            let choice = inquire::Select::new(&t!("main.prompt_theme"), labels)
                .with_help_message(&t!("main.prompt_theme_help"))
                .raw_prompt()?;
            choices[choice.index].0.to_string()
        }
    };
    let plugins = args.plugin;

    // Template da Aplicação
    let app_template = match args.template {
        Some(t) => t.to_lowercase(),
        None if args.non_interactive => "starter".to_string(),
        None => {
            let choices = [
                ("starter", t!("main.template_option_starter")),
                ("vault", t!("main.template_option_vault")),
            ];
            let labels: Vec<&str> = choices.iter().map(|(_, label)| label.as_ref()).collect();
            let choice = inquire::Select::new(&t!("main.prompt_template"), labels)
                .with_help_message(&t!("main.prompt_template_help"))
                .raw_prompt()?;
            choices[choice.index].0.to_string()
        }
    };

    let storage = if app_template == "vault" && storage == "none" {
        "sqlite".to_string()
    } else {
        storage
    };

    println!();
    ui::print_step(&t!("main.summary_title"));
    ui::print_substep(&t!("main.summary_name", value = name.cyan().bold()));
    ui::print_substep(&t!(
        "main.summary_template",
        value = app_template.cyan().bold()
    ));
    ui::print_substep(&t!("main.summary_package", value = package_id.cyan()));
    ui::print_substep(&t!(
        "main.summary_platforms",
        value = platforms.join(", ").cyan()
    ));
    ui::print_substep(&t!("main.summary_storage", value = storage.cyan()));
    if platforms.contains(&"android".to_string()) {
        ui::print_substep(&t!(
            "main.summary_android_build",
            value = android_build_system.cyan()
        ));
    }
    ui::print_substep(&t!("main.summary_theme", value = ui_theme.cyan()));
    if !plugins.is_empty() {
        ui::print_substep(&t!(
            "main.summary_plugins",
            value = plugins.join(", ").cyan()
        ));
    }
    if args.ci {
        ui::print_substep(&t!(
            "main.summary_ci",
            value = "GitHub Actions (.github/workflows/ci.yml)".cyan()
        ));
    }
    println!();

    let ctx = TemplateContext::with_full_spec(
        &name,
        &package_id,
        platforms.clone(),
        &storage,
        &android_build_system,
        &ui_theme,
        plugins,
        &app_template,
        args.ci,
        upgrade::CURRENT_TEMPLATE_VERSION,
    );
    let current_dir = env::current_dir()?;
    Scaffolder::create_project(&ctx, &current_dir)?;

    println!();
    println!("{}", t!("main.project_ready").green().bold());
    println!(
        "{}",
        "══════════════════════════════════════════════════════════".cyan()
    );
    println!("{}", t!("main.next_steps"));
    println!("  {} {}", "cd".cyan(), ctx.slug.bold());
    println!(
        "  {}        # {}",
        "jirau doctor".cyan(),
        t!("main.next_doctor")
    );
    if platforms.contains(&"android".to_string()) {
        println!(
            "  {}    # {}",
            "jirau dev android".cyan(),
            t!("main.next_dev_android")
        );
    }
    if platforms.contains(&"ios".to_string()) {
        println!(
            "  {}        # {}",
            "jirau dev ios".cyan(),
            t!("main.next_dev_ios")
        );
    }
    if platforms.contains(&"desktop".to_string()) {
        println!(
            "  {}    # {}",
            "jirau dev desktop".cyan(),
            t!("main.next_dev_desktop")
        );
    }
    println!(
        "  {}      # {}",
        "cargo test".cyan(),
        t!("main.next_cargo_test")
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
        ui::print_step(&t!("main.profile_running"));
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
                        ui::print_success(&t!(
                            "main.profile_report",
                            path = "android/build/reports/profile/"
                        ));
                    }
                    _ => {
                        ui::print_warning(&t!("main.profile_failed"));
                    }
                }
            } else {
                ui::print_warning(&t!("main.profile_no_gradlew"));
            }
        } else {
            ui::print_warning(&t!("main.profile_no_android_dir"));
        }
    }

    Ok(())
}

fn handle_dev(args: DevArgs) -> Result<()> {
    let (root_dir, config) = JirauConfig::find_in_current_or_parent()?;
    dev::DevRunner::run(&root_dir, &config, args.platform, !args.no_watch)
}

fn handle_run(args: RunArgs) -> Result<()> {
    let (root_dir, config) = JirauConfig::find_in_current_or_parent()?;
    run::RunEngine::run(
        &root_dir,
        &config,
        args.platform,
        args.device,
        !args.no_watch,
    )
}

fn handle_test(args: TestArgs) -> Result<()> {
    let (root_dir, config) = JirauConfig::find_in_current_or_parent()?;
    test_runner::TestRunner::run(&root_dir, &config, args.target, args.all)
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
            inquire::Select::new(&t!("main.prompt_add_platform"), options)
                .with_help_message(&t!("main.prompt_add_platform_help"))
                .prompt()?
                .to_string()
        }
    };

    if config.app.platforms.contains(&platform) {
        ui::print_warning(&t!("main.add_already_present", platform = platform));
        return Ok(());
    }

    let mut ctx = TemplateContext::with_options(
        &config.app.name,
        &config.app.package_id,
        vec![platform.clone()],
        &config.app.storage,
        &config.app.android_build_system,
        &config.meta.template_version,
    );
    ctx.app_template = config.app.template.clone();
    let spinner = ui::create_spinner(&t!("main.add_configuring"));

    let mut writer = ProjectWriter::new(&root_dir);
    match platform.as_str() {
        "desktop" => {
            Scaffolder::write_desktop(&ctx, &mut writer)?;

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
        "android" | "ios" => {
            if platform == "android" {
                Scaffolder::write_android(&ctx, &mut writer)?;
            } else {
                Scaffolder::write_ios(&ctx, &mut writer)?;
            }
            // Projetos antigos não têm o binário uniffi-bindgen da crate FFI.
            writer.write_if_missing(
                format!("crates/{}-ffi/src/bin/uniffi-bindgen.rs", ctx.slug),
                &templates::rust_ffi::ffi_uniffi_bindgen_rs(),
            )?;
        }
        other => anyhow::bail!("{}", t!("main.add_unsupported", platform = other)),
    }
    // Registra o que foi gerado como base do 3-way merge do `jirau upgrade`.
    writer.save_template_base()?;

    config.app.platforms.push(platform.clone());
    config.write_to_dir(&root_dir)?;

    ui::finish_spinner_success(&spinner, &t!("main.add_success", platform = platform));
    println!();
    ui::print_info(&t!("main.add_next", platform = platform));
    println!();

    Ok(())
}

fn handle_interactive_menu() -> Result<()> {
    ui::print_banner();

    // A ação de cada item do menu é decidida pela posição, não pelo texto
    // (que muda conforme o idioma).
    #[derive(Clone, Copy)]
    enum MenuAction {
        New,
        Run,
        Test,
        Add,
        Doctor,
        Dev,
        Bindings,
        Build,
        Upgrade,
        Exit,
    }
    let entries = [
        (MenuAction::New, t!("main.menu_new")),
        (MenuAction::Run, t!("main.menu_run")),
        (MenuAction::Test, t!("main.menu_test")),
        (MenuAction::Add, t!("main.menu_add")),
        (MenuAction::Doctor, t!("main.menu_doctor")),
        (MenuAction::Dev, t!("main.menu_dev")),
        (MenuAction::Bindings, t!("main.menu_bindings")),
        (MenuAction::Build, t!("main.menu_build")),
        (MenuAction::Upgrade, t!("main.menu_upgrade")),
        (MenuAction::Exit, t!("main.menu_exit")),
    ];
    let labels: Vec<&str> = entries.iter().map(|(_, label)| label.as_ref()).collect();

    let choice = inquire::Select::new(&t!("main.menu_prompt"), labels)
        .with_help_message(&t!("main.menu_help"))
        .raw_prompt();

    match choice {
        Ok(selected) => match entries[selected.index].0 {
            MenuAction::New => handle_new(NewArgs {
                name: None,
                package: None,
                android: false,
                ios: false,
                desktop: false,
                storage: None,
                sqlite: false,
                android_build_system: None,
                amper: false,
                theme: None,
                plugin: Vec::new(),
                template: None,
                ci: false,
                non_interactive: false,
            }),
            MenuAction::Run => handle_run(RunArgs::default()),
            MenuAction::Test => handle_test(TestArgs::default()),
            MenuAction::Add => handle_add(AddArgs { platform: None }),
            MenuAction::Doctor => handle_doctor(DoctorArgs {
                json: false,
                profile: false,
            }),
            MenuAction::Dev => handle_dev(DevArgs {
                platform: None,
                no_watch: false,
            }),
            MenuAction::Bindings => handle_bindings(None),
            MenuAction::Build => handle_build(None, true),
            MenuAction::Upgrade => upgrade::run_upgrade(&UpgradeArgs::default()),
            MenuAction::Exit => {
                println!("{}", t!("main.goodbye"));
                Ok(())
            }
        },
        Err(_) => Ok(()),
    }
}
