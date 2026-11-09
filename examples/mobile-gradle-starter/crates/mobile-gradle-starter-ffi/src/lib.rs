use std::sync::Mutex;
use mobile_gradle_starter_core as core;

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

}

impl From<core::CoreError> for NativeError {
    fn from(err: core::CoreError) -> Self {
        match err {
            core::CoreError::NegativeValue => NativeError::NegativeValue,
            core::CoreError::EmptyTitle => NativeError::EmptyTitle,

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

#[uniffi::export]
pub fn native_greet(name: String) -> String {
    core::greet(&name)
}
