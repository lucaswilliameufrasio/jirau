use super::TemplateContext;

pub fn ffi_cargo_toml(ctx: &TemplateContext) -> String {
    format!(
        r#"[package]
name = "{slug}-ffi"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]
name = "{rust_name}_ffi"

[dependencies]
{slug}-core = {{ path = "../{slug}-core" }}
uniffi = {{ version = "0.28", features = ["cli"] }}

[build-dependencies]
uniffi = {{ version = "0.28", features = ["build"] }}
"#,
        slug = ctx.slug,
        rust_name = ctx.rust_name
    )
}

pub fn ffi_uniffi_toml(ctx: &TemplateContext) -> String {
    format!(
        r#"[bindings.kotlin]
package_name = "{package_id}.uniffi"

[bindings.swift]
module_name = "{pascal_name}FFI"
"#,
        package_id = ctx.package_id,
        pascal_name = ctx.pascal_name
    )
}

pub fn ffi_build_rs() -> String {
    r#"fn main() {
    uniffi::generate_scaffolding("src/lib.rs").unwrap_or_default();
}
"#
    .to_string()
}

pub fn ffi_lifecycle_test_rs(ctx: &TemplateContext) -> String {
    let sqlite_checks = if ctx.storage == "sqlite" {
        r#"
    // 3. Validação do Lifecycle do NativeStorage (SQLite)
    let storage = native::NativeStorage::in_memory().expect("deve inicializar storage em memória");
    assert!(storage.get_item("app_theme".to_string()).unwrap().is_none());

    storage.set_item("app_theme".to_string(), "dark".to_string()).unwrap();
    assert_eq!(storage.get_item("app_theme".to_string()).unwrap(), Some("dark".to_string()));

    let items = storage.list_items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].key, "app_theme");
    assert_eq!(items[0].value, "dark");

    assert!(storage.delete_item("app_theme".to_string()).unwrap());
    assert!(storage.get_item("app_theme".to_string()).unwrap().is_none());
"#
    } else {
        ""
    };

    format!(
        r#"//! Teste de integração end-to-end simulando o ciclo de vida nativo (App Launch, State Mutation, Multi-threading)
use {rust_name}_ffi as native;
use std::sync::Arc;
use std::thread;

#[test]
fn test_native_app_lifecycle_and_state_flow() {{
    // 1. Inicialização e Cold Start
    let engine = native::AppEngine::new();
    let initial_state = engine.get_state();
    assert_eq!(initial_state.counter, 0);
    assert_eq!(initial_state.title, "Jirau Native App");

    // 2. Mutações de Estado e Limites de Erro
    let state_after_inc = engine.increment();
    assert_eq!(state_after_inc.counter, 1);

    let state_after_dec = engine.decrement().expect("decremento deve ter sucesso");
    assert_eq!(state_after_dec.counter, 0);

    // Validação de erro tipado exportado via UniFFI
    let dec_error = engine.decrement().expect_err("decremento abaixo de zero deve falhar");
    assert!(matches!(dec_error, native::NativeError::NegativeValue));

    let title_res = engine.update_title("Meu App Pro".to_string()).expect("título válido");
    assert_eq!(title_res.title, "Meu App Pro");

    let empty_title_error = engine.update_title("   ".to_string()).expect_err("título vazio deve falhar");
    assert!(matches!(empty_title_error, native::NativeError::EmptyTitle));
{sqlite_checks}
    // 4. Teste de Concorrência Simulando UI Threads Concorrentes
    let engine_arc = Arc::new(engine);
    let mut handles = Vec::new();

    for _ in 0..10 {{
        let engine_clone = Arc::clone(&engine_arc);
        handles.push(thread::spawn(move || {{
            engine_clone.increment();
        }}));
    }}

    for handle in handles {{
        handle.join().unwrap();
    }}

    let final_state = engine_arc.get_state();
    assert_eq!(final_state.counter, 10);
}}
"#,
        rust_name = ctx.rust_name,
        sqlite_checks = sqlite_checks
    )
}

pub fn ffi_lib_rs(ctx: &TemplateContext) -> String {
    let sqlite_ffi = if ctx.storage == "sqlite" {
        r#"
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
"#
    } else {
        ""
    };

    let storage_error_enum = if ctx.storage == "sqlite" {
        r#"    StorageError { message: String },"#
    } else {
        ""
    };

    let storage_error_match = if ctx.storage == "sqlite" {
        r#"            core::CoreError::StorageError(msg) => NativeError::StorageError { message: msg },"#
    } else {
        ""
    };

    format!(
        r#"use std::sync::Mutex;
use {rust_name}_core as core;

uniffi::setup_scaffolding!();

#[derive(uniffi::Record)]
pub struct NativeAppState {{
    pub counter: i32,
    pub title: String,
}}

impl From<core::AppState> for NativeAppState {{
    fn from(s: core::AppState) -> Self {{
        Self {{
            counter: s.counter,
            title: s.title,
        }}
    }}
}}

#[derive(uniffi::Error, Debug)]
pub enum NativeError {{
    NegativeValue,
    EmptyTitle,
{storage_error_enum}
}}

impl From<core::CoreError> for NativeError {{
    fn from(err: core::CoreError) -> Self {{
        match err {{
            core::CoreError::NegativeValue => NativeError::NegativeValue,
            core::CoreError::EmptyTitle => NativeError::EmptyTitle,
{storage_error_match}
        }}
    }}
}}

impl std::fmt::Display for NativeError {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        write!(f, "{{:?}}", self)
    }}
}}

impl std::error::Error for NativeError {{}}

#[derive(uniffi::Object)]
pub struct AppEngine {{
    inner: Mutex<core::StateEngine>,
}}

#[uniffi::export]
impl AppEngine {{
    #[uniffi::constructor]
    pub fn new() -> Self {{
        Self {{
            inner: Mutex::new(core::StateEngine::new()),
        }}
    }}

    pub fn get_state(&self) -> NativeAppState {{
        let engine = self.inner.lock().unwrap();
        engine.get_state().into()
    }}

    pub fn increment(&self) -> NativeAppState {{
        let mut engine = self.inner.lock().unwrap();
        engine.increment().into()
    }}

    pub fn decrement(&self) -> Result<NativeAppState, NativeError> {{
        let mut engine = self.inner.lock().unwrap();
        engine.decrement().map(Into::into).map_err(NativeError::from)
    }}

    pub fn update_title(&self, new_title: String) -> Result<NativeAppState, NativeError> {{
        let mut engine = self.inner.lock().unwrap();
        engine.update_title(&new_title).map(Into::into).map_err(NativeError::from)
    }}
}}
{sqlite_ffi}
#[uniffi::export]
pub fn native_greet(name: String) -> String {{
    core::greet(&name)
}}
"#,
        rust_name = ctx.rust_name,
        storage_error_enum = storage_error_enum,
        storage_error_match = storage_error_match,
        sqlite_ffi = sqlite_ffi
    )
}
