use secure_vault_core as core;
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
