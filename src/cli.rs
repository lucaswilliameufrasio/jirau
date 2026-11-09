use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "jirau",
    author = "Lucas Eufrasio <lucaswilliameufrasio@users.noreply.github.com>",
    version,
    about = "🪵 Native cross-platform scaffold CLI: Rust + Android (Compose) + iOS (SwiftUI)",
    long_about = "Jirau is a Flutter-inspired CLI (create, doctor, run) that generates and operates projects with a shared Rust core and 100% native UIs in Jetpack Compose and SwiftUI via UniFFI."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Language / idioma (en, pt-BR). Default: JIRAU_LANG or the system locale.
    #[arg(long, global = true, value_parser = ["en", "pt-BR"])]
    pub lang: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Creates a new project from the native template
    New(NewArgs),

    /// Runs the app on the selected device or simulator
    Run(RunArgs),

    /// Runs unit, integration and on-device tests
    Test(TestArgs),

    /// Diagnoses the development environment (Rust, Android SDK/NDK, Xcode, etc.)
    Doctor(DoctorArgs),

    /// Fast development loop (build, install and redeploy on change)
    Dev(DevArgs),

    /// Regenerates the Kotlin and Swift bindings from the UniFFI annotations
    Bindings(BindingsArgs),

    /// Builds final artifacts (multi-ABI APK for Android, XCFramework for iOS)
    Build(BuildArgs),

    /// Adds a platform to an existing project (e.g. jirau add desktop)
    Add(AddArgs),

    /// Updates the project to the current template via 3-way merge
    Upgrade(UpgradeArgs),
}

#[derive(Args, Debug, Default)]
pub struct UpgradeArgs {
    /// Run even if the project is already on the latest template
    #[arg(short, long)]
    pub force: bool,

    /// Show what would change without writing anything
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Platform to add (desktop, android, ios)
    pub platform: Option<String>,
}

#[derive(Args, Debug)]
pub struct NewArgs {
    /// App/project name
    pub name: Option<String>,

    /// Package ID / Bundle Identifier (e.g. com.example.myapp)
    #[arg(short, long)]
    pub package: Option<String>,

    /// Include Android support
    #[arg(long)]
    pub android: bool,

    /// Include iOS support
    #[arg(long)]
    pub ios: bool,

    /// Include Desktop support (Rust UI with Rune)
    #[arg(long)]
    pub desktop: bool,

    /// Local persistence layer (none, sqlite)
    #[arg(long)]
    pub storage: Option<String>,

    /// Enable embedded SQLite via rusqlite
    #[arg(long)]
    pub sqlite: bool,

    /// Android build system (only gradle; amper is no longer supported)
    #[arg(long)]
    pub android_build_system: Option<String>,

    /// Deprecated: JetBrains Amper is no longer supported (the command fails explaining why)
    #[arg(long)]
    pub amper: bool,

    /// Initial UI theme (default, material3-extended, minimal)
    #[arg(long)]
    pub theme: Option<String>,

    /// Optional architecture plugins (e.g. --plugin logger)
    #[arg(long)]
    pub plugin: Vec<String>,

    /// App template (starter, vault)
    #[arg(long)]
    pub template: Option<String>,

    /// Generate a GitHub Actions CI workflow (.github/workflows/ci.yml)
    #[arg(long)]
    pub ci: bool,

    /// Never prompt (use defaults for missing values)
    #[arg(long)]
    pub non_interactive: bool,
}

#[derive(Args, Debug, Default)]
pub struct RunArgs {
    /// Target platform (android, ios, desktop)
    pub platform: Option<String>,

    /// Device/emulator ID or serial (skips selection)
    #[arg(short, long)]
    pub device: Option<String>,

    /// Disable watch mode (no redeploy on code changes)
    #[arg(long)]
    pub no_watch: bool,
}

#[derive(Args, Debug, Default)]
pub struct TestArgs {
    /// Test target (core, ffi, android, ios, desktop, all)
    pub target: Option<String>,

    /// Run every suite on every available platform
    #[arg(long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Print the diagnosis as JSON (ideal for CI)
    #[arg(long)]
    pub json: bool,

    /// Profile the Gradle build (--profile) to find slow steps
    #[arg(long)]
    pub profile: bool,
}

#[derive(Args, Debug)]
pub struct DevArgs {
    /// Target platform (android, ios, desktop)
    pub platform: Option<String>,

    /// Disable watch mode (no redeploy on code changes)
    #[arg(long)]
    pub no_watch: bool,
}

#[derive(Args, Debug)]
pub struct BindingsArgs {
    /// Platform (android, ios; omit for both)
    pub platform: Option<String>,
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Target platform (android, ios, desktop; omit for all)
    pub platform: Option<String>,

    /// Optimized release build
    #[arg(long, default_value_t = true)]
    pub release: bool,
}
