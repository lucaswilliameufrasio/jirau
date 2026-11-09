use std::sync::Mutex;
use secure_vault_core as core;

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
    Database { message: String },
    NotFound { id: String },
    InvalidInput,
}

impl From<core::VaultError> for NativeVaultError {
    fn from(err: core::VaultError) -> Self {
        match err {
            core::VaultError::DatabaseError(msg) => NativeVaultError::Database { message: msg },
            core::VaultError::NotFound(id) => NativeVaultError::NotFound { id },
            core::VaultError::InvalidInput => NativeVaultError::InvalidInput,
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
        Ok(Self { inner: Mutex::new(engine) })
    }

    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Self, NativeVaultError> {
        let engine = core::VaultEngine::open(&path).map_err(NativeVaultError::from)?;
        Ok(Self { inner: Mutex::new(engine) })
    }

    pub fn save_item(
        &self,
        id: Option<String>,
        title: String,
        category: NativeCategory,
        username: Option<String>,
        secret_payload: String,
        tags: Vec<String>,
        is_favorite: bool,
    ) -> Result<NativeVaultItem, NativeVaultError> {
        let engine = self.inner.lock().unwrap();
        let item = engine
            .save_item(id, title, category.into(), username, secret_payload, tags, is_favorite)
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
