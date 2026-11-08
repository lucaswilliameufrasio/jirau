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
        let engine = core::StorageEngine::in_memory().map_err(Into::into)?;
        Ok(Self { inner: Mutex::new(engine) })
    }

    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Self, NativeError> {
        let engine = core::StorageEngine::open(&path).map_err(Into::into)?;
        Ok(Self { inner: Mutex::new(engine) })
    }

    pub fn set_item(&self, key: String, value: String) -> Result<(), NativeError> {
        let engine = self.inner.lock().unwrap();
        engine.set_item(&key, &value).map_err(Into::into)
    }

    pub fn get_item(&self, key: String) -> Result<Option<String>, NativeError> {
        let engine = self.inner.lock().unwrap();
        engine.get_item(&key).map_err(Into::into)
    }

    pub fn list_items(&self) -> Result<Vec<NativeStorageItem>, NativeError> {
        let engine = self.inner.lock().unwrap();
        let items = engine.list_items().map_err(Into::into)?;
        Ok(items.into_iter().map(|(k, v)| NativeStorageItem { key: k, value: v }).collect())
    }

    pub fn delete_item(&self, key: String) -> Result<bool, NativeError> {
        let engine = self.inner.lock().unwrap();
        engine.delete_item(&key).map_err(Into::into)
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
use {slug}_core as core;

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
        engine.decrement().map(Into::into).map_err(Into::into)
    }}

    pub fn update_title(&self, new_title: String) -> Result<NativeAppState, NativeError> {{
        let mut engine = self.inner.lock().unwrap();
        engine.update_title(&new_title).map(Into::into).map_err(Into::into)
    }}
}}
{sqlite_ffi}
#[uniffi::export]
pub fn native_greet(name: String) -> String {{
    core::greet(&name)
}}
"#,
        slug = ctx.slug,
        storage_error_enum = storage_error_enum,
        storage_error_match = storage_error_match,
        sqlite_ffi = sqlite_ffi
    )
}
