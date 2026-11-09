//! Tradução da interface do clap: ajuda, uso e mensagens de erro.
//!
//! O clap não tem i18n: títulos ("Usage", "Options"...), as flags `--help` e
//! `--version`, o subcomando `help`, a lista de valores possíveis e todas as
//! mensagens de erro são fixos em inglês. Aqui cada um é substituído pela API
//! pública do clap: `help_template`, títulos explícitos, flags declaradas por
//! nós, a linha de uso no template e um [`ErrorFormatter`] próprio. As frases ficam em
//! `locales/clap.yml`.

use crate::cli::Cli;
use clap::builder::{StyledStr, Styles};
use clap::error::{ContextKind, ContextValue, ErrorFormatter, ErrorKind};
use clap::{Arg, ArgAction, ArgMatches, Command, CommandFactory, FromArgMatches};
use rust_i18n::t;
use std::fmt::Write as _;

const TAB: &str = "  ";
const HELP_SUBCOMMAND: &str = "help";

/// Texto traduzido de uma chave; `None` se a chave não existir.
fn lookup(key: &str) -> Option<String> {
    let text = t!(key).into_owned();
    (text != key).then_some(text)
}

/// Aplica as traduções de `cli.*` (textos do jirau) e `clap.*` (textos do clap).
pub fn localize(cmd: Command) -> Command {
    let cmd = localize_texts(cmd);
    let cmd = localize_command(cmd, true);
    localize_usage(cmd)
}

/// Descrições do jirau: `cli.about`, `cli.<arg>`, `cli.<sub>.about`, `cli.<sub>.<arg>`.
fn localize_texts(mut cmd: Command) -> Command {
    if let Some(about) = lookup("cli.about") {
        cmd = cmd.about(about);
    }
    if let Some(long) = lookup("cli.long_about") {
        cmd = cmd.long_about(long);
    }
    cmd = localize_arg_help(cmd, "cli");
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
            localize_arg_help(sub, &format!("cli.{name}"))
        });
    }
    cmd
}

fn localize_arg_help(mut cmd: Command, prefix: &str) -> Command {
    for id in arg_ids(&cmd) {
        if let Some(help) = lookup(&format!("{prefix}.{id}")) {
            cmd = cmd.mut_arg(id, |a| a.help(help));
        }
    }
    cmd
}

fn arg_ids(cmd: &Command) -> Vec<String> {
    cmd.get_arguments()
        .map(|a| a.get_id().to_string())
        .collect()
}

/// Template, títulos, nomes de valores, valores possíveis e flags de ajuda/versão.
fn localize_command(mut cmd: Command, is_root: bool) -> Command {
    cmd = cmd
        .subcommand_help_heading(t!("clap.heading.commands").into_owned())
        .subcommand_value_name(t!("clap.value.COMMAND").into_owned());

    for id in arg_ids(&cmd) {
        cmd = cmd.mut_arg(id, localize_arg);
    }

    let has_long_help =
        cmd.get_long_about().is_some() || cmd.get_arguments().any(|a| a.get_long_help().is_some());
    let mut help = Arg::new("help")
        .short('h')
        .long("help")
        .action(ArgAction::Help)
        .help_heading(t!("clap.heading.options").into_owned());
    help = if has_long_help {
        help.help(t!("clap.flag.help_more").into_owned())
            .long_help(t!("clap.flag.help_summary").into_owned())
    } else {
        help.help(t!("clap.flag.help").into_owned())
    };
    cmd = cmd.disable_help_flag(true).arg(help);

    if is_root && cmd.get_version().is_some() {
        cmd = cmd.disable_version_flag(true).arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help(t!("clap.flag.version").into_owned())
                .help_heading(t!("clap.heading.options").into_owned()),
        );
    }

    let names: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for name in names {
        cmd = cmd.mut_subcommand(&name, |sub| localize_command(sub, false));
    }

    if cmd.has_subcommands() {
        let help_subcommand = Command::new(HELP_SUBCOMMAND)
            .about(t!("clap.help_subcommand.about").into_owned())
            .arg(
                Arg::new("subcommand")
                    .action(ArgAction::Append)
                    .num_args(..)
                    .value_name(t!("clap.value.COMMAND").into_owned())
                    .help(t!("clap.help_subcommand.arg").into_owned()),
            );
        cmd = cmd
            .disable_help_subcommand(true)
            .subcommand(localize_command(help_subcommand, false));
    }
    cmd
}

fn localize_arg(mut arg: Arg) -> Arg {
    let takes_values = arg.get_action().takes_values();
    let heading = if arg.is_positional() {
        t!("clap.heading.arguments")
    } else {
        t!("clap.heading.options")
    };
    if arg.get_help_heading().is_none() {
        arg = arg.help_heading(heading.into_owned());
    }

    if let Some(names) = arg.get_value_names() {
        let translated: Vec<String> = names
            .iter()
            .map(|n| lookup(&format!("clap.value.{n}")).unwrap_or_else(|| n.to_string()))
            .collect();
        arg = arg.value_names(translated);
    } else if let Some(name) = lookup(&format!(
        "clap.value.{}",
        arg.get_id().as_str().to_uppercase()
    )) {
        arg = arg.value_name(name);
    }

    let mut extra = Vec::new();
    let defaults: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    if takes_values && !defaults.is_empty() && !arg.is_hide_default_value_set() {
        extra.push(format!(
            "[{}: {}]",
            t!("clap.default_value"),
            defaults.join(" ")
        ));
        arg = arg.hide_default_value(true);
    }
    let possible: Vec<String> = arg
        .get_possible_values()
        .iter()
        .filter(|v| !v.is_hide_set())
        .map(|v| v.get_name().to_string())
        .collect();
    if takes_values && !possible.is_empty() && !arg.is_hide_possible_values_set() {
        extra.push(format!(
            "[{}: {}]",
            t!("clap.possible_values"),
            possible.join(", ")
        ));
        arg = arg.hide_possible_values(true);
    }
    if !extra.is_empty() {
        let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
        arg = arg.help(format!("{help} {}", extra.join(" ")).trim().to_string());
    }
    arg
}

/// O clap escreve "Usage:" e `[OPTIONS]` fixos na linha de uso. A linha de uso
/// completa é renderizada uma vez e embutida já traduzida no template de ajuda;
/// a linha de uso dos erros (que o clap adapta aos argumentos usados) é
/// traduzida no [`LocalizedFormatter`].
fn localize_usage(cmd: Command) -> Command {
    let mut built = cmd.clone();
    built.build();
    apply_usage(cmd, &mut built)
}

fn apply_usage(mut cmd: Command, built: &mut Command) -> Command {
    let usage = usage_without_heading(built).ansi().to_string();
    let usage_style = Styles::default().get_usage().to_owned();
    let template = format!(
        "{{before-help}}{{about-with-newline}}\n{usage_style}{heading}:{usage_style:#} {usage}\n\n{{all-args}}{{after-help}}",
        heading = t!("clap.usage"),
        usage = usage.replace('{', "{{").replace('}', "}}"),
    );
    cmd = cmd.help_template(template);
    let names: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for name in names {
        if let Some(built_sub) = built.find_subcommand_mut(&name) {
            let mut built_sub = built_sub.clone();
            cmd = cmd.mut_subcommand(&name, |sub| apply_usage(sub, &mut built_sub));
        }
    }
    cmd
}

fn usage_without_heading(cmd: &mut Command) -> StyledStr {
    let rendered = cmd.render_usage().ansi().to_string();
    // "<estilo>Usage:<reset> jirau new [OPTIONS]" → "jirau new [OPÇÕES]"
    let body = rendered
        .split_once(':')
        .map(|(_, rest)| rest)
        .unwrap_or(&rendered);
    let body = body.split_once(' ').map(|(_, rest)| rest).unwrap_or(body);
    StyledStr::from(translate_options_marker(body))
}

fn translate_options_marker(usage: &str) -> String {
    usage.replace("[OPTIONS]", &format!("[{}]", t!("clap.value.OPTIONS")))
}

/// Lê os argumentos com ajuda e erros traduzidos.
pub fn parse_localized_cli() -> anyhow::Result<Cli> {
    parse_from(std::env::args_os())
}

fn parse_from<I, T>(args: I) -> anyhow::Result<Cli>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let mut cmd = localize(Cli::command());
    let matches = match cmd.try_get_matches_from_mut(args) {
        Ok(matches) => matches,
        Err(err) => exit_with(err),
    };
    if let Some((HELP_SUBCOMMAND, sub)) = matches.subcommand() {
        print_help_for(&mut cmd, sub);
    }
    match Cli::from_arg_matches(&matches) {
        Ok(cli) => Ok(cli),
        Err(err) => exit_with(err.with_cmd(&cmd)),
    }
}

fn exit_with(err: clap::Error) -> ! {
    match err.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => err.exit(),
        _ => err.apply::<LocalizedFormatter>().exit(),
    }
}

/// `jirau help [comando...]`: mostra a ajuda completa do comando pedido.
fn print_help_for(cmd: &mut Command, matches: &ArgMatches) -> ! {
    let path: Vec<String> = matches
        .get_many::<String>("subcommand")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let mut current = cmd;
    for name in &path {
        if current.find_subcommand(name).is_none() {
            let mut err = clap::Error::new(ErrorKind::InvalidSubcommand).with_cmd(current);
            err.insert(
                ContextKind::InvalidSubcommand,
                ContextValue::String(name.clone()),
            );
            exit_with(err);
        }
        current = current
            .find_subcommand_mut(name)
            .expect("subcomando conferido acima");
    }
    let _ = current.print_long_help();
    std::process::exit(0);
}

/// Mesma estrutura do `RichFormatter` do clap, com as frases traduzidas.
pub struct LocalizedFormatter;

impl ErrorFormatter for LocalizedFormatter {
    fn format_error(error: &clap::error::Error<Self>) -> StyledStr {
        let styles = Styles::default();
        let (err_style, valid) = (styles.get_error(), styles.get_valid());
        let literal = styles.get_literal();

        let mut out = String::new();
        let _ = write!(out, "{err_style}{}:{err_style:#} ", t!("clap.error.prefix"));
        out.push_str(&describe(error, &styles));

        let mut suggested = false;
        for (kind, what) in [
            (ContextKind::SuggestedSubcommand, "subcommand"),
            (ContextKind::SuggestedArg, "argument"),
            (ContextKind::SuggestedValue, "value"),
        ] {
            if let Some(possibles) = error.get(kind) {
                out.push('\n');
                if !suggested {
                    out.push('\n');
                    suggested = true;
                }
                did_you_mean(&mut out, &styles, what, possibles);
            }
        }
        if let Some(ContextValue::StyledStrs(tips)) = error.get(ContextKind::Suggested) {
            if !suggested {
                out.push('\n');
            }
            for tip in tips {
                let _ = write!(
                    out,
                    "\n{TAB}{valid}{}:{valid:#} {}",
                    t!("clap.error.tip"),
                    translate_tip(&tip.ansi().to_string())
                );
            }
        }

        if let Some(ContextValue::StyledStr(usage)) = error.get(ContextKind::Usage) {
            let usage = usage.ansi().to_string();
            let body = usage.split_once(':').map(|(_, r)| r).unwrap_or(&usage);
            let body = body.split_once(' ').map(|(_, r)| r).unwrap_or(body);
            let usage_style = styles.get_usage();
            let _ = write!(
                out,
                "\n\n{usage_style}{}:{usage_style:#} {}",
                t!("clap.usage"),
                translate_options_marker(body)
            );
        }

        let help = format!("{literal}--help{literal:#}");
        let _ = writeln!(out, "\n\n{}", t!("clap.error.more_info", help = help));
        StyledStr::from(out)
    }
}

/// Dicas fixas que o clap gera para `--` e para valores começando com `-`.
fn translate_tip(tip: &str) -> String {
    if tip.contains("to pass '") && tip.contains("as a value, use '") {
        let quoted: Vec<&str> = tip.split('\'').collect();
        if quoted.len() >= 4 {
            return t!(
                "clap.error.tip_escape",
                value = quoted[1],
                escaped = quoted[3]
            )
            .into_owned();
        }
    }
    tip.to_string()
}

fn ctx_string(error: &clap::error::Error<LocalizedFormatter>, kind: ContextKind) -> Option<String> {
    match error.get(kind) {
        Some(ContextValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn ctx_number(error: &clap::error::Error<LocalizedFormatter>, kind: ContextKind) -> Option<isize> {
    match error.get(kind) {
        Some(ContextValue::Number(n)) => Some(*n),
        _ => None,
    }
}

fn describe(error: &clap::error::Error<LocalizedFormatter>, styles: &Styles) -> String {
    let (invalid, valid, literal) = (
        styles.get_invalid(),
        styles.get_valid(),
        styles.get_literal(),
    );
    let bad = |s: &str| format!("{invalid}{s}{invalid:#}");
    let lit = |s: &str| format!("{literal}{s}{literal:#}");
    let good = |s: &str| format!("{valid}{s}{valid:#}");
    let arg = ctx_string(error, ContextKind::InvalidArg);
    let value = ctx_string(error, ContextKind::InvalidValue);

    let message = match error.kind() {
        ErrorKind::ArgumentConflict => {
            let prior = error.get(ContextKind::PriorArg);
            let mut msg = match (&arg, ctx_string(error, ContextKind::InvalidSubcommand)) {
                (Some(a), _) if prior == Some(&ContextValue::String(a.clone())) => {
                    return t!("clap.error.argument_repeated", arg = bad(a)).into_owned();
                }
                (Some(a), _) => t!("clap.error.argument_conflict", arg = bad(a)).into_owned(),
                (None, Some(s)) => t!("clap.error.subcommand_conflict", sub = bad(&s)).into_owned(),
                (None, None) => t!("clap.error.kind.argument_conflict").into_owned(),
            };
            match prior {
                Some(ContextValue::Strings(values)) => {
                    msg.push(':');
                    for v in values {
                        let _ = write!(msg, "\n{TAB}{}", bad(v));
                    }
                }
                Some(ContextValue::String(v)) => {
                    let _ = write!(msg, " '{}'", bad(v));
                }
                _ => {
                    let _ = write!(msg, " {}", t!("clap.error.other_arguments"));
                }
            }
            Some(msg)
        }
        ErrorKind::NoEquals => arg.map(|a| t!("clap.error.no_equals", arg = bad(&a)).into_owned()),
        ErrorKind::InvalidValue => match (&arg, &value) {
            (Some(a), Some(v)) => {
                let mut msg = if v.is_empty() {
                    t!("clap.error.value_required", arg = bad(a)).into_owned()
                } else {
                    t!("clap.error.invalid_value", value = bad(v), arg = lit(a)).into_owned()
                };
                values_list(
                    &mut msg,
                    &t!("clap.possible_values"),
                    error.get(ContextKind::ValidValue),
                    &good,
                );
                Some(msg)
            }
            _ => None,
        },
        ErrorKind::InvalidSubcommand => ctx_string(error, ContextKind::InvalidSubcommand)
            .map(|s| t!("clap.error.unrecognized_subcommand", sub = bad(&s)).into_owned()),
        ErrorKind::MissingRequiredArgument => match error.get(ContextKind::InvalidArg) {
            Some(ContextValue::Strings(args)) => {
                let mut msg = t!("clap.error.missing_required").into_owned();
                for a in args {
                    let _ = write!(msg, "\n{TAB}{}", good(a));
                }
                Some(msg)
            }
            _ => None,
        },
        ErrorKind::MissingSubcommand => {
            ctx_string(error, ContextKind::InvalidSubcommand).map(|s| {
                let mut msg = t!("clap.error.missing_subcommand", sub = bad(&s)).into_owned();
                values_list(
                    &mut msg,
                    &t!("clap.subcommands"),
                    error.get(ContextKind::ValidSubcommand),
                    &good,
                );
                msg
            })
        }
        ErrorKind::TooManyValues => match (&arg, &value) {
            (Some(a), Some(v)) => {
                Some(t!("clap.error.too_many_values", value = bad(v), arg = lit(a)).into_owned())
            }
            _ => None,
        },
        ErrorKind::TooFewValues | ErrorKind::WrongNumberOfValues => {
            let expected_kind = if error.kind() == ErrorKind::TooFewValues {
                ContextKind::MinValues
            } else {
                ContextKind::ExpectedNumValues
            };
            match (
                &arg,
                ctx_number(error, ContextKind::ActualNumValues),
                ctx_number(error, expected_kind),
            ) {
                (Some(a), Some(actual), Some(expected)) => Some(
                    t!(
                        "clap.error.wrong_number_of_values",
                        expected = good(&expected.to_string()),
                        arg = lit(a),
                        actual = bad(&actual.to_string())
                    )
                    .into_owned(),
                ),
                _ => None,
            }
        }
        ErrorKind::ValueValidation => match (&arg, &value) {
            (Some(a), Some(v)) => {
                let mut msg =
                    t!("clap.error.invalid_value", value = bad(v), arg = lit(a)).into_owned();
                if let Some(source) = std::error::Error::source(error) {
                    let _ = write!(msg, ": {source}");
                }
                Some(msg)
            }
            _ => None,
        },
        ErrorKind::UnknownArgument => {
            arg.map(|a| t!("clap.error.unknown_argument", arg = bad(&a)).into_owned())
        }
        _ => None,
    };
    message.unwrap_or_else(|| kind_message(error.kind()))
}

fn kind_message(kind: ErrorKind) -> String {
    let key = match kind {
        ErrorKind::InvalidValue => "invalid_value",
        ErrorKind::UnknownArgument => "unknown_argument",
        ErrorKind::InvalidSubcommand => "invalid_subcommand",
        ErrorKind::NoEquals => "no_equals",
        ErrorKind::ValueValidation => "value_validation",
        ErrorKind::TooManyValues => "too_many_values",
        ErrorKind::TooFewValues => "too_few_values",
        ErrorKind::WrongNumberOfValues => "wrong_number_of_values",
        ErrorKind::ArgumentConflict => "argument_conflict",
        ErrorKind::MissingRequiredArgument => "missing_required_argument",
        ErrorKind::MissingSubcommand => "missing_subcommand",
        ErrorKind::InvalidUtf8 => "invalid_utf8",
        ErrorKind::Io => "io",
        ErrorKind::Format => "format",
        _ => "unknown",
    };
    let key = format!("clap.error.kind.{key}");
    t!(&key).into_owned()
}

fn values_list(
    out: &mut String,
    name: &str,
    values: Option<&ContextValue>,
    good: &dyn Fn(&str) -> String,
) {
    if let Some(ContextValue::Strings(values)) = values
        && !values.is_empty()
    {
        let list: Vec<String> = values.iter().map(|v| good(v)).collect();
        let _ = write!(out, "\n{TAB}[{name}: {}]", list.join(", "));
    }
}

fn did_you_mean(out: &mut String, styles: &Styles, what: &str, possibles: &ContextValue) {
    let valid = styles.get_valid();
    let quoted = |s: &str| format!("'{valid}{s}{valid:#}'");
    let list: Vec<String> = match possibles {
        ContextValue::String(p) => vec![quoted(p)],
        ContextValue::Strings(ps) => ps.iter().map(|p| quoted(p)).collect(),
        _ => return,
    };
    let key = if list.len() == 1 {
        format!("clap.error.similar_{what}")
    } else {
        format!("clap.error.similar_{what}s")
    };
    let _ = write!(
        out,
        "{TAB}{valid}{}:{valid:#} {}",
        t!("clap.error.tip"),
        t!(key.as_str(), list = list.join(", "))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::tests::LOCALE_LOCK;

    fn plain(s: StyledStr) -> String {
        s.to_string()
    }

    fn render(lang: &str, args: &[&str]) -> String {
        rust_i18n::set_locale(lang);
        let mut cmd = localize(Cli::command());
        let text = match cmd.try_get_matches_from_mut(args) {
            Ok(_) => String::new(),
            Err(err) => match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => err.to_string(),
                _ => plain(err.apply::<LocalizedFormatter>().render()),
            },
        };
        rust_i18n::set_locale("en");
        text
    }

    /// Frases fixas do clap em inglês que não podem aparecer em pt-BR.
    const CLAP_ENGLISH: &[&str] = &[
        "Usage:",
        "Commands:",
        "Options:",
        "Arguments:",
        "Print help",
        "Print version",
        "Print this message",
        "possible values",
        "[OPTIONS]",
        "[COMMAND]",
        "error:",
        "tip:",
        "For more information",
        "unexpected argument",
        "invalid value",
        "unrecognized subcommand",
        "a similar",
    ];

    #[test]
    fn portuguese_help_and_errors_have_no_clap_english() {
        let _guard = LOCALE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cases: &[&[&str]] = &[
            &["jirau", "--help"],
            &["jirau", "-h"],
            &["jirau", "new", "--help"],
            &["jirau", "run", "--help"],
            &["jirau", "upgrade", "-h"],
            &["jirau", "nwe"],
            &["jirau", "upgrad"],
            &["jirau", "run", "--x"],
            &["jirau", "new", "--bogus"],
            &["jirau", "new", "--andriod"],
            &["jirau", "--lang", "xx"],
            &["jirau", "--lnag", "en"],
        ];
        for args in cases {
            let out = render("pt-BR", args);
            assert!(!out.is_empty(), "{args:?} sem saída");
            for english in CLAP_ENGLISH {
                assert!(
                    !out.contains(english),
                    "{args:?} contém '{english}':\n{out}"
                );
            }
        }
        let help = render("pt-BR", &["jirau", "--help"]);
        for expected in [
            "Uso:",
            "Comandos:",
            "Opções:",
            "Mostra a ajuda",
            "Mostra a versão",
            "valores possíveis",
            "[OPÇÕES]",
            "[COMANDO]",
        ] {
            assert!(help.contains(expected), "faltou '{expected}':\n{help}");
        }
        let err = render("pt-BR", &["jirau", "upgrad"]);
        assert!(
            err.contains("erro:")
                && err.contains("dica: existe um subcomando parecido: 'upgrade'")
                && err.contains("Para mais informações"),
            "{err}"
        );
    }

    #[test]
    fn english_matches_clap_wording() {
        let _guard = LOCALE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let help = render("en", &["jirau", "--help"]);
        for expected in [
            "Usage: jirau [OPTIONS] [COMMAND]",
            "Commands:",
            "Options:",
            "Print help",
            "Print version",
            "[possible values: en, pt-BR]",
        ] {
            assert!(help.contains(expected), "faltou '{expected}':\n{help}");
        }
        let err = render("en", &["jirau", "new", "--bogus"]);
        assert!(
            err.starts_with("error: unexpected argument '--bogus' found"),
            "{err}"
        );
        assert!(err.contains("For more information, try '--help'."), "{err}");
    }

    #[test]
    fn help_subcommand_is_localized() {
        let _guard = LOCALE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let help = render("pt-BR", &["jirau", "--help"]);
        assert!(help.contains("Mostra esta mensagem ou a ajuda"), "{help}");
        rust_i18n::set_locale("pt-BR");
        let matches = localize(Cli::command())
            .try_get_matches_from(["jirau", "help", "new"])
            .unwrap();
        rust_i18n::set_locale("en");
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, HELP_SUBCOMMAND);
        assert_eq!(
            sub.get_many::<String>("subcommand")
                .unwrap()
                .collect::<Vec<_>>(),
            ["new"]
        );
    }

    #[test]
    fn texts_from_cli_yml_are_applied() {
        let _guard = LOCALE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let en = render("en", &["jirau", "--help"]);
        let pt = render("pt-BR", &["jirau", "--help"]);
        assert!(en.contains("Creates a new project"), "{en}");
        assert!(pt.contains("Cria um novo projeto"), "{pt}");
    }
}
