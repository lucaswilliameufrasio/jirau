//! Template "vault" (cofre de senhas). As funções de conteúdo abaixo são
//! geradas a partir de examples/secure-vault (validado em simulador/emulador)
//! por um script de sincronização; edite o exemplo e re-sincronize.

use super::{TemplateContext, versions};

pub fn vault_core_cargo_toml(ctx: &TemplateContext) -> String {
    format!(
        r#"[package]
name = "{slug}-core"
version = "0.1.0"
edition = "{edition}"

[dependencies]
serde = {{ version = "{serde}", features = ["derive"] }}
serde_json = "{serde_json}"
rusqlite = {{ version = "{rusqlite}", features = ["bundled"] }}
uuid = {{ version = "{uuid}", features = ["v4", "fast-rng"] }}
thiserror = "{thiserror}"
aws-lc-rs = "{aws_lc_rs}"
"#,
        slug = ctx.slug,
        edition = versions::RUST_EDITION,
        serde = versions::SERDE,
        serde_json = versions::SERDE_JSON,
        rusqlite = versions::RUSQLITE,
        uuid = versions::UUID,
        thiserror = versions::THISERROR,
        aws_lc_rs = versions::AWS_LC_RS,
    )
}

/// Núcleo do cofre: SQLite + segredos cifrados (AES-256-GCM-SIV via aws-lc-rs).
pub fn vault_core_lib_rs(_ctx: &TemplateContext) -> String {
    r####"use aws_lc_rs::aead::{AES_256_GCM_SIV, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

/// Tamanho da chave do cofre (AES-256-GCM-SIV usa chaves de 256 bits).
pub const VAULT_KEY_LEN: usize = 32;
const TAG_LEN: usize = 16;
const SCHEMA_VERSION: i32 = 1;
const KEY_CHECK_ID: &str = "key_check";
const KEY_CHECK_PLAINTEXT: &[u8] = b"jirau-vault-key-check-v1";

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Erro no banco de dados SQLite: {0}")]
    DatabaseError(String),
    #[error("Item não encontrado: {0}")]
    NotFound(String),
    #[error("Título ou chave não pode ser vazio")]
    InvalidInput,
    #[error("Chave do cofre inválida: são necessários 32 bytes")]
    InvalidKey,
    #[error("Chave incorreta ou dados do cofre corrompidos")]
    WrongKeyOrCorrupted,
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

    pub fn from_db(s: &str) -> Self {
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

/// Gera uma chave aleatória para o cofre a partir do CSPRNG do sistema.
///
/// A chave nunca é gravada no banco: cada plataforma a guarda no seu cofre de
/// chaves nativo (Keychain no iOS, Android Keystore no Android).
pub fn generate_vault_key() -> Vec<u8> {
    let mut key = vec![0u8; VAULT_KEY_LEN];
    fill_random(&mut key);
    key
}

/// Linha do esquema antigo (segredo em texto puro), lida só na migração.
type LegacyRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    i32,
    i64,
    i64,
);

/// Linha do banco antes de o segredo ser decifrado.
struct StoredItem {
    id: String,
    title: String,
    category: String,
    username: Option<String>,
    secret_cipher: Vec<u8>,
    tags: String,
    is_favorite: bool,
    created_at: i64,
    updated_at: i64,
}

const SELECT_COLUMNS: &str = "SELECT id, title, category, username, secret_cipher, tags, is_favorite, created_at, updated_at FROM vault_items";

fn stored_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredItem> {
    let is_fav: i32 = row.get(6)?;
    Ok(StoredItem {
        id: row.get(0)?,
        title: row.get(1)?,
        category: row.get(2)?,
        username: row.get(3)?,
        secret_cipher: row.get(4)?,
        tags: row.get(5)?,
        is_favorite: is_fav == 1,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

/// Preenche `buf` com bytes do CSPRNG do aws-lc (semeado pelo sistema).
fn fill_random(buf: &mut [u8]) {
    aws_lc_rs::rand::fill(buf).expect("CSPRNG do sistema indisponível");
}

/// Cifra com AES-256-GCM-SIV (aws-lc-rs). Formato: `nonce (12 bytes) ||
/// ciphertext || tag`. O GCM-SIV tolera repetição acidental de nonce, o que
/// torna seguros os nonces aleatórios. O `aad` (id do item) amarra o segredo à
/// sua linha: copiar o blob para outro item faz a decifragem falhar.
fn seal(key: &LessSafeKey, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, VaultError> {
    let mut nonce = [0u8; NONCE_LEN];
    fill_random(&mut nonce);
    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(aad),
        &mut in_out,
    )
    .map_err(|_| VaultError::WrongKeyOrCorrupted)?;
    let mut out = Vec::with_capacity(NONCE_LEN + in_out.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&in_out);
    Ok(out)
}

fn unseal(key: &LessSafeKey, blob: &[u8], aad: &[u8]) -> Result<Vec<u8>, VaultError> {
    if blob.len() < NONCE_LEN + TAG_LEN {
        return Err(VaultError::WrongKeyOrCorrupted);
    }
    let (nonce, ciphertext) = blob.split_at(NONCE_LEN);
    let nonce =
        Nonce::try_assume_unique_for_key(nonce).map_err(|_| VaultError::WrongKeyOrCorrupted)?;
    let mut in_out = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::from(aad), &mut in_out)
        .map_err(|_| VaultError::WrongKeyOrCorrupted)?;
    Ok(plaintext.to_vec())
}

/// Dados para criar (`id: None`) ou atualizar (`id: Some`) um item do cofre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultItemInput {
    pub id: Option<String>,
    pub title: String,
    pub category: VaultCategory,
    pub username: Option<String>,
    pub secret_payload: String,
    pub tags: Vec<String>,
    pub is_favorite: bool,
}

/// Cofre local: metadados (título, usuário, tags) ficam pesquisáveis no SQLite;
/// o segredo de cada item é cifrado com a chave do cofre antes de ser gravado.
pub struct VaultEngine {
    conn: Mutex<Connection>,
    cipher: LessSafeKey,
}

impl VaultEngine {
    /// Abre (ou cria) o cofre em `path`. A mesma `key` precisa ser usada em
    /// todas as aberturas; uma chave diferente é recusada com
    /// [`VaultError::WrongKeyOrCorrupted`].
    pub fn open(path: &str, key: &[u8]) -> Result<Self, VaultError> {
        Self::from_connection(Connection::open(path)?, key)
    }

    /// Cofre efêmero (testes e demos), com uma chave aleatória descartável.
    pub fn in_memory() -> Result<Self, VaultError> {
        Self::from_connection(Connection::open_in_memory()?, &generate_vault_key())
    }

    fn from_connection(mut conn: Connection, key: &[u8]) -> Result<Self, VaultError> {
        if key.len() != VAULT_KEY_LEN {
            return Err(VaultError::InvalidKey);
        }
        let cipher = LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM_SIV, key).map_err(|_| VaultError::InvalidKey)?,
        );
        // Sobrescreve com zeros o conteúdo de linhas removidas/atualizadas.
        conn.pragma_update(None, "secure_delete", "ON")?;
        Self::init_schema(&mut conn, &cipher)?;
        Ok(Self {
            conn: Mutex::new(conn),
            cipher,
        })
    }

    fn init_schema(conn: &mut Connection, cipher: &LessSafeKey) -> Result<(), VaultError> {
        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        let has_items_table: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'vault_items')",
            [],
            |r| r.get(0),
        )?;
        let legacy_plaintext = version == 0 && has_items_table;

        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS vault_meta (id TEXT PRIMARY KEY, value BLOB NOT NULL);",
        )?;

        // A verificação da chave vem antes de qualquer escrita: abrir o cofre
        // com a chave errada não pode gravar dados que ninguém conseguirá ler.
        let key_check: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM vault_meta WHERE id = ?1",
                params![KEY_CHECK_ID],
                |r| r.get(0),
            )
            .optional()?;
        match key_check {
            Some(blob) => {
                if unseal(cipher, &blob, KEY_CHECK_ID.as_bytes())? != KEY_CHECK_PLAINTEXT {
                    return Err(VaultError::WrongKeyOrCorrupted);
                }
            }
            None => {
                let blob = seal(cipher, KEY_CHECK_PLAINTEXT, KEY_CHECK_ID.as_bytes())?;
                tx.execute(
                    "INSERT INTO vault_meta (id, value) VALUES (?1, ?2)",
                    params![KEY_CHECK_ID, blob],
                )?;
            }
        }

        if legacy_plaintext {
            // Versões antigas do template gravavam o segredo em texto puro.
            tx.execute_batch("ALTER TABLE vault_items RENAME TO vault_items_plaintext;")?;
        }

        tx.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS vault_items (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                category TEXT NOT NULL,
                username TEXT,
                secret_cipher BLOB NOT NULL,
                tags TEXT NOT NULL,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )?;

        if legacy_plaintext {
            let rows: Vec<LegacyRow> = {
                let mut stmt = tx.prepare(
                    "SELECT id, title, category, username, secret_payload, tags, is_favorite, created_at, updated_at FROM vault_items_plaintext",
                )?;
                let mapped = stmt.query_map([], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                    ))
                })?;
                mapped.collect::<Result<_, _>>()?
            };
            for (id, title, category, username, secret, tags, fav, created, updated) in rows {
                let blob = seal(cipher, secret.as_bytes(), id.as_bytes())?;
                tx.execute(
                    "INSERT INTO vault_items (id, title, category, username, secret_cipher, tags, is_favorite, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![id, title, category, username, blob, tags, fav, created, updated],
                )?;
            }
            tx.execute_batch("DROP TABLE vault_items_plaintext;")?;
        }

        tx.execute_batch(&format!(
            r#"
            CREATE INDEX IF NOT EXISTS idx_vault_category ON vault_items(category);
            CREATE INDEX IF NOT EXISTS idx_vault_favorite ON vault_items(is_favorite);
            PRAGMA user_version = {SCHEMA_VERSION};
            "#
        ))?;
        tx.commit()?;

        if legacy_plaintext {
            // Reescreve o arquivo para não deixar o texto puro em páginas livres.
            conn.execute_batch("VACUUM;")?;
        }
        Ok(())
    }

    fn decrypt(&self, stored: StoredItem) -> Result<VaultItem, VaultError> {
        let secret = unseal(&self.cipher, &stored.secret_cipher, stored.id.as_bytes())?;
        let secret_payload =
            String::from_utf8(secret).map_err(|_| VaultError::WrongKeyOrCorrupted)?;
        Ok(VaultItem {
            category: VaultCategory::from_db(&stored.category),
            tags: serde_json::from_str(&stored.tags).unwrap_or_default(),
            id: stored.id,
            title: stored.title,
            username: stored.username,
            secret_payload,
            is_favorite: stored.is_favorite,
            created_at: stored.created_at,
            updated_at: stored.updated_at,
        })
    }

    pub fn save_item(&self, input: VaultItemInput) -> Result<VaultItem, VaultError> {
        let VaultItemInput {
            id,
            title,
            category,
            username,
            secret_payload,
            tags,
            is_favorite,
        } = input;
        if title.trim().is_empty() || secret_payload.is_empty() {
            return Err(VaultError::InvalidInput);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let item_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        let secret_cipher = seal(&self.cipher, secret_payload.as_bytes(), item_id.as_bytes())?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO vault_items (id, title, category, username, secret_cipher, tags, is_favorite, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                category = excluded.category,
                username = excluded.username,
                secret_cipher = excluded.secret_cipher,
                tags = excluded.tags,
                is_favorite = excluded.is_favorite,
                updated_at = excluded.updated_at;
            "#,
            params![
                item_id,
                title.trim(),
                category.as_str(),
                username,
                secret_cipher,
                tags_json,
                if is_favorite { 1 } else { 0 },
                now
            ],
        )?;
        let created_at: i64 = conn.query_row(
            "SELECT created_at FROM vault_items WHERE id = ?1",
            params![item_id],
            |r| r.get(0),
        )?;

        Ok(VaultItem {
            id: item_id,
            title: title.trim().to_string(),
            category,
            username,
            secret_payload,
            tags,
            is_favorite,
            created_at,
            updated_at: now,
        })
    }

    pub fn get_item(&self, id: &str) -> Result<VaultItem, VaultError> {
        let stored = {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                &format!("{SELECT_COLUMNS} WHERE id = ?1"),
                params![id],
                stored_item_from_row,
            )
            .optional()?
        };
        match stored {
            Some(stored) => self.decrypt(stored),
            None => Err(VaultError::NotFound(id.to_string())),
        }
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
        let mut sql = format!("{SELECT_COLUMNS} WHERE 1=1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(cat) = category {
            sql.push_str(" AND category = ?");
            params_vec.push(Box::new(cat.as_str().to_string()));
        }

        // A busca cobre só metadados: o segredo cifrado nunca é pesquisável.
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

        let stored: Vec<StoredItem> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(param_refs.as_slice(), stored_item_from_row)?;
            rows.collect::<Result<_, _>>()?
        };

        stored.into_iter().map(|s| self.decrypt(s)).collect()
    }

    pub fn toggle_favorite(&self, id: &str) -> Result<bool, VaultError> {
        let conn = self.conn.lock().unwrap();
        let current_fav: i32 = conn
            .query_row(
                "SELECT is_favorite FROM vault_items WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| VaultError::NotFound(id.to_string()))?;

        let new_fav = if current_fav == 1 { 0 } else { 1 };
        conn.execute(
            "UPDATE vault_items SET is_favorite = ?1 WHERE id = ?2",
            params![new_fav, id],
        )?;
        Ok(new_fav == 1)
    }

    pub fn get_stats(&self) -> Result<VaultStats, VaultError> {
        let items = self.list_items(None, None)?;
        let mut stats = VaultStats {
            total_items: items.len() as u32,
            ..VaultStats::default()
        };

        let mut total_score = 0;

        for item in &items {
            match item.category {
                VaultCategory::Login => stats.logins_count += 1,
                VaultCategory::Note => stats.notes_count += 1,
                VaultCategory::ApiKey => stats.apikeys_count += 1,
                VaultCategory::CreditCard => {}
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

        // Cofre vazio conta como 100% saudável.
        stats.health_score = total_score.checked_div(stats.total_items).unwrap_or(100);

        Ok(stats)
    }
}

// =============================================================================
// Gerador Criptográfico & Avaliador de Força de Senhas
// =============================================================================

/// Inteiro uniforme em `0..n` a partir do CSPRNG do sistema operacional,
/// com amostragem por rejeição (sem viés de módulo).
fn random_below(n: usize) -> usize {
    let n = n as u32;
    let zone = u32::MAX - (u32::MAX % n);
    loop {
        let mut bytes = [0u8; 4];
        fill_random(&mut bytes);
        let v = u32::from_le_bytes(bytes);
        if v < zone {
            return (v % n) as usize;
        }
    }
}

pub fn generate_password(opts: &PasswordOptions) -> String {
    let mut classes: Vec<&str> = Vec::new();
    if opts.include_uppercase {
        classes.push("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if opts.include_lowercase {
        classes.push("abcdefghijklmnopqrstuvwxyz");
    }
    if opts.include_numbers {
        classes.push("0123456789");
    }
    if opts.include_symbols {
        classes.push("!@#$%&*-_=+");
    }
    if classes.is_empty() {
        classes.push("abcdefghijklmnopqrstuvwxyz");
        classes.push("0123456789");
    }

    let length = opts.length.clamp(6, 128);
    let pool: Vec<char> = classes.iter().flat_map(|c| c.chars()).collect();

    // Garante ao menos um caractere de cada classe solicitada...
    let mut chars: Vec<char> = classes
        .iter()
        .map(|c| {
            let set: Vec<char> = c.chars().collect();
            set[random_below(set.len())]
        })
        .collect();

    // ...completa com o pool inteiro...
    while chars.len() < length {
        chars.push(pool[random_below(pool.len())]);
    }

    // ...e embaralha (Fisher-Yates) para não fixar a posição das classes.
    for i in (1..chars.len()).rev() {
        let j = random_below(i + 1);
        chars.swap(i, j);
    }

    chars.into_iter().collect()
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
    if len >= 8 {
        points += 20;
    }
    if len >= 12 {
        points += 20;
    }
    if len >= 16 {
        points += 20;
    }
    if has_lower {
        points += 10;
    }
    if has_upper {
        points += 10;
    }
    if has_digit {
        points += 10;
    }
    if has_symbol {
        points += 10;
    }

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
            .save_item(VaultItemInput {
                id: None,
                title: "GitHub Access Token".to_string(),
                category: VaultCategory::ApiKey,
                username: Some("lucas@exemplo.com".to_string()),
                secret_payload: "ghp_secretTokenExample1234567890".to_string(),
                tags: vec!["dev".to_string(), "infra".to_string()],
                is_favorite: true,
            })
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
        let list = engine
            .list_items(Some(VaultCategory::ApiKey), Some("GitHub"))
            .unwrap();
        assert_eq!(list.len(), 1);

        // 5. Exclusão
        assert!(engine.delete_item(&item.id).unwrap());
        assert!(engine.get_item(&item.id).is_err());
    }

    #[test]
    fn test_generated_password_always_contains_every_requested_class() {
        let opts = PasswordOptions {
            length: 8,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_symbols: true,
        };
        for _ in 0..500 {
            let pwd = generate_password(&opts);
            assert_eq!(pwd.chars().count(), 8);
            assert!(
                pwd.chars().any(|c| c.is_ascii_uppercase()),
                "sem maiúscula: {pwd}"
            );
            assert!(
                pwd.chars().any(|c| c.is_ascii_lowercase()),
                "sem minúscula: {pwd}"
            );
            assert!(pwd.chars().any(|c| c.is_ascii_digit()), "sem dígito: {pwd}");
            assert!(
                pwd.chars().any(|c| !c.is_ascii_alphanumeric()),
                "sem símbolo: {pwd}"
            );
        }
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

    fn temp_db_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vault-{label}-{}.sqlite3", Uuid::new_v4()))
    }

    fn save_secret(engine: &VaultEngine, title: &str, secret: &str) -> VaultItem {
        engine
            .save_item(VaultItemInput {
                id: None,
                title: title.to_string(),
                category: VaultCategory::Login,
                username: Some("ana@exemplo.com".to_string()),
                secret_payload: secret.to_string(),
                tags: vec![],
                is_favorite: false,
            })
            .unwrap()
    }

    #[test]
    fn test_secrets_are_encrypted_at_rest_and_survive_reopen() {
        let path = temp_db_path("at-rest");
        let key = generate_vault_key();
        let secret = "SegredoUnicoQueNaoPodeVazar#2026";

        let id = {
            let engine = VaultEngine::open(path.to_str().unwrap(), &key).unwrap();
            save_secret(&engine, "Banco", secret).id
        };

        // Nenhum byte do arquivo SQLite contém o segredo em texto puro.
        let raw = std::fs::read(&path).unwrap();
        assert!(
            !raw.windows(secret.len()).any(|w| w == secret.as_bytes()),
            "segredo em texto puro encontrado no arquivo do banco"
        );

        // A mesma chave reabre o cofre e decifra o item.
        let engine = VaultEngine::open(path.to_str().unwrap(), &key).unwrap();
        assert_eq!(engine.get_item(&id).unwrap().secret_payload, secret);
        drop(engine);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_wrong_or_malformed_key_is_rejected() {
        let path = temp_db_path("wrong-key");
        {
            let engine = VaultEngine::open(path.to_str().unwrap(), &generate_vault_key()).unwrap();
            save_secret(&engine, "Banco", "s3nh@-Forte-123");
        }

        let other_key = generate_vault_key();
        assert!(matches!(
            VaultEngine::open(path.to_str().unwrap(), &other_key),
            Err(VaultError::WrongKeyOrCorrupted)
        ));
        assert!(matches!(
            VaultEngine::open(path.to_str().unwrap(), &[0u8; 16]),
            Err(VaultError::InvalidKey)
        ));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_ciphertext_is_bound_to_its_item() {
        let engine = VaultEngine::in_memory().unwrap();
        let a = save_secret(&engine, "A", "segredo-do-a");
        let b = save_secret(&engine, "B", "segredo-do-b");
        {
            let conn = engine.conn.lock().unwrap();
            conn.execute(
                "UPDATE vault_items SET secret_cipher = (SELECT secret_cipher FROM vault_items WHERE id = ?1) WHERE id = ?2",
                params![a.id, b.id],
            )
            .unwrap();
        }
        assert!(matches!(
            engine.get_item(&b.id),
            Err(VaultError::WrongKeyOrCorrupted)
        ));
    }

    #[test]
    fn test_legacy_plaintext_database_is_migrated_and_scrubbed() {
        let path = temp_db_path("legacy");
        let secret = "LegacyPlaintextSecret!42";
        {
            // Esquema das versões anteriores do template (segredo em texto puro).
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE vault_items (
                    id TEXT PRIMARY KEY, title TEXT NOT NULL, category TEXT NOT NULL,
                    username TEXT, secret_payload TEXT NOT NULL, tags TEXT NOT NULL,
                    is_favorite INTEGER NOT NULL DEFAULT 0,
                    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
                );
                CREATE INDEX idx_vault_category ON vault_items(category);
                "#,
            )
            .unwrap();
            conn.execute(
                "INSERT INTO vault_items VALUES ('legacy-1', 'Conta Antiga', 'login', NULL, ?1, '[\"antigo\"]', 1, 10, 20)",
                params![secret],
            )
            .unwrap();
        }

        let key = generate_vault_key();
        let engine = VaultEngine::open(path.to_str().unwrap(), &key).unwrap();
        let item = engine.get_item("legacy-1").unwrap();
        assert_eq!(item.secret_payload, secret);
        assert_eq!(item.tags, vec!["antigo".to_string()]);
        assert!(item.is_favorite);
        assert_eq!(item.created_at, 10);
        drop(engine);

        let raw = std::fs::read(&path).unwrap();
        assert!(
            !raw.windows(secret.len()).any(|w| w == secret.as_bytes()),
            "migração deixou o segredo em texto puro no arquivo"
        );

        // Reabrir não migra de novo e continua exigindo a mesma chave.
        let engine = VaultEngine::open(path.to_str().unwrap(), &key).unwrap();
        assert_eq!(engine.list_items(None, None).unwrap().len(), 1);
        drop(engine);
        let _ = std::fs::remove_file(&path);
    }
}
"####
        .to_string()
}

/// Camada UniFFI do cofre.
pub fn vault_ffi_lib_rs(ctx: &TemplateContext) -> String {
    r####"use __RUST_NAME___core as core;
use std::sync::Mutex;

uniffi::setup_scaffolding!();

#[derive(uniffi::Enum)]
pub enum NativeCategory {
    Login,
    Note,
    ApiKey,
    CreditCard,
}

impl From<NativeCategory> for core::VaultCategory {
    fn from(c: NativeCategory) -> Self {
        match c {
            NativeCategory::Login => core::VaultCategory::Login,
            NativeCategory::Note => core::VaultCategory::Note,
            NativeCategory::ApiKey => core::VaultCategory::ApiKey,
            NativeCategory::CreditCard => core::VaultCategory::CreditCard,
        }
    }
}

impl From<core::VaultCategory> for NativeCategory {
    fn from(c: core::VaultCategory) -> Self {
        match c {
            core::VaultCategory::Login => NativeCategory::Login,
            core::VaultCategory::Note => NativeCategory::Note,
            core::VaultCategory::ApiKey => NativeCategory::ApiKey,
            core::VaultCategory::CreditCard => NativeCategory::CreditCard,
        }
    }
}

#[derive(uniffi::Record)]
pub struct NativeVaultItem {
    pub id: String,
    pub title: String,
    pub category: NativeCategory,
    pub username: Option<String>,
    pub secret_payload: String,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<core::VaultItem> for NativeVaultItem {
    fn from(i: core::VaultItem) -> Self {
        Self {
            id: i.id,
            title: i.title,
            category: i.category.into(),
            username: i.username,
            secret_payload: i.secret_payload,
            tags: i.tags,
            is_favorite: i.is_favorite,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

/// Dados para criar ou atualizar um item (`id` vazio cria um novo).
#[derive(uniffi::Record)]
pub struct NativeVaultItemInput {
    #[uniffi(default = None)]
    pub id: Option<String>,
    pub title: String,
    pub category: NativeCategory,
    #[uniffi(default = None)]
    pub username: Option<String>,
    pub secret_payload: String,
    #[uniffi(default = [])]
    pub tags: Vec<String>,
    #[uniffi(default = false)]
    pub is_favorite: bool,
}

impl From<NativeVaultItemInput> for core::VaultItemInput {
    fn from(i: NativeVaultItemInput) -> Self {
        Self {
            id: i.id,
            title: i.title,
            category: i.category.into(),
            username: i.username,
            secret_payload: i.secret_payload,
            tags: i.tags,
            is_favorite: i.is_favorite,
        }
    }
}

#[derive(uniffi::Record)]
pub struct NativeVaultStats {
    pub total_items: u32,
    pub logins_count: u32,
    pub notes_count: u32,
    pub apikeys_count: u32,
    pub favorites_count: u32,
    pub weak_passwords_count: u32,
    pub health_score: u32,
}

impl From<core::VaultStats> for NativeVaultStats {
    fn from(s: core::VaultStats) -> Self {
        Self {
            total_items: s.total_items,
            logins_count: s.logins_count,
            notes_count: s.notes_count,
            apikeys_count: s.apikeys_count,
            favorites_count: s.favorites_count,
            weak_passwords_count: s.weak_passwords_count,
            health_score: s.health_score,
        }
    }
}

#[derive(uniffi::Record)]
pub struct NativePasswordOptions {
    pub length: u32,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_numbers: bool,
    pub include_symbols: bool,
}

#[derive(uniffi::Record)]
pub struct NativePasswordEvaluation {
    pub score: u32,
    pub label: String,
    pub is_secure: bool,
}

impl From<core::PasswordEvaluation> for NativePasswordEvaluation {
    fn from(e: core::PasswordEvaluation) -> Self {
        Self {
            score: e.score,
            label: e.label,
            is_secure: e.is_secure,
        }
    }
}

#[derive(uniffi::Error, Debug)]
pub enum NativeVaultError {
    Database {
        // Não pode se chamar `message`: colide com Throwable.message no Kotlin.
        detail: String,
    },
    NotFound {
        id: String,
    },
    InvalidInput,
    /// A chave não tem 32 bytes.
    InvalidKey,
    /// Chave diferente da usada para criar o cofre (ou dados adulterados).
    WrongKeyOrCorrupted,
}

impl From<core::VaultError> for NativeVaultError {
    fn from(err: core::VaultError) -> Self {
        match err {
            core::VaultError::DatabaseError(msg) => NativeVaultError::Database { detail: msg },
            core::VaultError::NotFound(id) => NativeVaultError::NotFound { id },
            core::VaultError::InvalidInput => NativeVaultError::InvalidInput,
            core::VaultError::InvalidKey => NativeVaultError::InvalidKey,
            core::VaultError::WrongKeyOrCorrupted => NativeVaultError::WrongKeyOrCorrupted,
        }
    }
}

impl std::fmt::Display for NativeVaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for NativeVaultError {}

#[derive(uniffi::Object)]
pub struct NativeVaultEngine {
    inner: Mutex<core::VaultEngine>,
}

#[uniffi::export]
impl NativeVaultEngine {
    #[uniffi::constructor]
    pub fn in_memory() -> Result<Self, NativeVaultError> {
        let engine = core::VaultEngine::in_memory().map_err(NativeVaultError::from)?;
        Ok(Self {
            inner: Mutex::new(engine),
        })
    }

    /// Abre o cofre em disco. `key` (32 bytes) deve vir do cofre de chaves da
    /// plataforma (Keychain / Android Keystore) e ser sempre a mesma.
    #[uniffi::constructor]
    pub fn open(path: String, key: Vec<u8>) -> Result<Self, NativeVaultError> {
        let engine = core::VaultEngine::open(&path, &key).map_err(NativeVaultError::from)?;
        Ok(Self {
            inner: Mutex::new(engine),
        })
    }

    /// Cria (`input.id == None`) ou atualiza um item; o segredo é cifrado no Rust.
    pub fn save_item(
        &self,
        input: NativeVaultItemInput,
    ) -> Result<NativeVaultItem, NativeVaultError> {
        let engine = self.inner.lock().unwrap();
        let item = engine
            .save_item(input.into())
            .map_err(NativeVaultError::from)?;
        Ok(item.into())
    }

    pub fn get_item(&self, id: String) -> Result<NativeVaultItem, NativeVaultError> {
        let engine = self.inner.lock().unwrap();
        let item = engine.get_item(&id).map_err(NativeVaultError::from)?;
        Ok(item.into())
    }

    pub fn delete_item(&self, id: String) -> Result<bool, NativeVaultError> {
        let engine = self.inner.lock().unwrap();
        engine.delete_item(&id).map_err(NativeVaultError::from)
    }

    pub fn list_items(
        &self,
        category: Option<NativeCategory>,
        query: Option<String>,
    ) -> Result<Vec<NativeVaultItem>, NativeVaultError> {
        let engine = self.inner.lock().unwrap();
        let cat = category.map(Into::into);
        let items = engine
            .list_items(cat, query.as_deref())
            .map_err(NativeVaultError::from)?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    pub fn toggle_favorite(&self, id: String) -> Result<bool, NativeVaultError> {
        let engine = self.inner.lock().unwrap();
        engine.toggle_favorite(&id).map_err(NativeVaultError::from)
    }

    pub fn get_stats(&self) -> Result<NativeVaultStats, NativeVaultError> {
        let engine = self.inner.lock().unwrap();
        let stats = engine.get_stats().map_err(NativeVaultError::from)?;
        Ok(stats.into())
    }
}

/// Gera uma nova chave de 32 bytes para o cofre (CSPRNG do sistema).
#[uniffi::export]
pub fn generate_vault_key() -> Vec<u8> {
    core::generate_vault_key()
}

#[uniffi::export]
pub fn generate_secure_password(options: NativePasswordOptions) -> String {
    let opts = core::PasswordOptions {
        length: options.length as usize,
        include_uppercase: options.include_uppercase,
        include_lowercase: options.include_lowercase,
        include_numbers: options.include_numbers,
        include_symbols: options.include_symbols,
    };
    core::generate_password(&opts)
}

#[uniffi::export]
pub fn evaluate_password(password: String) -> NativePasswordEvaluation {
    core::evaluate_password(&password).into()
}
"####
        .replace("__RUST_NAME__", &ctx.rust_name)
}

pub fn vault_ffi_lifecycle_test_rs(ctx: &TemplateContext) -> String {
    r####"//! Testes de Integração de Lifecycle do Cofre Seguro via UniFFI
//! Simula Cold Start, persistência SQLite e concorrência multithread.

use __RUST_NAME___ffi as native;
use std::sync::Arc;
use std::thread;

#[test]
fn test_vault_ffi_lifecycle_and_concurrency() {
    let engine =
        native::NativeVaultEngine::in_memory().expect("Inicialização in-memory deve suceder");

    // 1. Inserção de credenciais
    let item = engine
        .save_item(native::NativeVaultItemInput {
            id: None,
            title: "Chave de API Stripe".to_string(),
            category: native::NativeCategory::ApiKey,
            username: Some("contato@empresa.com".to_string()),
            secret_payload: "sk_live_stripe_secret_key_12345".to_string(),
            tags: vec!["pagamentos".to_string()],
            is_favorite: true,
        })
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
    let pwd = native::generate_secure_password(native::NativePasswordOptions {
        length: 20,
        include_uppercase: true,
        include_lowercase: true,
        include_numbers: true,
        include_symbols: true,
    });
    assert_eq!(pwd.len(), 20);

    let eval = native::evaluate_password(pwd);
    assert!(eval.is_secure);

    // 4. Teste de Concorrência Multithread
    let engine_arc = Arc::new(engine);
    let mut handles = Vec::new();

    for i in 0..10 {
        let eng = Arc::clone(&engine_arc);
        handles.push(thread::spawn(move || {
            eng.save_item(native::NativeVaultItemInput {
                id: None,
                title: format!("Item Concorrente {}", i),
                category: native::NativeCategory::Note,
                username: None,
                secret_payload: format!("Payload seguro {}", i),
                tags: vec![],
                is_favorite: false,
            })
            .unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_stats = engine_arc.get_stats().unwrap();
    assert_eq!(final_stats.total_items, 11);
}

#[test]
fn test_vault_ffi_encrypted_persistence_requires_the_same_key() {
    let path = std::env::temp_dir().join(format!("ffi-vault-{}.sqlite3", std::process::id()));
    let path_str = path.to_str().unwrap().to_string();
    let _ = std::fs::remove_file(&path);
    let key = native::generate_vault_key();
    assert_eq!(key.len(), 32);

    let id = {
        let engine = native::NativeVaultEngine::open(path_str.clone(), key.clone()).unwrap();
        engine
            .save_item(native::NativeVaultItemInput {
                id: None,
                title: "Banco".to_string(),
                category: native::NativeCategory::Login,
                username: None,
                secret_payload: "SegredoFFI#2026".to_string(),
                tags: vec![],
                is_favorite: false,
            })
            .unwrap()
            .id
    };

    let reopened = native::NativeVaultEngine::open(path_str.clone(), key).unwrap();
    assert_eq!(
        reopened.get_item(id).unwrap().secret_payload,
        "SegredoFFI#2026"
    );
    drop(reopened);

    assert!(matches!(
        native::NativeVaultEngine::open(path_str.clone(), native::generate_vault_key()),
        Err(native::NativeVaultError::WrongKeyOrCorrupted)
    ));
    assert!(matches!(
        native::NativeVaultEngine::open(path_str, vec![1, 2, 3]),
        Err(native::NativeVaultError::InvalidKey)
    ));
    let _ = std::fs::remove_file(&path);
}
"####
        .replace("__RUST_NAME__", &ctx.rust_name)
}

pub fn vault_desktop_main_rs(ctx: &TemplateContext) -> String {
    r####"use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, el, row, text};
use __RUST_NAME___core as core;

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
        .child(text("🔐 __NAME__").font(type_::TITLE).color(t.text))
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
        "__NAME__ Desktop Vault",
        (820.0, 560.0),
        DesktopState::default,
        view,
    ));
}
"####
        .replace("__NAME__", &ctx.name)
        .replace("__RUST_NAME__", &ctx.rust_name)
}

/// UI Compose do cofre.
pub fn vault_android_main_activity_kt(ctx: &TemplateContext) -> String {
    r####"package __PACKAGE__

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import __PACKAGE__.uniffi.NativeCategory
import __PACKAGE__.uniffi.NativeVaultException
import __PACKAGE__.uniffi.NativeVaultItem
import __PACKAGE__.uniffi.NativeVaultStats
import __PACKAGE__.ui.theme.JirauTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            JirauTheme {
                Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    val context = LocalContext.current.applicationContext
                    VaultScreen(viewModel { VaultViewModel.persistent(context) })
                }
            }
        }
    }
}

fun strengthColor(score: UInt): Color = when {
    score >= 70u -> Color(0xFF2E7D32)
    score >= 40u -> Color(0xFFEF6C00)
    else -> Color(0xFFC62828)
}

fun copyToClipboard(context: Context, label: String, text: String) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
}

// =============================================================================
// Tela principal
// =============================================================================

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VaultScreen(vm: VaultViewModel) {
    val state by vm.state.collectAsState()
    var editing by remember { mutableStateOf<EditorTarget?>(null) }
    var detailId by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("🔐 __NAME__", fontWeight = FontWeight.Bold) },
                actions = {
                    TextButton(
                        onClick = { editing = EditorTarget.New },
                        modifier = Modifier.testTag("add-item")
                    ) { Text("＋ Nova") }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer
                )
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            state.errorMessage?.let { message ->
                item {
                    Card(
                        onClick = vm::dismissError,
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer)
                    ) {
                        Text("⚠️ $message", modifier = Modifier.padding(16.dp).testTag("error-message"))
                    }
                }
            }

            // Cofre vazio: o convite fica no topo, sempre visível.
            if (state.items.isEmpty() && !state.isFiltering) {
                item {
                    EmptyVaultCard(onAdd = { editing = EditorTarget.New }, onLoadSamples = vm::loadSampleItems)
                }
            }

            state.stats?.let { stats -> item { HealthSummary(stats) } }

            item { PasswordGenerator(state, vm) }

            item {
                OutlinedTextField(
                    value = state.searchQuery,
                    onValueChange = vm::setSearchQuery,
                    label = { Text("Buscar por título, usuário ou tag") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("search")
                )
            }

            item {
                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = state.categoryFilter == null,
                        onClick = { vm.setCategoryFilter(null) },
                        label = { Text("Todas") }
                    )
                    NativeCategory.entries.forEach { category ->
                        FilterChip(
                            selected = state.categoryFilter == category,
                            onClick = { vm.setCategoryFilter(category) },
                            label = { Text("${category.emoji} ${category.displayName}") }
                        )
                    }
                }
            }

            item {
                Text("Credenciais (SQLite cifrado)", style = MaterialTheme.typography.titleMedium)
            }

            if (state.items.isEmpty()) {
                item {
                    Text(
                        if (state.isFiltering) "Nenhuma credencial encontrada." else "Nenhuma credencial salva ainda.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            items(state.items, key = { it.id }) { item ->
                VaultRow(
                    item = item,
                    onOpen = { detailId = item.id },
                    onToggleFavorite = { vm.toggleFavorite(item.id) }
                )
            }
        }
    }

    detailId?.let { id ->
        val item = vm.item(id)
        if (item == null) {
            detailId = null
        } else {
            ItemDetailDialog(
                item = item,
                vm = vm,
                onEdit = {
                    detailId = null
                    editing = EditorTarget.Existing(item)
                },
                onDelete = {
                    vm.delete(item.id)
                    detailId = null
                },
                onDismiss = { detailId = null }
            )
        }
    }

    editing?.let { target ->
        ItemEditorDialog(target = target, vm = vm, onDismiss = { editing = null })
    }
}

sealed interface EditorTarget {
    data object New : EditorTarget
    data class Existing(val item: NativeVaultItem) : EditorTarget
}

@Composable
fun EmptyVaultCard(onAdd: () -> Unit, onLoadSamples: () -> Unit) {
    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text("🛡️", style = MaterialTheme.typography.displaySmall)
            Text("Seu cofre está vazio", style = MaterialTheme.typography.titleMedium)
            Button(onClick = onAdd) { Text("Adicionar credencial") }
            TextButton(onClick = onLoadSamples, modifier = Modifier.testTag("load-samples")) {
                Text("Carregar exemplos")
            }
        }
    }
}

@Composable
fun HealthSummary(stats: NativeVaultStats) {
    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Saúde do Cofre", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                Text(
                    "${stats.healthScore}%",
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.Bold,
                    color = strengthColor(stats.healthScore),
                    modifier = Modifier.testTag("health-score")
                )
            }
            LinearProgressIndicator(
                progress = { stats.healthScore.toFloat() / 100f },
                color = strengthColor(stats.healthScore),
                modifier = Modifier.fillMaxWidth()
            )
            Row(horizontalArrangement = Arrangement.SpaceEvenly, modifier = Modifier.fillMaxWidth()) {
                StatBadge("${stats.totalItems}", "Itens")
                StatBadge("${stats.favoritesCount}", "Favoritos")
                StatBadge("${stats.weakPasswordsCount}", "Fracas")
            }
        }
    }
}

@Composable
fun StatBadge(value: String, label: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(value, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
fun PasswordGenerator(state: VaultUiState, vm: VaultViewModel) {
    val context = LocalContext.current
    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Gerador de Senhas (Rust Core)", style = MaterialTheme.typography.titleMedium)
            Text(
                state.generatedPassword,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.testTag("generated-password")
            )
            state.generatedEvaluation?.let { evaluation ->
                Text(
                    "${evaluation.label} (${evaluation.score})",
                    color = strengthColor(evaluation.score),
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.testTag("generated-strength")
                )
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Tamanho: ${state.passwordLength}", modifier = Modifier.width(110.dp))
                Slider(
                    value = state.passwordLength.toFloat(),
                    onValueChange = { vm.setPasswordLength(it.toInt()) },
                    valueRange = 8f..64f,
                    modifier = Modifier.weight(1f)
                )
            }
            Row {
                TextButton(onClick = vm::regeneratePassword, modifier = Modifier.testTag("regenerate-password")) {
                    Text("↻ Gerar nova")
                }
                Spacer(Modifier.weight(1f))
                TextButton(onClick = { copyToClipboard(context, "Senha", state.generatedPassword) }) {
                    Text("Copiar")
                }
            }
        }
    }
}

@Composable
fun VaultRow(item: NativeVaultItem, onOpen: () -> Unit, onToggleFavorite: () -> Unit) {
    Card(modifier = Modifier.fillMaxWidth().clickable(onClick = onOpen)) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(14.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(item.category.emoji, style = MaterialTheme.typography.titleLarge)
            Column(modifier = Modifier.weight(1f).padding(horizontal = 12.dp)) {
                Text(item.title, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                Text(
                    listOfNotNull(item.category.displayName, item.username).joinToString(" • "),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            TextButton(onClick = onToggleFavorite) {
                Text(if (item.isFavorite) "★" else "☆", style = MaterialTheme.typography.titleLarge)
            }
        }
    }
}

// =============================================================================
// Detalhe
// =============================================================================

@Composable
fun ItemDetailDialog(
    item: NativeVaultItem,
    vm: VaultViewModel,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    var revealed by remember { mutableStateOf(false) }
    val evaluation = remember(item.secretPayload) { vm.evaluate(item.secretPayload) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("${item.category.emoji} ${item.title}") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Categoria: ${item.category.displayName}")
                item.username?.let { Text("Usuário: $it") }
                if (item.tags.isNotEmpty()) Text("Tags: ${item.tags.joinToString(", ")}")
                Text(
                    if (revealed) item.secretPayload else "••••••••••••••••",
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.testTag("secret-value")
                )
                Text(
                    "Força: ${evaluation.label} (${evaluation.score})",
                    color = strengthColor(evaluation.score)
                )
                Row {
                    TextButton(onClick = { revealed = !revealed }, modifier = Modifier.testTag("reveal-secret")) {
                        Text(if (revealed) "Ocultar" else "Revelar")
                    }
                    TextButton(onClick = { copyToClipboard(context, item.title, item.secretPayload) }) {
                        Text("Copiar")
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onEdit) { Text("Editar") } },
        dismissButton = {
            Row {
                TextButton(onClick = onDelete) { Text("Excluir", color = MaterialTheme.colorScheme.error) }
                TextButton(onClick = onDismiss) { Text("Fechar") }
            }
        }
    )
}

// =============================================================================
// Editor
// =============================================================================

@Composable
fun ItemEditorDialog(target: EditorTarget, vm: VaultViewModel, onDismiss: () -> Unit) {
    val existing = (target as? EditorTarget.Existing)?.item
    var title by remember { mutableStateOf(existing?.title ?: "") }
    var category by remember { mutableStateOf(existing?.category ?: NativeCategory.LOGIN) }
    var username by remember { mutableStateOf(existing?.username ?: "") }
    var secret by remember { mutableStateOf(existing?.secretPayload ?: "") }
    var tags by remember { mutableStateOf(existing?.tags?.joinToString(", ") ?: "") }
    var isFavorite by remember { mutableStateOf(existing?.isFavorite ?: false) }
    var showSecret by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (existing == null) "Nova credencial" else "Editar credencial") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Título") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("item-title")
                )
                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    NativeCategory.entries.forEach { option ->
                        FilterChip(
                            selected = category == option,
                            onClick = { category = option },
                            label = { Text(option.displayName) }
                        )
                    }
                }
                OutlinedTextField(
                    value = username,
                    onValueChange = { username = it },
                    label = { Text("Usuário (opcional)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = secret,
                    onValueChange = { secret = it },
                    label = { Text("Segredo") },
                    singleLine = true,
                    visualTransformation = if (showSecret) VisualTransformation.None else PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    modifier = Modifier.fillMaxWidth().testTag("item-secret")
                )
                if (secret.isNotEmpty()) {
                    val evaluation = vm.evaluate(secret)
                    Text("Força: ${evaluation.label} (${evaluation.score})", color = strengthColor(evaluation.score))
                }
                Row {
                    TextButton(
                        onClick = {
                            secret = vm.newPassword()
                            showSecret = true
                        },
                        modifier = Modifier.testTag("generate-secret")
                    ) { Text("Gerar senha forte (Rust)") }
                    TextButton(onClick = { showSecret = !showSecret }) {
                        Text(if (showSecret) "Ocultar" else "Mostrar")
                    }
                }
                OutlinedTextField(
                    value = tags,
                    onValueChange = { tags = it },
                    label = { Text("Tags (separadas por vírgula)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Favorito", modifier = Modifier.weight(1f))
                    Switch(checked = isFavorite, onCheckedChange = { isFavorite = it })
                }
                error?.let {
                    Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.testTag("editor-error"))
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    try {
                        vm.save(
                            id = existing?.id,
                            title = title,
                            category = category,
                            username = username,
                            secret = secret,
                            tags = tags.split(",").map { it.trim() }.filter { it.isNotEmpty() },
                            isFavorite = isFavorite,
                        )
                        onDismiss()
                    } catch (e: NativeVaultException) {
                        // Validação feita no Rust (título/segredo vazios etc.).
                        error = describe(e)
                    }
                },
                modifier = Modifier.testTag("save-item")
            ) { Text("Salvar") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancelar") } }
    )
}
"####
        .replace("__NAME__", &ctx.name)
        .replace("__PACKAGE__", &ctx.package_id)
}

/// ViewModel + chave no Android Keystore.
pub fn vault_android_core_kt(ctx: &TemplateContext) -> String {
    r####"package __PACKAGE__

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.lifecycle.ViewModel
import __PACKAGE__.uniffi.NativeCategory
import __PACKAGE__.uniffi.NativePasswordEvaluation
import __PACKAGE__.uniffi.NativePasswordOptions
import __PACKAGE__.uniffi.NativeVaultEngine
import __PACKAGE__.uniffi.NativeVaultException
import __PACKAGE__.uniffi.NativeVaultItem
import __PACKAGE__.uniffi.NativeVaultItemInput
import __PACKAGE__.uniffi.NativeVaultStats
import __PACKAGE__.uniffi.evaluatePassword
import __PACKAGE__.uniffi.generateSecurePassword
import __PACKAGE__.uniffi.generateVaultKey
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

// =============================================================================
// Integração com o núcleo Rust (UniFFI)
// `NativeVaultEngine`, `generateSecurePassword`, `evaluatePassword` e
// `generateVaultKey` vêm de app/build/generated/uniffi
// (scripts/build-android-ffi.sh, executado pelo Gradle antes do build).
// =============================================================================

val NativeCategory.displayName: String
    get() = when (this) {
        NativeCategory.LOGIN -> "Login"
        NativeCategory.NOTE -> "Nota"
        NativeCategory.API_KEY -> "Chave de API"
        NativeCategory.CREDIT_CARD -> "Cartão"
    }

val NativeCategory.emoji: String
    get() = when (this) {
        NativeCategory.LOGIN -> "🔑"
        NativeCategory.NOTE -> "📝"
        NativeCategory.API_KEY -> "⚡"
        NativeCategory.CREDIT_CARD -> "💳"
    }

fun describe(error: Throwable): String = when (error) {
    is NativeVaultException.InvalidInput -> "Título e segredo são obrigatórios."
    is NativeVaultException.NotFound -> "Item não encontrado (${error.id})."
    is NativeVaultException.Database -> "Erro no banco de dados: ${error.detail}"
    is NativeVaultException.InvalidKey -> "Chave do cofre inválida."
    is NativeVaultException.WrongKeyOrCorrupted ->
        "A chave do Keystore não abre este cofre (chave trocada ou dados corrompidos)."
    else -> error.message ?: error.toString()
}

// =============================================================================
// Chave do cofre no Android Keystore
// O Rust cifra cada segredo com AES-256-GCM-SIV (aws-lc-rs) usando uma chave de 32 bytes
// gerada pelo próprio núcleo. Essa chave só é gravada embrulhada (AES-GCM) por
// uma chave do Android Keystore, que nunca sai do hardware/TEE do aparelho.
// =============================================================================

object VaultKeyStore {
    private const val KEYSTORE = "AndroidKeyStore"
    private const val WRAPPING_ALIAS = "vault-master-key-wrapper"
    private const val PREFS = "vault_key"
    private const val PREF_WRAPPED_KEY = "wrapped_vault_key"
    private const val GCM_IV_BYTES = 12
    private const val GCM_TAG_BITS = 128

    /** Lê a chave do cofre; na primeira execução gera uma nova e a guarda. */
    fun loadOrCreate(context: Context): ByteArray {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        prefs.getString(PREF_WRAPPED_KEY, null)?.let { stored ->
            return unwrap(Base64.decode(stored, Base64.NO_WRAP))
        }
        val key = generateVaultKey()
        val wrapped = Base64.encodeToString(wrap(key), Base64.NO_WRAP)
        check(prefs.edit().putString(PREF_WRAPPED_KEY, wrapped).commit()) {
            "Não foi possível salvar a chave do cofre"
        }
        return key
    }

    private fun wrappingKey(): SecretKey {
        val keyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }
        (keyStore.getKey(WRAPPING_ALIAS, null) as? SecretKey)?.let { return it }

        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                WRAPPING_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        return generator.generateKey()
    }

    private fun wrap(plain: ByteArray): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, wrappingKey())
        return cipher.iv + cipher.doFinal(plain)
    }

    private fun unwrap(blob: ByteArray): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            wrappingKey(),
            GCMParameterSpec(GCM_TAG_BITS, blob, 0, GCM_IV_BYTES)
        )
        return cipher.doFinal(blob, GCM_IV_BYTES, blob.size - GCM_IV_BYTES)
    }
}

// =============================================================================
// ViewModel: toda regra de negócio e persistência acontece no Rust.
// =============================================================================

data class VaultUiState(
    val items: List<NativeVaultItem> = emptyList(),
    val stats: NativeVaultStats? = null,
    val searchQuery: String = "",
    val categoryFilter: NativeCategory? = null,
    val passwordLength: Int = 20,
    val generatedPassword: String = "",
    val generatedEvaluation: NativePasswordEvaluation? = null,
    val errorMessage: String? = null,
) {
    val isFiltering: Boolean get() = searchQuery.isNotBlank() || categoryFilter != null
}

class VaultViewModel(private val engine: NativeVaultEngine) : ViewModel() {
    private val _state = MutableStateFlow(VaultUiState())
    val state: StateFlow<VaultUiState> = _state.asStateFlow()

    init {
        regeneratePassword()
        reload()
    }

    fun reload() = perform {
        val current = _state.value
        val query = current.searchQuery.trim().ifEmpty { null }
        val items = engine.listItems(current.categoryFilter, query)
        val stats = engine.getStats()
        _state.update { it.copy(items = items, stats = stats) }
    }

    fun setSearchQuery(query: String) {
        _state.update { it.copy(searchQuery = query) }
        reload()
    }

    fun setCategoryFilter(category: NativeCategory?) {
        _state.update { it.copy(categoryFilter = category) }
        reload()
    }

    fun item(id: String): NativeVaultItem? = runCatching { engine.getItem(id) }.getOrNull()

    /** Salva no Rust; lança [NativeVaultException] para a UI exibir a validação. */
    fun save(
        id: String?,
        title: String,
        category: NativeCategory,
        username: String,
        secret: String,
        tags: List<String>,
        isFavorite: Boolean,
    ): NativeVaultItem {
        val item = engine.saveItem(
            NativeVaultItemInput(
                id = id,
                title = title,
                category = category,
                username = username.trim().ifEmpty { null },
                secretPayload = secret,
                tags = tags,
                isFavorite = isFavorite,
            )
        )
        reload()
        return item
    }

    fun delete(id: String) {
        perform { engine.deleteItem(id) }
        reload()
    }

    fun toggleFavorite(id: String) {
        perform { engine.toggleFavorite(id) }
        reload()
    }

    fun setPasswordLength(length: Int) {
        _state.update { it.copy(passwordLength = length) }
        regeneratePassword()
    }

    fun regeneratePassword() {
        val password = newPassword(_state.value.passwordLength)
        _state.update {
            it.copy(generatedPassword = password, generatedEvaluation = evaluatePassword(password))
        }
    }

    fun newPassword(length: Int = 20): String = generateSecurePassword(
        NativePasswordOptions(
            length = length.toUInt(),
            includeUppercase = true,
            includeLowercase = true,
            includeNumbers = true,
            includeSymbols = true,
        )
    )

    fun evaluate(password: String): NativePasswordEvaluation = evaluatePassword(password)

    fun loadSampleItems() {
        perform {
            engine.saveItem(
                NativeVaultItemInput(
                    title = "GitHub Personal Token",
                    category = NativeCategory.API_KEY,
                    username = "dev@example.com",
                    secretPayload = _state.value.generatedPassword,
                    tags = listOf("dev"),
                    isFavorite = true,
                )
            )
            engine.saveItem(
                NativeVaultItemInput(
                    title = "Console de Produção",
                    category = NativeCategory.LOGIN,
                    username = "admin@example.com",
                    secretPayload = "senha123",
                    tags = listOf("infra"),
                )
            )
            engine.saveItem(
                NativeVaultItemInput(
                    title = "Chave de recuperação",
                    category = NativeCategory.NOTE,
                    secretPayload = "Guardar fora do computador: 4F7A-91C2-DD03",
                )
            )
        }
        reload()
    }

    fun dismissError() = _state.update { it.copy(errorMessage = null) }

    fun reportError(message: String) = _state.update { it.copy(errorMessage = message) }

    private inline fun perform(block: () -> Unit) {
        try {
            block()
        } catch (e: NativeVaultException) {
            _state.update { it.copy(errorMessage = describe(e)) }
        }
    }

    override fun onCleared() {
        engine.close()
    }

    companion object {
        /** Cofre em SQLite (fora de backup) com a chave no Android Keystore. */
        fun persistent(context: Context): VaultViewModel = try {
            val path = File(context.noBackupFilesDir, "vault.sqlite3").absolutePath
            val key = VaultKeyStore.loadOrCreate(context)
            VaultViewModel(NativeVaultEngine.open(path, key))
        } catch (e: Exception) {
            // Sem disco ou sem Keystore o app segue com um cofre temporário:
            // nada é gravado em disco sem criptografia.
            VaultViewModel(NativeVaultEngine.inMemory()).also {
                it.reportError("Cofre em disco indisponível (${describe(e)}). Usando cofre temporário.")
            }
        }
    }
}
"####
        .replace("__PACKAGE__", &ctx.package_id)
}

/// Testes instrumentados (ViewModel, Keystore e UI) contra o núcleo Rust real.
pub fn vault_android_tests_kt(ctx: &TemplateContext) -> String {
    r####"package __PACKAGE__

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import __PACKAGE__.uniffi.NativeCategory
import __PACKAGE__.uniffi.NativeVaultEngine
import __PACKAGE__.uniffi.NativeVaultException
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** ViewModel e UI contra o núcleo Rust real (SQLite em memória via UniFFI). */
@RunWith(AndroidJUnit4::class)
class NativeLifecycleTest {
    @get:Rule
    val compose = createComposeRule()

    private fun makeViewModel() = VaultViewModel(NativeVaultEngine.inMemory())

    @Test
    fun emptyVaultStartsWithPerfectHealth() {
        val state = makeViewModel().state.value
        assertTrue(state.items.isEmpty())
        assertEquals(0u, state.stats?.totalItems)
        assertEquals(100u, state.stats?.healthScore)
    }

    @Test
    fun saveSearchFavoriteAndDeleteRoundTripThroughRust() {
        val vm = makeViewModel()
        val saved = vm.save(null, "GitHub", NativeCategory.API_KEY, "dev@example.com", "ghp_Abc123!xyz7890Qw", listOf("dev"), false)
        vm.save(null, "Banco", NativeCategory.LOGIN, "", "senha123", emptyList(), false)
        assertEquals(2, vm.state.value.items.size)
        assertEquals(1u, vm.state.value.stats?.weakPasswordsCount)
        val banco = vm.state.value.items.first { it.title == "Banco" }
        assertNull("usuário vazio deve ser persistido como nulo", vm.item(banco.id)?.username)

        vm.setSearchQuery("git")
        assertEquals(listOf("GitHub"), vm.state.value.items.map { it.title })

        vm.setSearchQuery("")
        vm.setCategoryFilter(NativeCategory.LOGIN)
        assertEquals(listOf("Banco"), vm.state.value.items.map { it.title })

        vm.setCategoryFilter(null)
        vm.toggleFavorite(saved.id)
        assertEquals(true, vm.item(saved.id)?.isFavorite)
        assertEquals("favoritos vêm primeiro", saved.id, vm.state.value.items.first().id)

        vm.delete(saved.id)
        assertNull(vm.item(saved.id))
        assertEquals(1u, vm.state.value.stats?.totalItems)
        assertNull(vm.state.value.errorMessage)
    }

    @Test
    fun rustRejectsInvalidInput() {
        val vm = makeViewModel()
        try {
            vm.save(null, "   ", NativeCategory.LOGIN, "", "x", emptyList(), false)
            fail("o Rust deveria recusar título vazio")
        } catch (e: NativeVaultException.InvalidInput) {
            assertEquals("Título e segredo são obrigatórios.", describe(e))
        }
        assertTrue(vm.state.value.items.isEmpty())
    }

    @Test
    fun generatedPasswordComesFromRustAndIsStrong() {
        val vm = makeViewModel()
        vm.setPasswordLength(24)
        val first = vm.state.value.generatedPassword
        assertEquals(24, first.length)
        assertEquals(true, vm.state.value.generatedEvaluation?.isSecure)

        vm.regeneratePassword()
        assertNotEquals(first, vm.state.value.generatedPassword)
    }

    @Test
    fun keystoreKeyIsStableAndOpensTheSameVault() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val key = VaultKeyStore.loadOrCreate(context)
        assertEquals(32, key.size)
        assertArrayEquals(key, VaultKeyStore.loadOrCreate(context))

        val path = context.cacheDir.resolve("keystore-test.sqlite3").also { it.delete() }.absolutePath
        val id = NativeVaultEngine.open(path, key).use { engine ->
            VaultViewModel(engine).save(null, "Banco", NativeCategory.LOGIN, "", "S3gredo!Forte#2026", emptyList(), false).id
        }
        NativeVaultEngine.open(path, VaultKeyStore.loadOrCreate(context)).use { engine ->
            assertEquals("S3gredo!Forte#2026", engine.getItem(id).secretPayload)
        }
    }

    @Test
    fun uiCreatesCredentialAndShowsRustValidation() {
        val vm = makeViewModel()
        compose.setContent { VaultScreen(vm) }

        // Título vazio: a validação vem do Rust e aparece no editor.
        compose.onNodeWithTag("add-item").performClick()
        compose.onNodeWithTag("save-item").performClick()
        compose.onNodeWithTag("editor-error").assertIsDisplayed()

        compose.onNodeWithTag("item-title").performTextInput("Conta Banco Digital")
        compose.onNodeWithTag("generate-secret").performClick()
        compose.onNodeWithTag("save-item").performClick()

        compose.onNodeWithText("Conta Banco Digital").assertIsDisplayed()
        assertEquals(1, vm.state.value.items.size)
        assertTrue(vm.evaluate(vm.state.value.items.first().secretPayload).isSecure)
    }
}
"####
        .replace("__PACKAGE__", &ctx.package_id)
}

pub fn vault_ios_content_view_swift(ctx: &TemplateContext) -> String {
    r####"import SwiftUI
import Observation
import UIKit
import Security

// =============================================================================
// Integração com o núcleo Rust (UniFFI)
// Tipos `Native*`, `NativeVaultEngine`, `generateSecurePassword` e
// `evaluatePassword` vêm de ios/Generated (scripts/build-ios-ffi.sh).
// =============================================================================

extension NativeCategory: CaseIterable, Identifiable {
    public static var allCases: [NativeCategory] { [.login, .note, .apiKey, .creditCard] }
    public var id: Self { self }

    var displayName: String {
        switch self {
        case .login: return "Login"
        case .note: return "Nota"
        case .apiKey: return "Chave de API"
        case .creditCard: return "Cartão"
        }
    }

    var systemImage: String {
        switch self {
        case .login: return "person.badge.key"
        case .note: return "note.text"
        case .apiKey: return "key.horizontal"
        case .creditCard: return "creditcard"
        }
    }
}

extension NativeVaultItem: Identifiable {}

func describe(_ error: Error) -> String {
    if let vaultError = error as? NativeVaultError {
        switch vaultError {
        case .InvalidInput:
            return "Título e segredo são obrigatórios."
        case .NotFound(let id):
            return "Item não encontrado (\(id))."
        case .Database(let detail):
            return "Erro no banco de dados: \(detail)"
        case .InvalidKey:
            return "Chave do cofre inválida."
        case .WrongKeyOrCorrupted:
            return "A chave do Keychain não abre este cofre (chave trocada ou dados corrompidos)."
        }
    }
    return error.localizedDescription
}

// =============================================================================
// Chave do cofre no Keychain
// O Rust cifra cada segredo com AES-256-GCM-SIV (aws-lc-rs); a chave de 32 bytes é
// gerada pelo próprio núcleo e fica só no Keychain (nunca no SQLite).
// =============================================================================

struct KeychainError: LocalizedError {
    let status: OSStatus

    var errorDescription: String? {
        let detail = SecCopyErrorMessageString(status, nil) as String? ?? "erro desconhecido"
        return "Keychain (\(status)): \(detail)"
    }
}

enum VaultKeyStore {
    private static let service = (Bundle.main.bundleIdentifier ?? "vault") + ".vault-key"
    private static let account = "vault-master-key"

    private static var baseQuery: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
    }

    /// Lê a chave do cofre; na primeira execução gera uma nova e a guarda.
    static func loadOrCreate() throws -> Data {
        if let key = try load() {
            return key
        }
        let key = generateVaultKey()
        var query = baseQuery
        query[kSecValueData as String] = key
        // Só neste aparelho e só depois do primeiro desbloqueio: não vai para
        // backups nem para outros dispositivos.
        query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainError(status: status) }
        return key
    }

    private static func load() throws -> Data? {
        var query = baseQuery
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        switch status {
        case errSecSuccess:
            return result as? Data
        case errSecItemNotFound:
            return nil
        default:
            throw KeychainError(status: status)
        }
    }
}

func strengthColor(_ score: UInt32) -> Color {
    switch score {
    case 70...: return .green
    case 40..<70: return .orange
    default: return .red
    }
}

// =============================================================================
// ViewModel: toda regra de negócio e persistência acontece no Rust.
// =============================================================================

@Observable
final class VaultViewModel {
    private let engine: NativeVaultEngine

    private(set) var items: [NativeVaultItem] = []
    private(set) var stats: NativeVaultStats?
    var searchQuery = ""
    var categoryFilter: NativeCategory?
    var passwordLength: Double = 20
    private(set) var generatedPassword = ""
    private(set) var generatedEvaluation: NativePasswordEvaluation?
    var errorMessage: String?

    init(engine: NativeVaultEngine) {
        self.engine = engine
        regeneratePassword()
        reload()
    }

    /// Cofre persistido em SQLite em Application Support, com a chave no Keychain.
    static func persistent() -> VaultViewModel {
        do {
            let dir = try FileManager.default.url(
                for: .applicationSupportDirectory,
                in: .userDomainMask,
                appropriateFor: nil,
                create: true
            )
            let path = dir.appendingPathComponent("vault.sqlite3").path
            let key = try VaultKeyStore.loadOrCreate()
            return VaultViewModel(engine: try NativeVaultEngine.open(path: path, key: key))
        } catch {
            // Sem disco ou sem Keychain o app segue com um cofre temporário:
            // nada é gravado em disco sem criptografia.
            let vm = VaultViewModel(engine: try! NativeVaultEngine.inMemory())
            vm.errorMessage = "Cofre em disco indisponível (\(describe(error))). Usando cofre temporário."
            return vm
        }
    }

    func reload() {
        perform {
            let query = searchQuery.trimmingCharacters(in: .whitespaces)
            items = try engine.listItems(category: categoryFilter, query: query.isEmpty ? nil : query)
            stats = try engine.getStats()
        }
    }

    func item(id: String) -> NativeVaultItem? {
        try? engine.getItem(id: id)
    }

    @discardableResult
    func save(
        id: String?,
        title: String,
        category: NativeCategory,
        username: String,
        secret: String,
        tags: [String],
        isFavorite: Bool
    ) throws -> NativeVaultItem {
        let trimmedUser = username.trimmingCharacters(in: .whitespaces)
        let item = try engine.saveItem(input: NativeVaultItemInput(
            id: id,
            title: title,
            category: category,
            username: trimmedUser.isEmpty ? nil : trimmedUser,
            secretPayload: secret,
            tags: tags,
            isFavorite: isFavorite
        ))
        reload()
        return item
    }

    func delete(id: String) {
        perform { _ = try engine.deleteItem(id: id) }
        reload()
    }

    func toggleFavorite(id: String) {
        perform { _ = try engine.toggleFavorite(id: id) }
        reload()
    }

    func regeneratePassword() {
        let options = NativePasswordOptions(
            length: UInt32(passwordLength),
            includeUppercase: true,
            includeLowercase: true,
            includeNumbers: true,
            includeSymbols: true
        )
        generatedPassword = generateSecurePassword(options: options)
        generatedEvaluation = evaluatePassword(password: generatedPassword)
    }

    func evaluate(_ password: String) -> NativePasswordEvaluation {
        evaluatePassword(password: password)
    }

    func loadSampleItems() {
        perform {
            _ = try engine.saveItem(input: NativeVaultItemInput(
                title: "GitHub Personal Token", category: .apiKey,
                username: "dev@example.com", secretPayload: generatedPassword,
                tags: ["dev"], isFavorite: true
            ))
            _ = try engine.saveItem(input: NativeVaultItemInput(
                title: "Console de Produção", category: .login,
                username: "admin@example.com", secretPayload: "senha123",
                tags: ["infra"]
            ))
            _ = try engine.saveItem(input: NativeVaultItemInput(
                title: "Chave de recuperação", category: .note,
                secretPayload: "Guardar fora do computador: 4F7A-91C2-DD03"
            ))
        }
        regeneratePassword()
        reload()
    }

    private func perform(_ block: () throws -> Void) {
        do {
            try block()
        } catch {
            errorMessage = describe(error)
        }
    }
}

// =============================================================================
// Tela principal
// =============================================================================

struct ContentView: View {
    @State private var vm = VaultViewModel.persistent()
    @State private var isAdding = false

    private var isFiltering: Bool {
        !vm.searchQuery.isEmpty || vm.categoryFilter != nil
    }

    var body: some View {
        NavigationStack {
            List {
                if let message = vm.errorMessage {
                    Section {
                        Label(message, systemImage: "exclamationmark.triangle.fill")
                            .foregroundStyle(.red)
                            .onTapGesture { vm.errorMessage = nil }
                    }
                }

                // Cofre vazio: o convite fica no topo; no fim da lista ele
                // ficava escondido sob a barra de busca flutuante.
                if vm.items.isEmpty && !isFiltering {
                    Section {
                        EmptyVaultView(
                            isFiltering: false,
                            onAdd: { isAdding = true },
                            onLoadSamples: { vm.loadSampleItems() }
                        )
                    }
                }

                if let stats = vm.stats {
                    Section("Saúde do Cofre") {
                        HealthSummary(stats: stats)
                    }
                }

                Section("Gerador de Senhas (Rust Core)") {
                    PasswordGenerator(vm: vm)
                }

                Section {
                    Picker("Categoria", selection: $vm.categoryFilter) {
                        Text("Todas").tag(NativeCategory?.none)
                        ForEach(NativeCategory.allCases) { category in
                            Label(category.displayName, systemImage: category.systemImage)
                                .tag(Optional(category))
                        }
                    }

                    if vm.items.isEmpty {
                        if isFiltering {
                            EmptyVaultView(isFiltering: true, onAdd: {}, onLoadSamples: {})
                        } else {
                            Text("Nenhuma credencial salva ainda.")
                                .foregroundStyle(.secondary)
                        }
                    } else {
                        ForEach(vm.items) { item in
                            NavigationLink(value: item.id) {
                                VaultRow(item: item)
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    vm.delete(id: item.id)
                                } label: {
                                    Label("Excluir", systemImage: "trash")
                                }
                            }
                            .swipeActions(edge: .leading) {
                                Button {
                                    vm.toggleFavorite(id: item.id)
                                } label: {
                                    Label("Favorito", systemImage: item.isFavorite ? "star.slash" : "star")
                                }
                                .tint(.yellow)
                            }
                        }
                    }
                } header: {
                    Text("Credenciais (SQLite)")
                }
            }
            .navigationTitle("🔐 __NAME__")
            .navigationDestination(for: String.self) { id in
                ItemDetailView(vm: vm, itemId: id)
            }
            .searchable(text: $vm.searchQuery, prompt: "Buscar por título, usuário ou tag")
            .onChange(of: vm.searchQuery) { vm.reload() }
            .onChange(of: vm.categoryFilter) { vm.reload() }
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        isAdding = true
                    } label: {
                        Label("Nova credencial", systemImage: "plus")
                    }
                    .accessibilityIdentifier("add-item")
                }
            }
            .sheet(isPresented: $isAdding) {
                ItemEditorView(vm: vm, existing: nil)
            }
        }
    }
}

struct HealthSummary: View {
    let stats: NativeVaultStats

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Pontuação")
                Spacer()
                Text("\(stats.healthScore)%")
                    .font(.title2.bold())
                    .foregroundStyle(strengthColor(stats.healthScore))
                    .accessibilityIdentifier("health-score")
            }
            ProgressView(value: Double(stats.healthScore), total: 100)
                .tint(strengthColor(stats.healthScore))
            HStack {
                StatBadge(value: stats.totalItems, label: "Itens")
                StatBadge(value: stats.favoritesCount, label: "Favoritos")
                StatBadge(value: stats.weakPasswordsCount, label: "Fracas")
            }
        }
        .padding(.vertical, 4)
    }
}

struct StatBadge: View {
    let value: UInt32
    let label: String

    var body: some View {
        VStack {
            Text("\(value)").font(.headline)
            Text(label).font(.caption).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
    }
}

struct PasswordGenerator: View {
    @Bindable var vm: VaultViewModel
    @State private var copied = false

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(vm.generatedPassword)
                .font(.system(.body, design: .monospaced).bold())
                .foregroundStyle(.blue)
                .textSelection(.enabled)
                .accessibilityIdentifier("generated-password")
            if let evaluation = vm.generatedEvaluation {
                StrengthMeter(evaluation: evaluation)
            }
            HStack {
                Text("Tamanho: \(Int(vm.passwordLength))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Slider(value: $vm.passwordLength, in: 8...64, step: 1)
                    .onChange(of: vm.passwordLength) { vm.regeneratePassword() }
            }
            HStack {
                Button {
                    vm.regeneratePassword()
                    copied = false
                } label: {
                    Label("Gerar nova", systemImage: "arrow.clockwise")
                }
                .accessibilityIdentifier("regenerate-password")
                Spacer()
                Button {
                    UIPasteboard.general.string = vm.generatedPassword
                    copied = true
                } label: {
                    Label(copied ? "Copiada" : "Copiar", systemImage: copied ? "checkmark" : "doc.on.doc")
                }
            }
            .buttonStyle(.borderless)
        }
    }
}

struct StrengthMeter: View {
    let evaluation: NativePasswordEvaluation

    var body: some View {
        HStack {
            ProgressView(value: Double(evaluation.score), total: 100)
                .tint(strengthColor(evaluation.score))
            Text("\(evaluation.label) (\(evaluation.score))")
                .font(.caption.bold())
                .foregroundStyle(strengthColor(evaluation.score))
        }
    }
}

struct VaultRow: View {
    let item: NativeVaultItem

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: item.category.systemImage)
                .foregroundStyle(.tint)
                .frame(width: 28)
            VStack(alignment: .leading, spacing: 3) {
                Text(item.title).font(.headline)
                Text([item.category.displayName, item.username].compactMap { $0 }.joined(separator: " • "))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if item.isFavorite {
                Image(systemName: "star.fill").foregroundStyle(.yellow)
            }
        }
    }
}

struct EmptyVaultView: View {
    let isFiltering: Bool
    let onAdd: () -> Void
    let onLoadSamples: () -> Void

    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: isFiltering ? "magnifyingglass" : "lock.shield")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text(isFiltering ? "Nenhuma credencial encontrada" : "Seu cofre está vazio")
                .font(.headline)
            if !isFiltering {
                Button("Adicionar credencial", action: onAdd)
                    .buttonStyle(.borderedProminent)
                Button("Carregar exemplos", action: onLoadSamples)
                    .buttonStyle(.borderless)
                    .accessibilityIdentifier("load-samples")
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical)
    }
}

// =============================================================================
// Detalhe
// =============================================================================

struct ItemDetailView: View {
    let vm: VaultViewModel
    let itemId: String
    @State private var item: NativeVaultItem?
    @State private var revealSecret = false
    @State private var isEditing = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        Group {
            if let item {
                List {
                    Section {
                        LabeledContent("Categoria", value: item.category.displayName)
                        if let username = item.username {
                            LabeledContent("Usuário", value: username)
                                .textSelection(.enabled)
                        }
                        if !item.tags.isEmpty {
                            LabeledContent("Tags", value: item.tags.joined(separator: ", "))
                        }
                    }
                    Section("Segredo") {
                        Text(revealSecret ? item.secretPayload : String(repeating: "•", count: 12))
                            .font(.system(.body, design: .monospaced))
                            .textSelection(.enabled)
                        StrengthMeter(evaluation: vm.evaluate(item.secretPayload))
                        HStack {
                            Button(revealSecret ? "Ocultar" : "Revelar") { revealSecret.toggle() }
                            Spacer()
                            Button("Copiar") { UIPasteboard.general.string = item.secretPayload }
                        }
                        .buttonStyle(.borderless)
                    }
                    Section {
                        LabeledContent(
                            "Atualizado",
                            value: Date(timeIntervalSince1970: TimeInterval(item.updatedAt))
                                .formatted(date: .abbreviated, time: .shortened)
                        )
                        Button(role: .destructive) {
                            vm.delete(id: item.id)
                            dismiss()
                        } label: {
                            Label("Excluir credencial", systemImage: "trash")
                        }
                    }
                }
                .navigationTitle(item.title)
                .toolbar {
                    ToolbarItemGroup(placement: .primaryAction) {
                        Button {
                            vm.toggleFavorite(id: item.id)
                            refresh()
                        } label: {
                            Image(systemName: item.isFavorite ? "star.fill" : "star")
                        }
                        Button("Editar") { isEditing = true }
                    }
                }
                .sheet(isPresented: $isEditing, onDismiss: refresh) {
                    ItemEditorView(vm: vm, existing: item)
                }
            } else {
                ContentUnavailableView("Item não encontrado", systemImage: "questionmark.folder")
            }
        }
        .onAppear(perform: refresh)
    }

    private func refresh() {
        item = vm.item(id: itemId)
    }
}

// =============================================================================
// Criação / edição
// =============================================================================

struct ItemEditorView: View {
    let vm: VaultViewModel
    let existing: NativeVaultItem?

    @State private var title = ""
    @State private var category: NativeCategory = .login
    @State private var username = ""
    @State private var secret = ""
    @State private var tags = ""
    @State private var isFavorite = false
    @State private var revealSecret = false
    @State private var errorMessage: String?
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Título", text: $title)
                        .accessibilityIdentifier("item-title")
                    Picker("Categoria", selection: $category) {
                        ForEach(NativeCategory.allCases) { category in
                            Text(category.displayName).tag(category)
                        }
                    }
                    TextField("Usuário / e-mail", text: $username)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.emailAddress)
                }
                Section("Segredo") {
                    HStack {
                        Group {
                            if revealSecret {
                                TextField("Senha ou conteúdo", text: $secret)
                            } else {
                                SecureField("Senha ou conteúdo", text: $secret)
                            }
                        }
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .font(.system(.body, design: .monospaced))
                        Button {
                            revealSecret.toggle()
                        } label: {
                            Image(systemName: revealSecret ? "eye.slash" : "eye")
                        }
                        .buttonStyle(.borderless)
                    }
                    if !secret.isEmpty {
                        StrengthMeter(evaluation: vm.evaluate(secret))
                    }
                    Button {
                        vm.regeneratePassword()
                        secret = vm.generatedPassword
                        revealSecret = true
                    } label: {
                        Label("Gerar senha forte (Rust)", systemImage: "wand.and.stars")
                    }
                }
                Section {
                    TextField("Tags separadas por vírgula", text: $tags)
                        .textInputAutocapitalization(.never)
                    Toggle("Favorito", isOn: $isFavorite)
                }
                if let errorMessage {
                    Section {
                        Text(errorMessage).foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle(existing == nil ? "Nova credencial" : "Editar credencial")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancelar") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Salvar", action: save)
                        .accessibilityIdentifier("save-item")
                }
            }
            .onAppear(perform: loadExisting)
        }
    }

    private func loadExisting() {
        guard let existing else { return }
        title = existing.title
        category = existing.category
        username = existing.username ?? ""
        secret = existing.secretPayload
        tags = existing.tags.joined(separator: ", ")
        isFavorite = existing.isFavorite
    }

    private func save() {
        let parsedTags = tags
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
        do {
            try vm.save(
                id: existing?.id,
                title: title,
                category: category,
                username: username,
                secret: secret,
                tags: parsedTags,
                isFavorite: isFavorite
            )
            dismiss()
        } catch {
            errorMessage = describe(error)
        }
    }
}

#Preview {
    ContentView()
}
"####
        .replace("__NAME__", &ctx.name)
}

pub fn vault_ios_tests_swift(ctx: &TemplateContext) -> String {
    r####"import XCTest
@testable import __PASCAL__

/// Testes do ViewModel contra o núcleo Rust real (SQLite em memória via UniFFI).
final class NativeLifecycleTests: XCTestCase {
    private func makeViewModel() throws -> VaultViewModel {
        VaultViewModel(engine: try NativeVaultEngine.inMemory())
    }

    func testEmptyVaultStartsWithPerfectHealth() throws {
        let vm = try makeViewModel()
        XCTAssertTrue(vm.items.isEmpty)
        XCTAssertEqual(vm.stats?.totalItems, 0)
        XCTAssertEqual(vm.stats?.healthScore, 100)
    }

    func testSaveSearchFavoriteAndDeleteRoundTripThroughRust() throws {
        let vm = try makeViewModel()
        let saved = try vm.save(
            id: nil, title: "GitHub", category: .apiKey, username: "dev@example.com",
            secret: "ghp_Abc123!xyz7890Qw", tags: ["dev"], isFavorite: false
        )
        try vm.save(
            id: nil, title: "Banco", category: .login, username: "",
            secret: "senha123", tags: [], isFavorite: false
        )
        XCTAssertEqual(vm.items.count, 2)
        XCTAssertEqual(vm.stats?.weakPasswordsCount, 1)
        XCTAssertNil(vm.item(id: vm.items.first { $0.title == "Banco" }!.id)?.username,
                     "usuário vazio deve ser persistido como nulo")

        vm.searchQuery = "git"
        vm.reload()
        XCTAssertEqual(vm.items.map(\.title), ["GitHub"])

        vm.searchQuery = ""
        vm.categoryFilter = .login
        vm.reload()
        XCTAssertEqual(vm.items.map(\.title), ["Banco"])

        vm.categoryFilter = nil
        vm.toggleFavorite(id: saved.id)
        XCTAssertEqual(vm.item(id: saved.id)?.isFavorite, true)
        XCTAssertEqual(vm.items.first?.id, saved.id, "favoritos vêm primeiro")

        vm.delete(id: saved.id)
        XCTAssertNil(vm.item(id: saved.id))
        XCTAssertEqual(vm.stats?.totalItems, 1)
        XCTAssertNil(vm.errorMessage)
    }

    func testEditingKeepsIdentity() throws {
        let vm = try makeViewModel()
        let saved = try vm.save(
            id: nil, title: "Original", category: .note, username: "",
            secret: "conteudo", tags: [], isFavorite: false
        )
        try vm.save(
            id: saved.id, title: "Renomeado", category: .note, username: "",
            secret: "conteudo", tags: ["a", "b"], isFavorite: true
        )
        XCTAssertEqual(vm.items.count, 1)
        XCTAssertEqual(vm.item(id: saved.id)?.title, "Renomeado")
        XCTAssertEqual(vm.item(id: saved.id)?.tags, ["a", "b"])
    }

    func testRustRejectsInvalidInput() throws {
        let vm = try makeViewModel()
        XCTAssertThrowsError(
            try vm.save(id: nil, title: "   ", category: .login, username: "",
                        secret: "x", tags: [], isFavorite: false)
        ) { error in
            guard case NativeVaultError.InvalidInput = error else {
                return XCTFail("erro inesperado: \(error)")
            }
        }
        XCTAssertTrue(vm.items.isEmpty)
    }

    func testKeychainKeyIsStableAndOpensTheSameVault() throws {
        // Falha com errSecMissingEntitlement (-34018) se o app não for assinado.
        let key = try VaultKeyStore.loadOrCreate()
        XCTAssertEqual(key.count, 32)
        XCTAssertEqual(try VaultKeyStore.loadOrCreate(), key)

        let path = FileManager.default.temporaryDirectory
            .appendingPathComponent("keychain-test-\(UUID().uuidString).sqlite3").path
        let id = try NativeVaultEngine.open(path: path, key: key).saveItem(input: NativeVaultItemInput(
            title: "Banco", category: .login, secretPayload: "S3gredo!Forte#2026"
        )).id
        let reopened = try NativeVaultEngine.open(path: path, key: try VaultKeyStore.loadOrCreate())
        XCTAssertEqual(try reopened.getItem(id: id).secretPayload, "S3gredo!Forte#2026")
    }

    func testGeneratedPasswordComesFromRustAndIsStrong() throws {
        let vm = try makeViewModel()
        vm.passwordLength = 24
        vm.regeneratePassword()
        let first = vm.generatedPassword
        XCTAssertEqual(first.count, 24)
        XCTAssertEqual(vm.generatedEvaluation?.isSecure, true)

        vm.regeneratePassword()
        XCTAssertNotEqual(vm.generatedPassword, first)
    }
}
"####
        .replace("__PASCAL__", &ctx.pascal_name)
}
