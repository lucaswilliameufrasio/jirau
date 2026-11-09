//! Execução de ferramentas externas (cargo, gradle, xcodebuild, adb...).

use anyhow::{Context, Result, bail};
use colored::*;
use rust_i18n::t;
use std::process::{Command, Output};

/// Linhas que identificam a causa de uma falha nas saídas do cargo, Gradle,
/// compilador Kotlin e xcodebuild.
fn is_error_line(line: &str) -> bool {
    let l = line.trim_start();
    l.contains("error:")
        || l.contains("error[")
        || l.starts_with("e: ")
        || l.contains("** BUILD FAILED")
        || l.contains("** TEST FAILED")
        || l.starts_with("* What went wrong")
        || l.starts_with("> ")
        || l.contains("FAILED")
        || l.contains("Exception")
}

/// Executa um comando capturando a saída; em caso de falha mostra só as linhas
/// relevantes (ou o final do log) em vez de despejar tudo.
pub fn run_quiet(cmd: &mut Command, what: &str) -> Result<Output> {
    let output = cmd
        .output()
        .with_context(|| t!("process.spawn_failed", what = what).into_owned())?;
    if output.status.success() {
        return Ok(output);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined: Vec<&str> = stdout.lines().chain(stderr.lines()).collect();
    let errors: Vec<&str> = combined
        .iter()
        .copied()
        .filter(|l| is_error_line(l))
        .collect();
    let shown: Vec<&str> = if errors.is_empty() {
        combined.iter().rev().take(30).rev().copied().collect()
    } else {
        errors.into_iter().take(40).collect()
    };
    eprintln!();
    for line in shown {
        eprintln!("    {}", line.red());
    }
    bail!("{}", t!("process.failed", what = what))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_errors_from_each_toolchain() {
        assert!(is_error_line("error[E0425]: cannot find function"));
        assert!(is_error_line(
            "e: file:///app/MainActivity.kt:10:5 Unresolved reference"
        ));
        assert!(is_error_line("* What went wrong:"));
        assert!(is_error_line("** BUILD FAILED **"));
        assert!(!is_error_line("BUILD SUCCESSFUL in 3s"));
    }
}
