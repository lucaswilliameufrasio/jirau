use aws_lc_rs::aead::{AES_256_GCM_SIV, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
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
