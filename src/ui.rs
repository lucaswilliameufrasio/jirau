use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rust_i18n::t;
use std::time::Duration;

pub fn print_banner() {
    let border = "─".repeat(58);
    println!("{}", format!("╭{border}╮").cyan());
    println!(
        "{}",
        format!("│  {:<55}│", t!("ui.banner_title")).cyan().bold()
    );
    println!(
        "{}",
        format!("│      {:<52}│", "Rust + Android (Compose) + iOS (SwiftUI)").cyan()
    );
    println!("{}", format!("╰{border}╯").cyan());
    println!();
}

pub fn print_step(title: &str) {
    println!("{} {}", "➜".cyan().bold(), title.bold());
}

pub fn print_substep(detail: &str) {
    println!("  {} {}", "•".bright_black(), detail);
}

pub fn print_success(message: &str) {
    println!("{} {}", "✔".green().bold(), message.green());
}

pub fn print_warning(message: &str) {
    println!("{} {}", "!".yellow().bold(), message.yellow());
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "✖".red().bold(), message.red());
}

pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn finish_spinner_success(pb: &ProgressBar, msg: &str) {
    pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
    pb.finish_with_message(format!("{} {}", "✔".green().bold(), msg));
}

pub fn finish_spinner_error(pb: &ProgressBar, msg: &str) {
    pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
    pb.finish_with_message(format!("{} {}", "✖".red().bold(), msg));
}
