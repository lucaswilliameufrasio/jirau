//! `jirau upgrade`: atualiza um projeto para o template atual via 3-way merge.
//!
//! A *base* do merge é `.jirau/template-base/`, a cópia exata do que o template
//! gerou da última vez (gravada por `jirau new`, `jirau add` e pelo próprio
//! upgrade). Com ela dá para separar o que o usuário mudou do que o template
//! mudou, arquivo a arquivo:
//!
//! | atual vs base | alvo vs base | resultado                       |
//! |---------------|--------------|---------------------------------|
//! | igual         | diferente    | atualiza para o alvo            |
//! | diferente     | igual        | preserva a versão do usuário    |
//! | diferente     | diferente    | `git merge-file` (conflito marcado) |
//!
//! Projetos sem base (criados antes dela existir) não têm como saber o que foi
//! customizado: o novo conteúdo vai para `<arquivo>.jirau-new` para revisão.

use crate::cli::UpgradeArgs;
use crate::config::{CONFIG_FILE_NAME, JirauConfig};
use crate::scaffold::{ProjectWriter, Scaffolder, TEMPLATE_BASE_DIR};
use crate::templates::TemplateContext;
use crate::ui;
use anyhow::{Context, Result};
use colored::Colorize;
use rust_i18n::t;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;
use walkdir::WalkDir;

pub const CURRENT_TEMPLATE_VERSION: &str = "0.4.0";

/// Sufixo do arquivo com o conteúdo novo quando não há base para mesclar.
pub const SIDE_FILE_SUFFIX: &str = "jirau-new";

#[derive(Debug, PartialEq, Eq)]
pub enum MergeResult {
    Unchanged,
    Created,
    Updated,
    Preserved,
    KeptDeleted,
    MergedCleanly,
    Conflict,
    NeedsReview,
}

pub fn run_upgrade(args: &UpgradeArgs) -> Result<()> {
    ui::print_banner();

    let (project_root, mut config) = JirauConfig::find_in_current_or_parent()?;
    let prev_version = config.meta.template_version.clone();
    let base_root = project_root.join(TEMPLATE_BASE_DIR);
    let has_base = base_root.is_dir();

    ui::print_step(&t!("upgrade.project", name = config.app.name.bold()));
    ui::print_substep(&t!("upgrade.location", path = project_root.display()));
    ui::print_substep(&t!(
        "upgrade.versions",
        project = prev_version.cyan(),
        cli = CURRENT_TEMPLATE_VERSION.green().bold()
    ));

    if prev_version == CURRENT_TEMPLATE_VERSION && !args.force {
        println!();
        ui::print_success(&t!(
            "upgrade.already_latest",
            version = CURRENT_TEMPLATE_VERSION
        ));
        ui::print_info(&t!("upgrade.force_hint"));
        return Ok(());
    }

    if !has_base {
        ui::print_warning(&t!(
            "upgrade.no_base",
            dir = TEMPLATE_BASE_DIR,
            suffix = SIDE_FILE_SUFFIX
        ));
    }
    if args.dry_run {
        ui::print_warning(&t!("upgrade.dry_run"));
    }

    let spinner = ui::create_spinner(&t!("upgrade.generating"));
    let temp_target = tempdir()?;
    let mut target_ctx = TemplateContext::with_all_options(
        &config.app.name,
        &config.app.package_id,
        config.app.platforms.clone(),
        &config.app.storage,
        &config.app.android_build_system,
        &config.app.ui_theme,
        config.app.plugins.clone(),
        CURRENT_TEMPLATE_VERSION,
    );
    target_ctx.app_template = config.app.template.clone();
    let target_dir = temp_target.path().join(&target_ctx.slug);
    fs::create_dir_all(&target_dir)?;
    let mut target_writer = ProjectWriter::new(&target_dir);
    Scaffolder::write_project_files(&target_ctx, &mut target_writer)
        .with_context(|| t!("upgrade.generate_failed").into_owned())?;
    ui::finish_spinner_success(&spinner, &t!("upgrade.generated"));

    println!();
    ui::print_step(&t!("upgrade.comparing"));

    let mut counts = Counts::default();
    let mut synced: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&target_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            continue;
        }
        let target_file = entry.path();
        let rel_path = target_file
            .strip_prefix(&target_dir)
            .with_context(|| t!("upgrade.relative_path_failed").into_owned())?
            .to_path_buf();
        // jirau.toml é atualizado à parte (versão do template).
        if rel_path == Path::new(CONFIG_FILE_NAME) {
            continue;
        }

        let base_file = has_base.then(|| base_root.join(&rel_path));
        let result = process_file_merge(
            base_file.as_deref(),
            target_file,
            &project_root.join(&rel_path),
            args.dry_run,
        )?;
        counts.report(&result, &rel_path);
        synced.push(rel_path);
    }

    if !args.dry_run {
        // O template atual vira a base do próximo upgrade.
        for rel in &synced {
            let dest = base_root.join(rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(target_dir.join(rel), dest)?;
        }
        config.meta.template_version = CURRENT_TEMPLATE_VERSION.to_string();
        config.write_to_dir(&project_root)?;
    }

    counts.print_summary();
    Ok(())
}

#[derive(Default)]
struct Counts {
    created: usize,
    updated: usize,
    merged: usize,
    preserved: usize,
    kept_deleted: usize,
    conflicts: usize,
    review: usize,
}

impl Counts {
    fn report(&mut self, result: &MergeResult, rel: &Path) {
        let (label, counter) = match result {
            MergeResult::Created => (
                t!("upgrade.label_created").green().bold(),
                &mut self.created,
            ),
            MergeResult::Updated => (t!("upgrade.label_updated").cyan().bold(), &mut self.updated),
            MergeResult::MergedCleanly => {
                (t!("upgrade.label_merged").blue().bold(), &mut self.merged)
            }
            MergeResult::Conflict => (
                t!("upgrade.label_conflict").yellow().bold(),
                &mut self.conflicts,
            ),
            MergeResult::NeedsReview => (
                t!("upgrade.label_review").magenta().bold(),
                &mut self.review,
            ),
            MergeResult::KeptDeleted => (
                t!("upgrade.label_kept_deleted").dimmed(),
                &mut self.kept_deleted,
            ),
            MergeResult::Preserved => {
                self.preserved += 1;
                return;
            }
            MergeResult::Unchanged => return,
        };
        *counter += 1;
        ui::print_substep(&format!("{} {}", label, rel.display()));
    }

    fn print_summary(&self) {
        println!();
        ui::print_step(&t!("upgrade.summary_title"));
        println!(
            "  • {}",
            t!(
                "upgrade.summary_created",
                count = self.created.to_string().green()
            )
        );
        println!(
            "  • {}",
            t!(
                "upgrade.summary_updated",
                count = self.updated.to_string().cyan()
            )
        );
        println!(
            "  • {}",
            t!(
                "upgrade.summary_merged",
                count = self.merged.to_string().blue()
            )
        );
        println!(
            "  • {}",
            t!("upgrade.summary_preserved", count = self.preserved)
        );
        if self.kept_deleted > 0 {
            println!(
                "  • {}",
                t!("upgrade.summary_kept_deleted", count = self.kept_deleted)
            );
        }
        if self.review > 0 {
            println!(
                "  • {}",
                t!(
                    "upgrade.summary_review",
                    suffix = SIDE_FILE_SUFFIX,
                    count = self.review.to_string().magenta().bold()
                )
            );
        }
        if self.conflicts > 0 {
            println!(
                "  • {}",
                t!(
                    "upgrade.summary_conflicts",
                    count = self.conflicts.to_string().yellow().bold()
                )
            );
            println!();
            ui::print_warning(&t!("upgrade.conflict_warning"));
        } else {
            println!();
            ui::print_success(&t!("upgrade.synced", version = CURRENT_TEMPLATE_VERSION));
        }
    }
}

fn side_file(current: &Path) -> PathBuf {
    let mut name = current.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}", SIDE_FILE_SUFFIX));
    current.with_file_name(name)
}

fn write(path: &Path, contents: &[u8], dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

/// Decide o destino de um arquivo do template. `base_file` é `None` quando o
/// projeto não tem `.jirau/template-base`.
fn process_file_merge(
    base_file: Option<&Path>,
    target_file: &Path,
    current_file: &Path,
    dry_run: bool,
) -> Result<MergeResult> {
    let target = fs::read(target_file)?;
    let base = match base_file {
        Some(p) if p.exists() => Some(fs::read(p)?),
        _ => None,
    };

    if !current_file.exists() {
        if base.is_some() {
            // Estava no template e o usuário apagou: respeita a decisão.
            return Ok(MergeResult::KeptDeleted);
        }
        write(current_file, &target, dry_run)?;
        return Ok(MergeResult::Created);
    }

    let current = fs::read(current_file)?;
    if current == target {
        return Ok(MergeResult::Unchanged);
    }

    let Some(base) = base else {
        // Sem base não há como distinguir customização de versão antiga.
        write(&side_file(current_file), &target, dry_run)?;
        return Ok(MergeResult::NeedsReview);
    };

    if base == current {
        write(current_file, &target, dry_run)?;
        return Ok(MergeResult::Updated);
    }
    if base == target {
        return Ok(MergeResult::Preserved);
    }

    // Ambos mudaram: merge textual (binários vão para revisão).
    let (Ok(current_text), Ok(base_text), Ok(target_text)) = (
        std::str::from_utf8(&current),
        std::str::from_utf8(&base),
        std::str::from_utf8(&target),
    ) else {
        write(&side_file(current_file), &target, dry_run)?;
        return Ok(MergeResult::NeedsReview);
    };

    let (merged, has_conflict) = three_way_merge(current_text, base_text, target_text)?;
    write(current_file, merged.as_bytes(), dry_run)?;
    Ok(if has_conflict {
        MergeResult::Conflict
    } else {
        MergeResult::MergedCleanly
    })
}

/// Merge linha a linha com `git merge-file`; sem git, marca o arquivo inteiro.
fn three_way_merge(current: &str, base: &str, target: &str) -> Result<(String, bool)> {
    let dir = tempdir()?;
    let paths = [
        dir.path().join("current"),
        dir.path().join("base"),
        dir.path().join("target"),
    ];
    for (path, text) in paths.iter().zip([current, base, target]) {
        fs::write(path, text)?;
    }

    let (yours, base_label, new) = (
        t!("upgrade.marker_yours"),
        t!("upgrade.marker_base"),
        t!("upgrade.marker_new"),
    );
    let git = Command::new("git")
        .args(["merge-file", "-p"])
        .args(["-L", &yours, "-L", &base_label, "-L", &new])
        .args(&paths)
        .output();

    match git {
        // Código de saída: 0 = limpo, >0 = nº de conflitos, <0 = erro.
        Ok(out) if out.status.code().is_some_and(|c| c >= 0) => Ok((
            String::from_utf8_lossy(&out.stdout).into_owned(),
            !out.status.success(),
        )),
        _ => Ok(fallback_three_way_merge(current, base, target)),
    }
}

fn fallback_three_way_merge(current: &str, base: &str, target: &str) -> (String, bool) {
    if current == base || current == target {
        return (target.to_string(), false);
    }
    if target == base {
        return (current.to_string(), false);
    }
    let conflict_text = format!(
        "<<<<<<< {}\n{}\n=======\n{}\n>>>>>>> {} (v{})\n",
        t!("upgrade.marker_yours"),
        current.trim_end(),
        target.trim_end(),
        t!("upgrade.marker_new"),
        CURRENT_TEMPLATE_VERSION
    );
    (conflict_text, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _dir: tempfile::TempDir,
        base: PathBuf,
        target: PathBuf,
        current: PathBuf,
    }

    fn fixture(base: Option<&str>, target: &str, current: Option<&str>) -> Fixture {
        let dir = tempdir().unwrap();
        let f = Fixture {
            base: dir.path().join("base.txt"),
            target: dir.path().join("target.txt"),
            current: dir.path().join("current.txt"),
            _dir: dir,
        };
        if let Some(b) = base {
            fs::write(&f.base, b).unwrap();
        }
        fs::write(&f.target, target).unwrap();
        if let Some(c) = current {
            fs::write(&f.current, c).unwrap();
        }
        f
    }

    fn run(f: &Fixture, with_base: bool) -> MergeResult {
        let base = with_base.then_some(f.base.as_path());
        process_file_merge(base, &f.target, &f.current, false).unwrap()
    }

    #[test]
    fn untouched_file_follows_the_template() {
        let f = fixture(Some("v1\n"), "v2\n", Some("v1\n"));
        assert_eq!(run(&f, true), MergeResult::Updated);
        assert_eq!(fs::read_to_string(&f.current).unwrap(), "v2\n");
    }

    #[test]
    fn customization_is_kept_when_template_did_not_change() {
        let f = fixture(Some("v1\n"), "v1\n", Some("meu\n"));
        assert_eq!(run(&f, true), MergeResult::Preserved);
        assert_eq!(fs::read_to_string(&f.current).unwrap(), "meu\n");
    }

    #[test]
    fn independent_changes_are_merged() {
        let f = fixture(
            Some("a\nb\nc\nd\ne\n"),
            "a\nb\nc\nd\nE-template\n",
            Some("A-usuario\nb\nc\nd\ne\n"),
        );
        assert_eq!(run(&f, true), MergeResult::MergedCleanly);
        assert_eq!(
            fs::read_to_string(&f.current).unwrap(),
            "A-usuario\nb\nc\nd\nE-template\n"
        );
    }

    #[test]
    fn overlapping_changes_are_marked_as_conflict() {
        let f = fixture(Some("linha\n"), "template\n", Some("usuario\n"));
        assert_eq!(run(&f, true), MergeResult::Conflict);
        let merged = fs::read_to_string(&f.current).unwrap();
        assert!(merged.contains(&format!("<<<<<<< {}", t!("upgrade.marker_yours"))));
        assert!(merged.contains(&format!(">>>>>>> {}", t!("upgrade.marker_new"))));
    }

    #[test]
    fn file_deleted_by_user_stays_deleted() {
        let f = fixture(Some("v1\n"), "v2\n", None);
        assert_eq!(run(&f, true), MergeResult::KeptDeleted);
        assert!(!f.current.exists());
    }

    #[test]
    fn new_template_file_is_created() {
        let f = fixture(None, "novo\n", None);
        assert_eq!(run(&f, true), MergeResult::Created);
        assert_eq!(fs::read_to_string(&f.current).unwrap(), "novo\n");
    }

    #[test]
    fn without_base_the_new_content_goes_to_a_side_file() {
        let f = fixture(None, "template\n", Some("antigo\n"));
        assert_eq!(run(&f, false), MergeResult::NeedsReview);
        assert_eq!(fs::read_to_string(&f.current).unwrap(), "antigo\n");
        let side = side_file(&f.current);
        assert!(side.to_string_lossy().ends_with(".jirau-new"));
        assert_eq!(fs::read_to_string(side).unwrap(), "template\n");
    }

    #[test]
    fn fallback_merge_marks_conflicts() {
        let (merged, conflict) = fallback_three_way_merge("usuario\n", "base\n", "template\n");
        assert!(conflict);
        assert!(merged.contains(&format!("<<<<<<< {}", t!("upgrade.marker_yours"))));
        let (merged, conflict) = fallback_three_way_merge("base\n", "base\n", "template\n");
        assert!(!conflict);
        assert_eq!(merged, "template\n");
    }
}
