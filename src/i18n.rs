//! Internacionalização da CLI (en, pt-BR).
//!
//! Idioma: `--lang` > `JIRAU_LANG` > locale do sistema (`pt*` → pt-BR) > `en`.
//! As mensagens ficam em `locales/*.yml` (formato v2 do rust-i18n: cada chave
//! traz `en` e `pt-BR` lado a lado, o que facilita manter as duas em dia).

use crate::cli::Cli;
use clap::{Command, CommandFactory, FromArgMatches};

/// Normaliza uma tag de idioma (`pt_BR.UTF-8`, `pt-PT`, `en-US`...) para um
/// dos idiomas suportados.
pub fn normalize(tag: &str) -> &'static str {
    if tag.trim().to_ascii_lowercase().starts_with("pt") {
        "pt-BR"
    } else {
        "en"
    }
}

/// Idioma a partir de `--lang`, `JIRAU_LANG` ou do sistema.
pub fn resolve(args: &[String], env_lang: Option<String>, system: Option<String>) -> &'static str {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix("--lang=") {
            return normalize(value);
        }
        if arg == "--lang"
            && let Some(value) = iter.next()
        {
            return normalize(value);
        }
    }
    env_lang
        .filter(|v| !v.trim().is_empty())
        .or(system)
        .map(|v| normalize(&v))
        .unwrap_or("en")
}

pub fn init_from_args(args: impl Iterator<Item = String>) {
    let args: Vec<String> = args.collect();
    let lang = resolve(
        &args,
        std::env::var("JIRAU_LANG").ok(),
        sys_locale::get_locale(),
    );
    rust_i18n::set_locale(lang);
}

/// Texto traduzido de uma chave; `None` se a chave não existir.
fn lookup(key: &str) -> Option<String> {
    let text = rust_i18n::t!(key).into_owned();
    (text != key).then_some(text)
}

/// Aplica as traduções `cli.*` aos textos de ajuda do clap.
pub fn localize(cmd: Command) -> Command {
    fn localize_args(mut cmd: Command, prefix: &str) -> Command {
        let ids: Vec<String> = cmd
            .get_arguments()
            .map(|a| a.get_id().to_string())
            .filter(|id| id != "help" && id != "version")
            .collect();
        for id in ids {
            if let Some(help) = lookup(&format!("{prefix}.{id}")) {
                cmd = cmd.mut_arg(id, |a| a.help(help));
            }
        }
        cmd
    }

    let mut cmd = cmd;
    if let Some(about) = lookup("cli.about") {
        cmd = cmd.about(about);
    }
    if let Some(long) = lookup("cli.long_about") {
        cmd = cmd.long_about(long);
    }
    cmd = localize_args(cmd, "cli");
    let names: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for name in names {
        cmd = cmd.mut_subcommand(&name, |sub| {
            let sub = match lookup(&format!("cli.{name}.about")) {
                Some(about) => sub.about(about),
                None => sub,
            };
            localize_args(sub, &format!("cli.{name}"))
        });
    }
    cmd
}

pub fn parse_localized_cli() -> anyhow::Result<Cli> {
    let matches = localize(Cli::command()).get_matches();
    Ok(Cli::from_arg_matches(&matches)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const SUPPORTED: &[&str] = &["en", "pt-BR"];

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn language_resolution_order() {
        assert_eq!(
            resolve(
                &args(&["jirau", "--lang", "en"]),
                Some("pt_BR".into()),
                None
            ),
            "en"
        );
        assert_eq!(
            resolve(
                &args(&["jirau", "--lang=pt-BR"]),
                None,
                Some("en-US".into())
            ),
            "pt-BR"
        );
        assert_eq!(
            resolve(
                &args(&["jirau"]),
                Some("pt_BR.UTF-8".into()),
                Some("en-US".into())
            ),
            "pt-BR"
        );
        assert_eq!(
            resolve(&args(&["jirau"]), None, Some("pt-PT".into())),
            "pt-BR"
        );
        assert_eq!(
            resolve(&args(&["jirau"]), Some(String::new()), Some("de-DE".into())),
            "en"
        );
        assert_eq!(resolve(&args(&["jirau"]), None, None), "en");
    }

    /// Lê `locales/*.yml` (formato v2) em chave → idiomas presentes.
    fn locale_entries() -> BTreeMap<String, Vec<String>> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut entries: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for file in std::fs::read_dir(dir).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_none_or(|e| e != "yml") {
                continue;
            }
            let mut current: Option<String> = None;
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if line.starts_with('#') || line.trim().is_empty() || line.starts_with("_version") {
                    continue;
                }
                if !line.starts_with(' ') {
                    let key = line.trim_end_matches(':').trim_matches('"').to_string();
                    assert!(
                        entries.insert(key.clone(), Vec::new()).is_none(),
                        "chave duplicada {key} em {}",
                        path.display()
                    );
                    current = Some(key);
                } else if let Some(key) = &current
                    && let Some((lang, _)) = line.trim_start().split_once(':')
                    && !line.starts_with("    ")
                {
                    entries
                        .get_mut(key)
                        .unwrap()
                        .push(lang.trim_matches('"').to_string());
                }
            }
        }
        entries
    }

    #[test]
    fn every_message_has_english_and_portuguese() {
        let entries = locale_entries();
        assert!(!entries.is_empty());
        for (key, langs) in &entries {
            for lang in SUPPORTED {
                assert!(
                    langs.iter().any(|l| l == lang),
                    "'{key}' sem tradução '{lang}'"
                );
            }
        }
    }

    #[test]
    fn help_is_localized() {
        let _guard = LOCALE_LOCK.lock().unwrap();
        rust_i18n::set_locale("en");
        let en = localize(Cli::command()).render_long_help().to_string();
        rust_i18n::set_locale("pt-BR");
        let pt = localize(Cli::command()).render_long_help().to_string();
        rust_i18n::set_locale("en");
        assert!(en.contains("Creates a new project"), "{en}");
        assert!(pt.contains("Cria um novo projeto"), "{pt}");
    }

    /// O idioma do rust-i18n é global: testes que o trocam rodam em série.
    pub static LOCALE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
