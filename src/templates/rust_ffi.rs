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
}}

impl From<core::CoreError> for NativeError {{
    fn from(err: core::CoreError) -> Self {{
        match err {{
            core::CoreError::NegativeValue => NativeError::NegativeValue,
            core::CoreError::EmptyTitle => NativeError::EmptyTitle,
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

#[uniffi::export]
pub fn native_greet(name: String) -> String {{
    core::greet(&name)
}}
"#,
        slug = ctx.slug
    )
}
