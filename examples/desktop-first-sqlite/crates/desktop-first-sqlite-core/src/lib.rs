use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("O valor não pode ser negativo")]
    NegativeValue,
    #[error("O título não pode ser vazio")]
    EmptyTitle,
    #[error("Erro de persistência: {0}")]
    StorageError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppState {
    pub counter: i32,
    pub title: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            counter: 0,
            title: "Jirau Native App".to_string(),
        }
    }
}

pub struct StateEngine {
    state: AppState,
}

impl StateEngine {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
        }
    }

    pub fn get_state(&self) -> AppState {
        self.state.clone()
    }

    pub fn increment(&mut self) -> AppState {
        self.state.counter += 1;
        self.state.clone()
    }

    pub fn decrement(&mut self) -> Result<AppState, CoreError> {
        if self.state.counter <= 0 {
            return Err(CoreError::NegativeValue);
        }
        self.state.counter -= 1;
        Ok(self.state.clone())
    }

    pub fn update_title(&mut self, new_title: &str) -> Result<AppState, CoreError> {
        let trimmed = new_title.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptyTitle);
        }
        self.state.title = trimmed.to_string();
        Ok(self.state.clone())
    }
}

impl Default for StateEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StorageEngine {
    conn: rusqlite::Connection,
}

impl StorageEngine {
    pub fn in_memory() -> Result<Self, CoreError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Self::init(conn)
    }

    pub fn open(path: &str) -> Result<Self, CoreError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Self::init(conn)
    }

    fn init(conn: rusqlite::Connection) -> Result<Self, CoreError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS kv_store (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Ok(Self { conn })
    }

    pub fn set_item(&self, key: &str, value: &str) -> Result<(), CoreError> {
        self.conn
            .execute(
                "INSERT INTO kv_store (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Ok(())
    }

    pub fn get_item(&self, key: &str) -> Result<Option<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM kv_store WHERE key = ?1")
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        let mut rows = stmt
            .query(rusqlite::params![key])
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| CoreError::StorageError(e.to_string()))? {
            let val: String = row.get(0).map_err(|e| CoreError::StorageError(e.to_string()))?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn list_items(&self) -> Result<Vec<(String, String)>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM kv_store ORDER BY key")
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| CoreError::StorageError(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn delete_item(&self, key: &str) -> Result<bool, CoreError> {
        let count = self
            .conn
            .execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![key])
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Ok(count > 0)
    }
}

pub fn greet(name: &str) -> String {
    format!("Olá, {}! Rodando diretamente do núcleo em Rust.", name.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(
            greet("Desenvolvedor"),
            "Olá, Desenvolvedor! Rodando diretamente do núcleo em Rust."
        );
    }

    #[test]
    fn test_increment_and_decrement() {
        let mut engine = StateEngine::new();
        assert_eq!(engine.get_state().counter, 0);

        let s1 = engine.increment();
        assert_eq!(s1.counter, 1);

        let s2 = engine.decrement().expect("decrement deve funcionar acima de zero");
        assert_eq!(s2.counter, 0);

        let err = engine.decrement().unwrap_err();
        assert_eq!(err, CoreError::NegativeValue);
    }

    #[test]
    fn test_update_title() {
        let mut engine = StateEngine::new();
        assert!(engine.update_title("").is_err());
        assert!(engine.update_title("   ").is_err());
        let s = engine.update_title("Novo App").expect("título válido");
        assert_eq!(s.title, "Novo App");
    }

    #[test]
    fn test_sqlite_storage() {
        let store = StorageEngine::in_memory().expect("deve abrir in-memory");
        assert_eq!(store.get_item("chave").unwrap(), None);

        store.set_item("chave", "valor1").unwrap();
        assert_eq!(store.get_item("chave").unwrap(), Some("valor1".into()));

        store.set_item("chave", "valor2").unwrap();
        assert_eq!(store.get_item("chave").unwrap(), Some("valor2".into()));

        let items = store.list_items().unwrap();
        assert_eq!(items, vec![("chave".into(), "valor2".into())]);

        assert!(store.delete_item("chave").unwrap());
        assert_eq!(store.get_item("chave").unwrap(), None);
    }

}
