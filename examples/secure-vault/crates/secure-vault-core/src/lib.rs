use rusqlite::{params, Connection};
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
