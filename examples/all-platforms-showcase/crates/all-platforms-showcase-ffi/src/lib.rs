use std::sync::Mutex;
use all_platforms_showcase_core as core;

uniffi::setup_scaffolding!();

#[derive(uniffi::Record)]
pub struct NativeAppState {
    pub counter: i32,
    pub title: String,
}

impl From<core::AppState> for NativeAppState {
    fn from(s: core::AppState) -> Self {
        Self {
            counter: s.counter,
            title: s.title,
        }
    }
}

#[derive(uniffi::Error, Debug)]
pub enum NativeError {
    NegativeValue,
    EmptyTitle,
    StorageError { message: String },
}

impl From<core::CoreError> for NativeError {
    fn from(err: core::CoreError) -> Self {
        match err {
            core::CoreError::NegativeValue => NativeError::NegativeValue,
            core::CoreError::EmptyTitle => NativeError::EmptyTitle,
            core::CoreError::StorageError(msg) => NativeError::StorageError { message: msg },
        }
    }
}

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for NativeError {}

#[derive(uniffi::Object)]
pub struct AppEngine {
    inner: Mutex<core::StateEngine>,
}

#[uniffi::export]
impl AppEngine {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(core::StateEngine::new()),
        }
    }

    pub fn get_state(&self) -> NativeAppState {
        let engine = self.inner.lock().unwrap();
        engine.get_state().into()
    }

    pub fn increment(&self) -> NativeAppState {
        let mut engine = self.inner.lock().unwrap();
        engine.increment().into()
    }

    pub fn decrement(&self) -> Result<NativeAppState, NativeError> {
        let mut engine = self.inner.lock().unwrap();
        engine.decrement().map(Into::into).map_err(NativeError::from)
    }

    pub fn update_title(&self, new_title: String) -> Result<NativeAppState, NativeError> {
        let mut engine = self.inner.lock().unwrap();
        engine.update_title(&new_title).map(Into::into).map_err(NativeError::from)
    }
}

#[derive(uniffi::Record)]
pub struct NativeStorageItem {
    pub key: String,
    pub value: String,
}

#[derive(uniffi::Object)]
pub struct NativeStorage {
    inner: Mutex<core::StorageEngine>,
}

#[uniffi::export]
impl NativeStorage {
    #[uniffi::constructor]
    pub fn in_memory() -> Result<Self, NativeError> {
        let engine = core::StorageEngine::in_memory().map_err(NativeError::from)?;
        Ok(Self { inner: Mutex::new(engine) })
    }

    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Self, NativeError> {
        let engine = core::StorageEngine::open(&path).map_err(NativeError::from)?;
        Ok(Self { inner: Mutex::new(engine) })
    }

    pub fn set_item(&self, key: String, value: String) -> Result<(), NativeError> {
        let engine = self.inner.lock().unwrap();
        engine.set_item(&key, &value).map_err(NativeError::from)
    }

    pub fn get_item(&self, key: String) -> Result<Option<String>, NativeError> {
        let engine = self.inner.lock().unwrap();
        engine.get_item(&key).map_err(NativeError::from)
    }

    pub fn list_items(&self) -> Result<Vec<NativeStorageItem>, NativeError> {
        let engine = self.inner.lock().unwrap();
        let items = engine.list_items().map_err(NativeError::from)?;
        Ok(items.into_iter().map(|(k, v)| NativeStorageItem { key: k, value: v }).collect())
    }

    pub fn delete_item(&self, key: String) -> Result<bool, NativeError> {
        let engine = self.inner.lock().unwrap();
        engine.delete_item(&key).map_err(NativeError::from)
    }
}

#[uniffi::export]
pub fn native_greet(name: String) -> String {
    core::greet(&name)
}
