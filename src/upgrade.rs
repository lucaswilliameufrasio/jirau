use crate::cli::UpgradeArgs;
use crate::config::{JirauConfig, CONFIG_FILE_NAME};
use crate::scaffold::Scaffolder;
use crate::templates::TemplateContext;
use crate::ui;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use walkdir::WalkDir;

pub const CURRENT_TEMPLATE_VERSION: &str = "0.2.0";

#[derive(Debug, PartialEq, Eq)]
pub enum MergeResult {
    Unchanged,
    Created,
    Updated,
    Preserved,
    MergedCleanly,
    Conflict,
}

pub fn run_upgrade(args: &UpgradeArgs) -> Result<()> {
    ui::print_banner();

    let (project_root, mut config) = JirauConfig::find_in_current_or_parent()?;
    let prev_version = config.meta.template_version.clone();

    ui::print_step(&format!("Projeto: {}", config.app.name.bold()));
    ui::print_substep(&format!("Localização: {}", project_root.display()));
    ui::print_substep(&format!(
        "Versão atual do template no projeto: {}",
        prev_version.cyan()
    ));
    ui::print_substep(&format!(
        "Versão do template no jirau CLI: {}",
        CURRENT_TEMPLATE_VERSION.green().bold()
    ));

    if prev_version == CURRENT_TEMPLATE_VERSION && !args.force {
        println!();
        ui::print_success(&format!(
            "O projeto já está sincronizado com a versão mais recente do template (v{}).",
            CURRENT_TEMPLATE_VERSION
        ));
        ui::print_info(
            "Para forçar uma re-sincronização via 3-way merge, execute: jirau upgrade --force",
        );
        return Ok(());
    }

    if args.dry_run {
        ui::print_warning("MODO DRY-RUN ATIVADO: nenhuma alteração será salva no disco.");
    }

    let spinner = ui::create_spinner("Gerando snapshots de template para 3-way merge...");

    let temp_base = tempdir()?;
    let temp_target = tempdir()?;

    let base_ctx = TemplateContext::with_options(
        &config.app.name,
        &config.app.package_id,
        config.app.platforms.clone(),
        &config.app.storage,
        &config.app.android_build_system,
        &prev_version,
    );

    let target_ctx = TemplateContext::with_options(
        &config.app.name,
        &config.app.package_id,
        config.app.platforms.clone(),
        &config.app.storage,
        &config.app.android_build_system,
        CURRENT_TEMPLATE_VERSION,
    );

    Scaffolder::create_project(&base_ctx, temp_base.path())
        .context("Falha ao gerar template base")?;
    Scaffolder::create_project(&target_ctx, temp_target.path())
        .context("Falha ao gerar template alvo")?;

    ui::finish_spinner_success(&spinner, "Snapshots de template gerados com sucesso.");

    let base_dir = temp_base.path().join(&base_ctx.slug);
    let target_dir = temp_target.path().join(&target_ctx.slug);

    let mut created_count = 0;
    let mut updated_count = 0;
    let mut merged_count = 0;
    let mut preserved_count = 0;
    let mut conflict_count = 0;

    println!();
    ui::print_step("Comparando e sincronizando arquivos...");

    for entry in WalkDir::new(&target_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            continue;
        }

        let target_file = entry.path();
        let rel_path = target_file
            .strip_prefix(&target_dir)
            .context("Falha ao calcular caminho relativo")?;

        // Ignora jirau.toml na checagem bruta pois ele será atualizado separadamente
        if rel_path == Path::new(CONFIG_FILE_NAME) {
            continue;
        }

        let current_file = project_root.join(rel_path);
        let base_file = base_dir.join(rel_path);

        let result = process_file_merge(
            &base_file,
            target_file,
            &current_file,
            rel_path,
            args.dry_run,
        )?;

        match result {
            MergeResult::Created => {
                created_count += 1;
                ui::print_substep(&format!(
                    "✨ {} {}",
                    "[NOVO]".green().bold(),
                    rel_path.display()
                ));
            }
            MergeResult::Updated => {
                updated_count += 1;
                ui::print_substep(&format!(
                    "⬆  {} {}",
                    "[ATUALIZADO]".cyan().bold(),
                    rel_path.display()
                ));
            }
            MergeResult::MergedCleanly => {
                merged_count += 1;
                ui::print_substep(&format!(
                    "🔀 {} {}",
                    "[MESCLADO 3-WAY]".blue().bold(),
                    rel_path.display()
                ));
            }
            MergeResult::Conflict => {
                conflict_count += 1;
                ui::print_substep(&format!(
                    "⚠️  {} {}",
                    "[CONFLITO 3-WAY]".yellow().bold(),
                    rel_path.display()
                ));
            }
            MergeResult::Preserved => {
                preserved_count += 1;
            }
            MergeResult::Unchanged => {}
        }
    }

    // Atualiza o template_version no jirau.toml
    if !args.dry_run {
        config.meta.template_version = CURRENT_TEMPLATE_VERSION.to_string();
        config.write_to_dir(&project_root)?;
    }

    println!();
    ui::print_step("Resumo do Upgrade:");
    println!(
        "  • Arquivos criados: {}",
        created_count.to_string().green()
    );
    println!(
        "  • Arquivos atualizados: {}",
        updated_count.to_string().cyan()
    );
    println!(
        "  • Arquivos mesclados automaticamente: {}",
        merged_count.to_string().blue()
    );
    println!(
        "  • Arquivos preservados: {}",
        preserved_count.to_string().white()
    );

    if conflict_count > 0 {
        println!(
            "  • Conflitos para resolver: {}",
            conflict_count.to_string().yellow().bold()
        );
        println!();
        ui::print_warning(&format!(
            "Foram encontrados {} conflito(s). Os arquivos contêm marcadores '<<<<<<<' e '>>>>>>>'.\nRevise-os e remova os marcadores antes de compilar.",
            conflict_count
        ));
    } else {
        println!();
        ui::print_success(&format!(
            "Projeto atualizado com sucesso para o template v{}!",
            CURRENT_TEMPLATE_VERSION
        ));
    }

    Ok(())
}

fn process_file_merge(
    base_file: &Path,
    target_file: &Path,
    current_file: &Path,
    _rel_path: &Path,
    dry_run: bool,
) -> Result<MergeResult> {
    let target_content = fs::read_to_string(target_file)?;

    if !current_file.exists() {
        if !dry_run {
            if let Some(parent) = current_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(current_file, &target_content)?;
        }
        return Ok(MergeResult::Created);
    }

    let current_content = fs::read_to_string(current_file)?;

    if current_content == target_content {
        return Ok(MergeResult::Unchanged);
    }

    let base_content = if base_file.exists() {
        fs::read_to_string(base_file).ok()
    } else {
        None
    };

    match base_content {
        Some(base) if base == current_content => {
            // O usuário não mexeu no arquivo e o template evoluiu -> atualiza limpo
            if !dry_run {
                fs::write(current_file, &target_content)?;
            }
            Ok(MergeResult::Updated)
        }
        Some(base) if base == target_content => {
            // O template não mudou esse arquivo; mantem a versão customizada do usuário
            Ok(MergeResult::Preserved)
        }
        Some(base) => {
            // Ambos alteraram o arquivo! Executa o 3-way merge
            let (merged_text, has_conflict) = execute_three_way_merge(
                &current_content,
                &base,
                &target_content,
                base_file,
                target_file,
                current_file,
            );

            if !dry_run {
                fs::write(current_file, merged_text)?;
            }

            if has_conflict {
                Ok(MergeResult::Conflict)
            } else {
                Ok(MergeResult::MergedCleanly)
            }
        }
        None => {
            // Arquivo não existia no base anterior; mantem o do usuário
            Ok(MergeResult::Preserved)
        }
    }
}

fn execute_three_way_merge(
    current: &str,
    base: &str,
    target: &str,
    base_path: &Path,
    target_path: &Path,
    current_path: &Path,
) -> (String, bool) {
    // Tenta usar `git merge-file` nativo
    let git_merge = Command::new("git")
        .args([
            "merge-file",
            "-p",
            "-L",
            "SEU CÓDIGO",
            "-L",
            "BASE",
            "-L",
            "NOVO TEMPLATE",
            current_path.to_str().unwrap_or(""),
            base_path.to_str().unwrap_or(""),
            target_path.to_str().unwrap_or(""),
        ])
        .output();

    if let Ok(output) = git_merge {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let conflict = !output.status.success();
        return (stdout, conflict);
    }

    // Fallback puro em Rust caso `git` não esteja disponível
    fallback_three_way_merge(current, base, target)
}

fn fallback_three_way_merge(current: &str, base: &str, target: &str) -> (String, bool) {
    if current == base {
        return (target.to_string(), false);
    }
    if target == base {
        return (current.to_string(), false);
    }
    if current == target {
        return (current.to_string(), false);
    }

    // Marcadores de conflito
    let conflict_text = format!(
        "<<<<<<< SEU CÓDIGO\n{}\n=======\n{}\n>>>>>>> NOVO TEMPLATE (v{})\n",
        current.trim_end(),
        target.trim_end(),
        CURRENT_TEMPLATE_VERSION
    );
    (conflict_text, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_merge_clean() {
        let base = "linha1\nlinha2\n";
        let target = "linha1\nlinha2\nlinha3\n";
        let current = "linha1\nlinha2\n"; // user didn't change
        let (merged, conflict) = fallback_three_way_merge(current, base, target);
        assert!(!conflict);
        assert_eq!(merged, target);
    }

    #[test]
    fn test_fallback_merge_conflict() {
        let base = "linha1\n";
        let current = "linha1 modificada pelo usuario\n";
        let target = "linha1 modificada pelo template\n";
        let (merged, conflict) = fallback_three_way_merge(current, base, target);
        assert!(conflict);
        assert!(merged.contains("<<<<<<< SEU CÓDIGO"));
        assert!(merged.contains(">>>>>>> NOVO TEMPLATE"));
    }
}
