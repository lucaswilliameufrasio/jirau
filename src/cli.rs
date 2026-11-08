use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "jirau",
    author = "Lucas Eufrasio <lucaswilliameufrasio@users.noreply.github.com>",
    version,
    about = "🪵 CLI de scaffold nativo multiplataforma: Rust + Android (Compose) + iOS (SwiftUI)",
    long_about = "Jirau é uma CLI inspirada no ecossistema Flutter (create, doctor, run) que gera e opera projetos com núcleo compartilhado em Rust e UIs 100% nativas em Jetpack Compose e SwiftUI via UniFFI."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Cria um novo projeto a partir do template nativo
    New(NewArgs),

    /// Diagnostica o ambiente de desenvolvimento (Rust, Android SDK/NDK, Xcode, etc.)
    Doctor(DoctorArgs),

    /// Executa o ciclo de desenvolvimento rápido (compilação de ABI única, hot loop)
    Dev(DevArgs),

    /// Regera os bindings Kotlin e Swift a partir das anotações UniFFI
    Bindings(BindingsArgs),

    /// Compila binários finais de release (multi-ABI para Android, XCFramework para iOS)
    Build(BuildArgs),

    /// Adiciona uma nova plataforma a um projeto existente (ex: jirau add desktop)
    Add(AddArgs),

    /// Sincroniza e atualiza o template do projeto via 3-way merge
    Upgrade(UpgradeArgs),
}

#[derive(Args, Debug, Default)]
pub struct UpgradeArgs {
    /// Força o upgrade mesmo que o projeto já esteja na versão mais recente
    #[arg(short, long)]
    pub force: bool,

    /// Simula as alterações sem gravar nada em disco
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Plataforma para adicionar (desktop, android, ios)
    pub platform: Option<String>,
}

#[derive(Args, Debug)]
pub struct NewArgs {
    /// Nome do aplicativo/projeto
    pub name: Option<String>,

    /// Package ID / Bundle Identifier (ex: com.exemplo.meuapp)
    #[arg(short, long)]
    pub package: Option<String>,

    /// Inclui suporte para Android
    #[arg(long)]
    pub android: bool,

    /// Inclui suporte para iOS
    #[arg(long)]
    pub ios: bool,

    /// Inclui suporte para Desktop (UI em Rust com Rune)
    #[arg(long)]
    pub desktop: bool,

    /// Camada de persistência local (none, sqlite)
    #[arg(long)]
    pub storage: Option<String>,

    /// Ativa SQLite embutido via rusqlite
    #[arg(long)]
    pub sqlite: bool,

    /// Sistema de build do Android (gradle, amper)
    #[arg(long)]
    pub android_build_system: Option<String>,

    /// Usa JetBrains Amper como sistema de build do Android
    #[arg(long)]
    pub amper: bool,

    /// Não pergunta nada de forma interativa (usa padrões para valores ausentes)
    #[arg(long)]
    pub non_interactive: bool,
}

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Exporta o resultado do diagnóstico em formato JSON (ideal para CI/CD)
    #[arg(long)]
    pub json: bool,

    /// Executa profiling de build no Gradle (--profile) para diagnosticar gargalos de tempo
    #[arg(long)]
    pub profile: bool,
}

#[derive(Args, Debug)]
pub struct DevArgs {
    /// Plataforma alvo (android ou ios)
    pub platform: Option<String>,

    /// Desativa o monitoramento contínuo de alterações de código (watch mode)
    #[arg(long)]
    pub no_watch: bool,
}

#[derive(Args, Debug)]
pub struct BindingsArgs {
    /// Plataforma para gerar bindings (android, ios ou omitido para ambas)
    pub platform: Option<String>,
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Plataforma alvo (android, ios ou omitido para ambas)
    pub platform: Option<String>,

    /// Gera compilação otimizada de release
    #[arg(long, default_value_t = true)]
    pub release: bool,
}
