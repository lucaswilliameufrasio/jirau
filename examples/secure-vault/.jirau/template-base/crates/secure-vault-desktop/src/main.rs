use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, el, row, text};
use secure_vault_core as core;

#[allow(dead_code)]
struct DesktopState {
    vault: core::VaultEngine,
    items: Vec<core::VaultItem>,
    stats: core::VaultStats,
    selected_category: Option<core::VaultCategory>,
    search_query: String,
    generated_pwd: String,
    status_message: String,
}

impl Default for DesktopState {
    fn default() -> Self {
        let vault = core::VaultEngine::in_memory().expect("Banco de dados SQLite in-memory");

        // Seed de demonstração inicial
        let _ = vault.save_item(core::VaultItemInput {
            id: None,
            title: "GitHub Personal Access Token".to_string(),
            category: core::VaultCategory::ApiKey,
            username: Some("lucas@empresa.com".to_string()),
            secret_payload: "ghp_prod_sec_890a9b8c7d6e5f4".to_string(),
            tags: vec!["dev".to_string(), "producao".to_string()],
            is_favorite: true,
        });

        let _ = vault.save_item(core::VaultItemInput {
            id: None,
            title: "Conta de Produção AWS".to_string(),
            category: core::VaultCategory::Login,
            username: Some("admin@nuvem.internal".to_string()),
            secret_payload: "S3cur3#P@ssw0rd!2026".to_string(),
            tags: vec!["aws".to_string()],
            is_favorite: true,
        });

        let _ = vault.save_item(core::VaultItemInput {
            id: None,
            title: "Nota Criptografada do Time".to_string(),
            category: core::VaultCategory::Note,
            username: None,
            secret_payload: "Servidor de VPN: 192.168.1.10 - Chave SSH em /secrets/team"
                .to_string(),
            tags: vec!["infra".to_string()],
            is_favorite: false,
        });

        let items = vault.list_items(None, None).unwrap_or_default();
        let stats = vault.get_stats().unwrap_or_default();
        let pwd = core::generate_password(&core::PasswordOptions::default());

        Self {
            vault,
            items,
            stats,
            selected_category: None,
            search_query: String::new(),
            generated_pwd: pwd,
            status_message: "Cofre protegido por SQLite local em Rust".to_string(),
        }
    }
}

fn view(s: &DesktopState, _: &mut Cx<DesktopState>) -> El {
    let t = tok::theme(true);

    // Sidebar: Estatísticas e Filtros
    let header = col()
        .gap(4.0)
        .child(text("🔐 Secure Vault").font(type_::TITLE).color(t.text))
        .child(
            text("Cofre Nativo Multiplataforma (Rust + GPU)")
                .font(type_::LABEL)
                .color(t.text_secondary),
        );

    let stats_card = col()
        .w_full()
        .gap(6.0)
        .padding(12.0)
        .radius(8.0)
        .fill(t.card)
        .child(
            text("Métricas de Segurança:")
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
        .child(
            text(format!("Total de Itens: {}", s.stats.total_items))
                .font(type_::LABEL)
                .color(t.text),
        )
        .child(
            text(format!("Saúde do Cofre: {}%", s.stats.health_score))
                .font(type_::TITLE)
                .color(tok::accent(0, true)),
        );

    let generator_card = col()
        .w_full()
        .gap(8.0)
        .padding(12.0)
        .radius(8.0)
        .fill(t.card)
        .child(
            text("Gerador Criptográfico:")
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
        .child(
            text(&s.generated_pwd)
                .font(type_::TITLE)
                .color(tok::accent(0, true)),
        )
        .child(
            el().role("button")
                .focusable(true)
                .px(10.0)
                .py(6.0)
                .radius(6.0)
                .fill(tok::accent(0, true))
                .on_press(|s: &mut DesktopState| {
                    s.generated_pwd = core::generate_password(&core::PasswordOptions::default());
                    s.status_message = "Nova senha segura gerada via Rust Core!".to_string();
                })
                .child(text("⚡ Gerar Nova Senha").color(t.text)),
        );

    let sidebar = col()
        .w(220.0)
        .h_full()
        .gap(16.0)
        .padding(16.0)
        .fill(t.bg)
        .child(header)
        .child(stats_card)
        .child(generator_card)
        .child(
            text(&s.status_message)
                .font(type_::LABEL)
                .color(t.text_secondary),
        );

    // Main: Lista de Itens do Cofre
    let mut items_col = col().w_full().gap(10.0);

    for item in &s.items {
        let cat_label = match item.category {
            core::VaultCategory::Login => "🔑 Login",
            core::VaultCategory::Note => "📝 Nota",
            core::VaultCategory::ApiKey => "⚡ Chave API",
            core::VaultCategory::CreditCard => "💳 Cartão",
        };

        let masked_payload = if item.category == core::VaultCategory::Note {
            item.secret_payload.chars().take(28).collect::<String>() + "..."
        } else {
            "••••••••••••••••".to_string()
        };

        let card = col()
            .w_full()
            .gap(6.0)
            .padding(12.0)
            .radius(8.0)
            .fill(t.card)
            .child(
                row()
                    .w_full()
                    .child(text(&item.title).font(type_::TITLE).color(t.text)),
            )
            .child(
                text(format!(
                    "{} | {}",
                    cat_label,
                    item.username.as_deref().unwrap_or("Sem usuário")
                ))
                .font(type_::LABEL)
                .color(t.text_secondary),
            )
            .child(
                text(masked_payload)
                    .font(type_::LABEL)
                    .color(tok::accent(0, true)),
            );

        items_col = items_col.child(card);
    }

    let main_content = col()
        .w_full()
        .h_full()
        .padding(16.0)
        .gap(12.0)
        .child(
            text("Itens Armazenados no SQLite:")
                .font(type_::TITLE)
                .color(t.text),
        )
        .child(items_col);

    row().w_full().h_full().child(sidebar).child(main_content)
}

fn main() {
    rune::run(AppSpec::new(
        "Secure Vault Desktop Vault",
        (820.0, 560.0),
        DesktopState::default,
        view,
    ));
}
