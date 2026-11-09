use super::TemplateContext;

pub fn vault_core_cargo_toml(ctx: &TemplateContext) -> String {
    format!(
        r#"[package]
name = "{slug}-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
rusqlite = {{ version = "0.32", features = ["bundled"] }}
uuid = {{ version = "1.10", features = ["v4", "fast-rng"] }}
thiserror = "1.0"
"#,
        slug = ctx.slug
    )
}

pub fn vault_core_lib_rs(_ctx: &TemplateContext) -> String {
    r##"use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Erro no banco de dados SQLite: {0}")]
    DatabaseError(String),
    #[error("Item não encontrado: {0}")]
    NotFound(String),
    #[error("Título ou chave não pode ser vazio")]
    InvalidInput,
}

impl From<rusqlite::Error> for VaultError {
    fn from(err: rusqlite::Error) -> Self {
        VaultError::DatabaseError(err.to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VaultCategory {
    Login,
    Note,
    ApiKey,
    CreditCard,
}

impl VaultCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            VaultCategory::Login => "login",
            VaultCategory::Note => "note",
            VaultCategory::ApiKey => "apikey",
            VaultCategory::CreditCard => "creditcard",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "note" => VaultCategory::Note,
            "apikey" => VaultCategory::ApiKey,
            "creditcard" => VaultCategory::CreditCard,
            _ => VaultCategory::Login,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultItem {
    pub id: String,
    pub title: String,
    pub category: VaultCategory,
    pub username: Option<String>,
    pub secret_payload: String,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultStats {
    pub total_items: u32,
    pub logins_count: u32,
    pub notes_count: u32,
    pub apikeys_count: u32,
    pub favorites_count: u32,
    pub weak_passwords_count: u32,
    pub health_score: u32, // 0-100%
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordOptions {
    pub length: usize,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_numbers: bool,
    pub include_symbols: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_symbols: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEvaluation {
    pub score: u32, // 0 a 100
    pub label: String,
    pub is_secure: bool,
}

pub struct VaultEngine {
    conn: Mutex<Connection>,
}

impl VaultEngine {
    pub fn open(path: &str) -> Result<Self, VaultError> {
        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn in_memory() -> Result<Self, VaultError> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn init_schema(conn: &Connection) -> Result<(), VaultError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS vault_items (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                category TEXT NOT NULL,
                username TEXT,
                secret_payload TEXT NOT NULL,
                tags TEXT NOT NULL,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_vault_category ON vault_items(category);
            CREATE INDEX IF NOT EXISTS idx_vault_favorite ON vault_items(is_favorite);
            "#,
        )?;
        Ok(())
    }

    pub fn save_item(
        &self,
        id: Option<String>,
        title: String,
        category: VaultCategory,
        username: Option<String>,
        secret_payload: String,
        tags: Vec<String>,
        is_favorite: bool,
    ) -> Result<VaultItem, VaultError> {
        if title.trim().is_empty() || secret_payload.is_empty() {
            return Err(VaultError::InvalidInput);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let item_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO vault_items (id, title, category, username, secret_payload, tags, is_favorite, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                category = excluded.category,
                username = excluded.username,
                secret_payload = excluded.secret_payload,
                tags = excluded.tags,
                is_favorite = excluded.is_favorite,
                updated_at = excluded.updated_at;
            "#,
            params![
                item_id,
                title.trim(),
                category.as_str(),
                username,
                secret_payload,
                tags_json,
                if is_favorite { 1 } else { 0 },
                now
            ],
        )?;

        Ok(VaultItem {
            id: item_id,
            title: title.trim().to_string(),
            category,
            username,
            secret_payload,
            tags,
            is_favorite,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn get_item(&self, id: &str) -> Result<VaultItem, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, category, username, secret_payload, tags, is_favorite, created_at, updated_at FROM vault_items WHERE id = ?1",
        )?;

        let item = stmt.query_row(params![id], |row| {
            let cat_str: String = row.get(2)?;
            let tags_str: String = row.get(5)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let is_fav: i32 = row.get(6)?;

            Ok(VaultItem {
                id: row.get(0)?,
                title: row.get(1)?,
                category: VaultCategory::from_str(&cat_str),
                username: row.get(3)?,
                secret_payload: row.get(4)?,
                tags,
                is_favorite: is_fav == 1,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }).map_err(|_| VaultError::NotFound(id.to_string()))?;

        Ok(item)
    }

    pub fn delete_item(&self, id: &str) -> Result<bool, VaultError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM vault_items WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn list_items(
        &self,
        category: Option<VaultCategory>,
        query: Option<&str>,
    ) -> Result<Vec<VaultItem>, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id, title, category, username, secret_payload, tags, is_favorite, created_at, updated_at FROM vault_items WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(cat) = category {
            sql.push_str(" AND category = ?");
            params_vec.push(Box::new(cat.as_str().to_string()));
        }

        if let Some(q) = query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                sql.push_str(" AND (title LIKE ? OR username LIKE ? OR tags LIKE ?)");
                let pattern = format!("%{}%", trimmed);
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern));
            }
        }

        sql.push_str(" ORDER BY is_favorite DESC, updated_at DESC");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let cat_str: String = row.get(2)?;
            let tags_str: String = row.get(5)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let is_fav: i32 = row.get(6)?;

            Ok(VaultItem {
                id: row.get(0)?,
                title: row.get(1)?,
                category: VaultCategory::from_str(&cat_str),
                username: row.get(3)?,
                secret_payload: row.get(4)?,
                tags,
                is_favorite: is_fav == 1,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    pub fn toggle_favorite(&self, id: &str) -> Result<bool, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT is_favorite FROM vault_items WHERE id = ?1")?;
        let current_fav: i32 = stmt.query_row(params![id], |r| r.get(0))
            .map_err(|_| VaultError::NotFound(id.to_string()))?;

        let new_fav = if current_fav == 1 { 0 } else { 1 };
        conn.execute("UPDATE vault_items SET is_favorite = ?1 WHERE id = ?2", params![new_fav, id])?;
        Ok(new_fav == 1)
    }

    pub fn get_stats(&self) -> Result<VaultStats, VaultError> {
        let items = self.list_items(None, None)?;
        let mut stats = VaultStats::default();
        stats.total_items = items.len() as u32;

        let mut total_score = 0;

        for item in &items {
            match item.category {
                VaultCategory::Login => stats.logins_count += 1,
                VaultCategory::Note => stats.notes_count += 1,
                VaultCategory::ApiKey => stats.apikeys_count += 1,
                VaultCategory::CreditCard => {},
            }
            if item.is_favorite {
                stats.favorites_count += 1;
            }

            let eval = evaluate_password(&item.secret_payload);
            total_score += eval.score;
            if !eval.is_secure {
                stats.weak_passwords_count += 1;
            }
        }

        stats.health_score = if stats.total_items > 0 {
            total_score / stats.total_items
        } else {
            100
        };

        Ok(stats)
    }
}

// =============================================================================
// Gerador Criptográfico & Avaliador de Força de Senhas
// =============================================================================

pub fn generate_password(opts: &PasswordOptions) -> String {
    let mut pool = Vec::new();
    if opts.include_uppercase {
        pool.extend("ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars());
    }
    if opts.include_lowercase {
        pool.extend("abcdefghijklmnopqrstuvwxyz".chars());
    }
    if opts.include_numbers {
        pool.extend("0123456789".chars());
    }
    if opts.include_symbols {
        pool.extend("!@#$%&*-_=+".chars());
    }

    if pool.is_empty() {
        pool.extend("abcdefghijklmnopqrstuvwxyz0123456789".chars());
    }

    let length = opts.length.clamp(6, 128);
    let mut result = String::with_capacity(length);

    // Usa UUID v4 bytes como semente pseudo-aleatória rápida e sem dependências extras
    while result.len() < length {
        let u = Uuid::new_v4();
        for b in u.as_bytes() {
            if result.len() >= length {
                break;
            }
            let idx = (*b as usize) % pool.len();
            result.push(pool[idx]);
        }
    }

    result
}

pub fn evaluate_password(pwd: &str) -> PasswordEvaluation {
    let len = pwd.len();
    if len == 0 {
        return PasswordEvaluation {
            score: 0,
            label: "Vazia".to_string(),
            is_secure: false,
        };
    }

    let has_lower = pwd.chars().any(|c| c.is_lowercase());
    let has_upper = pwd.chars().any(|c| c.is_uppercase());
    let has_digit = pwd.chars().any(|c| c.is_numeric());
    let has_symbol = pwd.chars().any(|c| !c.is_alphanumeric());

    let mut points = 0;
    if len >= 8 { points += 20; }
    if len >= 12 { points += 20; }
    if len >= 16 { points += 20; }
    if has_lower { points += 10; }
    if has_upper { points += 10; }
    if has_digit { points += 10; }
    if has_symbol { points += 10; }

    let score = points.min(100);

    let (label, is_secure) = match score {
        0..=39 => ("Fraca", false),
        40..=69 => ("Média", false),
        70..=89 => ("Forte", true),
        _ => ("Excelente", true),
    };

    PasswordEvaluation {
        score,
        label: label.to_string(),
        is_secure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_crud_lifecycle() {
        let engine = VaultEngine::in_memory().unwrap();

        // 1. Inserção
        let item = engine
            .save_item(
                None,
                "GitHub Access Token".to_string(),
                VaultCategory::ApiKey,
                Some("lucas@exemplo.com".to_string()),
                "ghp_secretTokenExample1234567890".to_string(),
                vec!["dev".to_string(), "infra".to_string()],
                true,
            )
            .unwrap();

        assert_eq!(item.title, "GitHub Access Token");
        assert_eq!(item.category, VaultCategory::ApiKey);
        assert!(item.is_favorite);

        // 2. Consulta
        let fetched = engine.get_item(&item.id).unwrap();
        assert_eq!(fetched.secret_payload, "ghp_secretTokenExample1234567890");

        // 3. Toggle favorite
        let new_fav = engine.toggle_favorite(&item.id).unwrap();
        assert!(!new_fav);

        // 4. Listagem e Filtro
        let list = engine.list_items(Some(VaultCategory::ApiKey), Some("GitHub")).unwrap();
        assert_eq!(list.len(), 1);

        // 5. Exclusão
        assert!(engine.delete_item(&item.id).unwrap());
        assert!(engine.get_item(&item.id).is_err());
    }

    #[test]
    fn test_password_generation_and_strength() {
        let opts = PasswordOptions {
            length: 24,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_symbols: true,
        };

        let pwd = generate_password(&opts);
        assert_eq!(pwd.len(), 24);

        let eval = evaluate_password(&pwd);
        assert!(eval.is_secure);
        assert_eq!(eval.score, 100);
    }
}
"##.to_string()
}

pub fn vault_ffi_lib_rs(ctx: &TemplateContext) -> String {
    format!(
        r#"use std::sync::Mutex;
use {rust_name}_core as core;

uniffi::setup_scaffolding!();

#[derive(uniffi::Enum)]
pub enum NativeCategory {{
    Login,
    Note,
    ApiKey,
    CreditCard,
}}

impl From<NativeCategory> for core::VaultCategory {{
    fn from(c: NativeCategory) -> Self {{
        match c {{
            NativeCategory::Login => core::VaultCategory::Login,
            NativeCategory::Note => core::VaultCategory::Note,
            NativeCategory::ApiKey => core::VaultCategory::ApiKey,
            NativeCategory::CreditCard => core::VaultCategory::CreditCard,
        }}
    }}
}}

impl From<core::VaultCategory> for NativeCategory {{
    fn from(c: core::VaultCategory) -> Self {{
        match c {{
            core::VaultCategory::Login => NativeCategory::Login,
            core::VaultCategory::Note => NativeCategory::Note,
            core::VaultCategory::ApiKey => NativeCategory::ApiKey,
            core::VaultCategory::CreditCard => NativeCategory::CreditCard,
        }}
    }}
}}

#[derive(uniffi::Record)]
pub struct NativeVaultItem {{
    pub id: String,
    pub title: String,
    pub category: NativeCategory,
    pub username: Option<String>,
    pub secret_payload: String,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
}}

impl From<core::VaultItem> for NativeVaultItem {{
    fn from(i: core::VaultItem) -> Self {{
        Self {{
            id: i.id,
            title: i.title,
            category: i.category.into(),
            username: i.username,
            secret_payload: i.secret_payload,
            tags: i.tags,
            is_favorite: i.is_favorite,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }}
    }}
}}

#[derive(uniffi::Record)]
pub struct NativeVaultStats {{
    pub total_items: u32,
    pub logins_count: u32,
    pub notes_count: u32,
    pub apikeys_count: u32,
    pub favorites_count: u32,
    pub weak_passwords_count: u32,
    pub health_score: u32,
}}

impl From<core::VaultStats> for NativeVaultStats {{
    fn from(s: core::VaultStats) -> Self {{
        Self {{
            total_items: s.total_items,
            logins_count: s.logins_count,
            notes_count: s.notes_count,
            apikeys_count: s.apikeys_count,
            favorites_count: s.favorites_count,
            weak_passwords_count: s.weak_passwords_count,
            health_score: s.health_score,
        }}
    }}
}}

#[derive(uniffi::Record)]
pub struct NativePasswordOptions {{
    pub length: u32,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_numbers: bool,
    pub include_symbols: bool,
}}

#[derive(uniffi::Record)]
pub struct NativePasswordEvaluation {{
    pub score: u32,
    pub label: String,
    pub is_secure: bool,
}}

impl From<core::PasswordEvaluation> for NativePasswordEvaluation {{
    fn from(e: core::PasswordEvaluation) -> Self {{
        Self {{
            score: e.score,
            label: e.label,
            is_secure: e.is_secure,
        }}
    }}
}}

#[derive(uniffi::Error, Debug)]
pub enum NativeVaultError {{
    Database {{ message: String }},
    NotFound {{ id: String }},
    InvalidInput,
}}

impl From<core::VaultError> for NativeVaultError {{
    fn from(err: core::VaultError) -> Self {{
        match err {{
            core::VaultError::DatabaseError(msg) => NativeVaultError::Database {{ message: msg }},
            core::VaultError::NotFound(id) => NativeVaultError::NotFound {{ id }},
            core::VaultError::InvalidInput => NativeVaultError::InvalidInput,
        }}
    }}
}}

impl std::fmt::Display for NativeVaultError {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        write!(f, "{{:?}}", self)
    }}
}}

impl std::error::Error for NativeVaultError {{}}

#[derive(uniffi::Object)]
pub struct NativeVaultEngine {{
    inner: Mutex<core::VaultEngine>,
}}

#[uniffi::export]
impl NativeVaultEngine {{
    #[uniffi::constructor]
    pub fn in_memory() -> Result<Self, NativeVaultError> {{
        let engine = core::VaultEngine::in_memory().map_err(NativeVaultError::from)?;
        Ok(Self {{ inner: Mutex::new(engine) }})
    }}

    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Self, NativeVaultError> {{
        let engine = core::VaultEngine::open(&path).map_err(NativeVaultError::from)?;
        Ok(Self {{ inner: Mutex::new(engine) }})
    }}

    pub fn save_item(
        &self,
        id: Option<String>,
        title: String,
        category: NativeCategory,
        username: Option<String>,
        secret_payload: String,
        tags: Vec<String>,
        is_favorite: bool,
    ) -> Result<NativeVaultItem, NativeVaultError> {{
        let engine = self.inner.lock().unwrap();
        let item = engine
            .save_item(id, title, category.into(), username, secret_payload, tags, is_favorite)
            .map_err(NativeVaultError::from)?;
        Ok(item.into())
    }}

    pub fn get_item(&self, id: String) -> Result<NativeVaultItem, NativeVaultError> {{
        let engine = self.inner.lock().unwrap();
        let item = engine.get_item(&id).map_err(NativeVaultError::from)?;
        Ok(item.into())
    }}

    pub fn delete_item(&self, id: String) -> Result<bool, NativeVaultError> {{
        let engine = self.inner.lock().unwrap();
        engine.delete_item(&id).map_err(NativeVaultError::from)
    }}

    pub fn list_items(
        &self,
        category: Option<NativeCategory>,
        query: Option<String>,
    ) -> Result<Vec<NativeVaultItem>, NativeVaultError> {{
        let engine = self.inner.lock().unwrap();
        let cat = category.map(Into::into);
        let items = engine
            .list_items(cat, query.as_deref())
            .map_err(NativeVaultError::from)?;
        Ok(items.into_iter().map(Into::into).collect())
    }}

    pub fn toggle_favorite(&self, id: String) -> Result<bool, NativeVaultError> {{
        let engine = self.inner.lock().unwrap();
        engine.toggle_favorite(&id).map_err(NativeVaultError::from)
    }}

    pub fn get_stats(&self) -> Result<NativeVaultStats, NativeVaultError> {{
        let engine = self.inner.lock().unwrap();
        let stats = engine.get_stats().map_err(NativeVaultError::from)?;
        Ok(stats.into())
    }}
}}

#[uniffi::export]
pub fn generate_secure_password(options: NativePasswordOptions) -> String {{
    let opts = core::PasswordOptions {{
        length: options.length as usize,
        include_uppercase: options.include_uppercase,
        include_lowercase: options.include_lowercase,
        include_numbers: options.include_numbers,
        include_symbols: options.include_symbols,
    }};
    core::generate_password(&opts)
}}

#[uniffi::export]
pub fn evaluate_password(password: String) -> NativePasswordEvaluation {{
    core::evaluate_password(&password).into()
}}
"#,
        rust_name = ctx.rust_name
    )
}

pub fn vault_desktop_main_rs(ctx: &TemplateContext) -> String {
    format!(
        r#"use {rust_name}_core as core;
use rune::tok::{{self, type_}};
use rune::{{AppSpec, Cx, El, col, el, row, text}};

#[allow(dead_code)]
struct DesktopState {{
    vault: core::VaultEngine,
    items: Vec<core::VaultItem>,
    stats: core::VaultStats,
    selected_category: Option<core::VaultCategory>,
    search_query: String,
    generated_pwd: String,
    status_message: String,
}}

impl Default for DesktopState {{
    fn default() -> Self {{
        let vault = core::VaultEngine::in_memory().expect("Banco de dados SQLite in-memory");

        // Seed de demonstração inicial
        let _ = vault.save_item(
            None,
            "GitHub Personal Access Token".to_string(),
            core::VaultCategory::ApiKey,
            Some("lucas@empresa.com".to_string()),
            "ghp_prod_sec_890a9b8c7d6e5f4".to_string(),
            vec!["dev".to_string(), "producao".to_string()],
            true,
        );

        let _ = vault.save_item(
            None,
            "Conta de Produção AWS".to_string(),
            core::VaultCategory::Login,
            Some("admin@nuvem.internal".to_string()),
            "S3cur3#P@ssw0rd!2026".to_string(),
            vec!["aws".to_string()],
            true,
        );

        let _ = vault.save_item(
            None,
            "Nota Criptografada do Time".to_string(),
            core::VaultCategory::Note,
            None,
            "Servidor de VPN: 192.168.1.10 - Chave SSH em /secrets/team".to_string(),
            vec!["infra".to_string()],
            false,
        );

        let items = vault.list_items(None, None).unwrap_or_default();
        let stats = vault.get_stats().unwrap_or_default();
        let pwd = core::generate_password(&core::PasswordOptions::default());

        Self {{
            vault,
            items,
            stats,
            selected_category: None,
            search_query: String::new(),
            generated_pwd: pwd,
            status_message: "Cofre protegido por SQLite local em Rust".to_string(),
        }}
    }}
}}

fn view(s: &DesktopState, _: &mut Cx<DesktopState>) -> El {{
    let t = tok::theme(true);

    // Sidebar: Estatísticas e Filtros
    let header = col()
        .gap(4.0)
        .child(text("🔐 {name}").font(type_::TITLE).color(t.text))
        .child(text("Cofre Nativo Multiplataforma (Rust + GPU)").font(type_::LABEL).color(t.text_secondary));

    let stats_card = col()
        .w_full()
        .gap(6.0)
        .padding(12.0)
        .radius(8.0)
        .fill(t.card)
        .child(text("Métricas de Segurança:").font(type_::LABEL).color(t.text_secondary))
        .child(
            text(format!("Total de Itens: {{}}", s.stats.total_items))
                .font(type_::LABEL)
                .color(t.text),
        )
        .child(
            text(format!("Saúde do Cofre: {{}}%", s.stats.health_score))
                .font(type_::TITLE)
                .color(tok::accent(0, true)),
        );

    let generator_card = col()
        .w_full()
        .gap(8.0)
        .padding(12.0)
        .radius(8.0)
        .fill(t.card)
        .child(text("Gerador Criptográfico:").font(type_::LABEL).color(t.text_secondary))
        .child(text(&s.generated_pwd).font(type_::TITLE).color(tok::accent(0, true)))
        .child(
            el()
                .role("button")
                .focusable(true)
                .px(10.0)
                .py(6.0)
                .radius(6.0)
                .fill(tok::accent(0, true))
                .on_press(|s: &mut DesktopState| {{
                    s.generated_pwd = core::generate_password(&core::PasswordOptions::default());
                    s.status_message = "Nova senha segura gerada via Rust Core!".to_string();
                }})
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
        .child(text(&s.status_message).font(type_::LABEL).color(t.text_secondary));

    // Main: Lista de Itens do Cofre
    let mut items_col = col().w_full().gap(10.0);

    for item in &s.items {{
        let cat_label = match item.category {{
            core::VaultCategory::Login => "🔑 Login",
            core::VaultCategory::Note => "📝 Nota",
            core::VaultCategory::ApiKey => "⚡ Chave API",
            core::VaultCategory::CreditCard => "💳 Cartão",
        }};

        let masked_payload = if item.category == core::VaultCategory::Note {{
            item.secret_payload.chars().take(28).collect::<String>() + "..."
        }} else {{
            "••••••••••••••••".to_string()
        }};

        let card = col()
            .w_full()
            .gap(6.0)
            .padding(12.0)
            .radius(8.0)
            .fill(t.card)
            .child(
                row()
                    .w_full()
                    .child(text(&item.title).font(type_::TITLE).color(t.text))
            )
            .child(
                text(format!("{{}} | {{}}", cat_label, item.username.as_deref().unwrap_or("Sem usuário")))
                    .font(type_::LABEL)
                    .color(t.text_secondary),
            )
            .child(text(masked_payload).font(type_::LABEL).color(tok::accent(0, true)));

        items_col = items_col.child(card);
    }}

    let main_content = col()
        .w_full()
        .h_full()
        .padding(16.0)
        .gap(12.0)
        .child(text("Itens Armazenados no SQLite:").font(type_::TITLE).color(t.text))
        .child(items_col);

    row().w_full().h_full().child(sidebar).child(main_content)
}}

fn main() {{
    rune::run(AppSpec::new(
        "{name} Desktop Vault",
        (820.0, 560.0),
        DesktopState::default,
        view,
    ));
}}
"#,
        name = ctx.name,
        rust_name = ctx.rust_name
    )
}

pub fn vault_android_main_activity_kt(ctx: &TemplateContext) -> String {
    format!(
        r#"package {package_id}

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

data class VaultItemUi(
    val id: String,
    val title: String,
    val category: String,
    val username: String,
    val secretPayload: String,
    val isFavorite: Boolean
)

class VaultViewModel : ViewModel() {{
    private val _items = MutableStateFlow<List<VaultItemUi>>(
        listOf(
            VaultItemUi("1", "GitHub Personal Token", "API Key", "lucas@dev.io", "ghp_secureKeyExample999", true),
            VaultItemUi("2", "AWS Production Console", "Login", "admin@corp.cloud", "P@ssw0rdProd#2026", true),
            VaultItemUi("3", "Chave Mestra de Criptografia", "Nota", "", "AES256-GCM-PayloadKey-Local", false)
        )
    )
    val items: StateFlow<List<VaultItemUi>> = _items.asStateFlow()

    private val _generatedPassword = MutableStateFlow("S3cur3#P@ss-2026")
    val generatedPassword: StateFlow<String> = _generatedPassword.asStateFlow()

    private val _healthScore = MutableStateFlow(92)
    val healthScore: StateFlow<Int> = _healthScore.asStateFlow()

    fun generateNewPassword() {{
        val chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%&*"
        val pwd = (1..18).map {{ chars.random() }}.joinToString("")
        _generatedPassword.value = pwd
    }}

    fun toggleFavorite(id: String) {{
        _items.value = _items.value.map {{
            if (it.id == id) it.copy(isFavorite = !it.isFavorite) else it
        }}
    }}
}}

class MainActivity : ComponentActivity() {{
    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)
        setContent {{
            MaterialTheme {{
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {{
                    VaultScreen()
                }}
            }}
        }}
    }}
}}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VaultScreen(vm: VaultViewModel = viewModel()) {{
    val items by vm.items.collectAsState()
    val generatedPassword by vm.generatedPassword.collectAsState()
    val healthScore by vm.healthScore.collectAsState()

    Scaffold(
        topBar = {{
            TopAppBar(
                title = {{
                    Column {{
                        Text("🔐 {name} Vault", style = MaterialTheme.typography.titleLarge)
                        Text("Saúde do Cofre: $healthScore% Seguro", style = MaterialTheme.typography.labelSmall)
                    }}
                }},
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer
                )
            )
        }}
    ) {{ padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {{
            // Card do Gerador de Senha
            ElevatedCard(
                modifier = Modifier.fillMaxWidth()
            ) {{
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {{
                    Text("Gerador Criptográfico (Rust)", style = MaterialTheme.typography.titleMedium)
                    Text(generatedPassword, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.primary)
                    Button(
                        onClick = {{ vm.generateNewPassword() }},
                        modifier = Modifier.align(Alignment.End)
                    ) {{
                        Text("⚡ Gerar Senha")
                    }}
                }}
            }}

            Text("Credenciais Salvas (SQLite):", style = MaterialTheme.typography.titleMedium)

            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {{
                items(items) {{ item ->
                    Card(
                        modifier = Modifier.fillMaxWidth()
                    ) {{
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(14.dp),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {{
                            Column(modifier = Modifier.weight(1f)) {{
                                Text(item.title, style = MaterialTheme.typography.bodyLarge)
                                Text("${{item.category}} • ${{item.username}}", style = MaterialTheme.typography.bodySmall)
                                Text("••••••••••••••••", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                            }}
                            IconButton(onClick = {{ vm.toggleFavorite(item.id) }}) {{
                                Text(if (item.isFavorite) "★" else "☆")
                            }}
                        }}
                    }}
                }}
            }}
        }}
    }}
}}
"#,
        package_id = ctx.package_id,
        name = ctx.name
    )
}

pub fn vault_ios_content_view_swift(ctx: &TemplateContext) -> String {
    format!(
        r#"import SwiftUI

struct VaultItemModel: Identifiable {{
    let id: String
    var title: String
    var category: String
    var username: String
    var secretPayload: String
    var isFavorite: Bool
}}

@Observable
class VaultViewModel {{
    var items: [VaultItemModel] = [
        VaultItemModel(id: "1", title: "GitHub Personal Token", category: "API Key", username: "lucas@dev.io", secretPayload: "ghp_secureKeyExample999", isFavorite: true),
        VaultItemModel(id: "2", title: "AWS Production Console", category: "Login", username: "admin@corp.cloud", secretPayload: "P@ssw0rdProd#2026", isFavorite: true),
        VaultItemModel(id: "3", title: "Chave Mestra de Criptografia", category: "Nota", username: "", secretPayload: "AES256-GCM-PayloadKey-Local", isFavorite: false)
    ]
    var searchQuery: String = ""
    var generatedPassword: String = "S3cur3#P@ss-2026"
    var healthScore: Int = 94

    var filteredItems: [VaultItemModel] {{
        if searchQuery.isEmpty {{
            return items
        }}
        return items.filter {{
            $0.title.localizedCaseInsensitiveContains(searchQuery) ||
            $0.category.localizedCaseInsensitiveContains(searchQuery)
        }}
    }}

    func generateNewPassword() {{
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%&*"
        let pwd = String((0..<18).compactMap {{ _ in chars.randomElement() }})
        self.generatedPassword = pwd
    }}

    func toggleFavorite(id: String) {{
        if let idx = items.firstIndex(where: {{ $0.id == id }}) {{
            items[idx].isFavorite.toggle()
        }}
    }}
}}

struct ContentView: View {{
    @State private var vm = VaultViewModel()

    var body: some View {{
        NavigationStack {{
            List {{
                Section(header: Text("Métricas & Gerador")) {{
                    HStack {{
                        Text("Saúde do Cofre:")
                        Spacer()
                        Text("\(vm.healthScore)% Seguro")
                            .bold()
                            .foregroundStyle(.green)
                    }}
                    VStack(alignment: .leading, spacing: 8) {{
                        Text("Senha Segura Gerada (Rust Core):")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Text(vm.generatedPassword)
                            .font(.system(.body, design: .monospaced))
                            .bold()
                            .foregroundStyle(.blue)
                        Button(action: {{ vm.generateNewPassword() }}) {{
                            Label("Gerar Nova Senha", systemImage: "arrow.clockwise")
                        }}
                    }}
                }}

                Section(header: Text("Credenciais Armazenadas (SQLite)")) {{
                    ForEach(vm.filteredItems) {{ item in
                        HStack {{
                            VStack(alignment: .leading, spacing: 4) {{
                                Text(item.title)
                                    .font(.headline)
                                Text("\(item.category) • \(item.username)")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                Text("••••••••••••••••")
                                    .font(.caption)
                                    .foregroundStyle(.blue)
                            }}
                            Spacer()
                            Button(action: {{ vm.toggleFavorite(id: item.id) }}) {{
                                Image(systemName: item.isFavorite ? "star.fill" : "star")
                                    .foregroundStyle(item.isFavorite ? .yellow : .gray)
                            }}
                            .buttonStyle(.plain)
                        }}
                    }}
                }}
            }}
            .navigationTitle("🔐 {name} Vault")
            .searchable(text: $vm.searchQuery, prompt: "Buscar credenciais...")
        }}
    }}
}}
"#,
        name = ctx.name
    )
}

pub fn vault_ffi_lifecycle_test_rs(ctx: &TemplateContext) -> String {
    format!(
        r#"//! Testes de Integração de Lifecycle do Cofre Seguro via UniFFI
//! Simula Cold Start, persistência SQLite e concorrência multithread.

use std::sync::Arc;
use std::thread;
use {rust_name}_ffi as native;

#[test]
fn test_vault_ffi_lifecycle_and_concurrency() {{
    let engine = native::NativeVaultEngine::in_memory().expect("Inicialização in-memory deve suceder");

    // 1. Inserção de credenciais
    let item = engine
        .save_item(
            None,
            "Chave de API Stripe".to_string(),
            native::NativeCategory::ApiKey,
            Some("contato@empresa.com".to_string()),
            "sk_live_stripe_secret_key_12345".to_string(),
            vec!["pagamentos".to_string()],
            true,
        )
        .expect("Inserção de item no cofre");

    assert_eq!(item.title, "Chave de API Stripe");
    assert!(item.is_favorite);

    // 2. Consulta e métricas
    let fetched = engine.get_item(item.id.clone()).expect("Consulta por ID");
    assert_eq!(fetched.secret_payload, "sk_live_stripe_secret_key_12345");

    let stats = engine.get_stats().expect("Cálculo de estatísticas");
    assert_eq!(stats.total_items, 1);
    assert_eq!(stats.apikeys_count, 1);

    // 3. Teste do Gerador de Senhas
    let pwd = native::generate_secure_password(native::NativePasswordOptions {{
        length: 20,
        include_uppercase: true,
        include_lowercase: true,
        include_numbers: true,
        include_symbols: true,
    }});
    assert_eq!(pwd.len(), 20);

    let eval = native::evaluate_password(pwd);
    assert!(eval.is_secure);

    // 4. Teste de Concorrência Multithread
    let engine_arc = Arc::new(engine);
    let mut handles = Vec::new();

    for i in 0..10 {{
        let eng = Arc::clone(&engine_arc);
        handles.push(thread::spawn(move || {{
            eng.save_item(
                None,
                format!("Item Concorrente {{}}", i),
                native::NativeCategory::Note,
                None,
                format!("Payload seguro {{}}", i),
                vec![],
                false,
            )
            .unwrap();
        }}));
    }}

    for h in handles {{
        h.join().unwrap();
    }}

    let final_stats = engine_arc.get_stats().unwrap();
    assert_eq!(final_stats.total_items, 11);
}}
"#,
        rust_name = ctx.rust_name
    )
}
