"""Gera src/templates/vault.rs a partir de examples/secure-vault.

O exemplo é a fonte da verdade do template "vault" (é ele que roda no
simulador/emulador). Depois de editá-lo, rode: python3 scripts/sync-vault-template.py
"""
import re
import os
ROOT=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EX=ROOT+'/examples/secure-vault'
T=ROOT+'/src/templates/vault.rs'
PKG_PATH='android/app/src/main/java/io/github/jirau/vault'
TEST_PATH='android/app/src/androidTest/java/io/github/jirau/vault'

def tpl(content):
    out=content
    out=out.replace('io.github.jirau.vault','__PACKAGE__')
    out=out.replace('secure_vault','__RUST_NAME__')
    out=out.replace('secure-vault','__SLUG__')
    out=out.replace('SecureVault','__PASCAL__')
    out=out.replace('Secure Vault','__NAME__')
    return out

REPL={'__NAME__':'&ctx.name','__PACKAGE__':'&ctx.package_id','__RUST_NAME__':'&ctx.rust_name','__SLUG__':'&ctx.slug','__PASCAL__':'&ctx.pascal_name'}

def make_fn(name, path, doc):
    body=tpl(open(f'{EX}/{path}').read())
    assert '"####' not in body
    used=[k for k in REPL if k in body]
    arg='ctx' if used else '_ctx'
    chain=''.join(f'\n        .replace("{k}", {REPL[k]})' for k in used)
    if not used: chain='\n        .to_string()'
    return f'{doc}pub fn {name}({arg}: &TemplateContext) -> String {{\n    r####"{body}"####{chain}\n}}\n'

FNS=[
 ('vault_core_lib_rs','crates/secure-vault-core/src/lib.rs','/// Núcleo do cofre: SQLite + segredos cifrados (AES-256-GCM-SIV via aws-lc-rs).\n'),
 ('vault_ffi_lib_rs','crates/secure-vault-ffi/src/lib.rs','/// Camada UniFFI do cofre.\n'),
 ('vault_ffi_lifecycle_test_rs','crates/secure-vault-ffi/tests/lifecycle_integration_tests.rs',''),
 ('vault_desktop_main_rs','crates/secure-vault-desktop/src/main.rs',''),
 ('vault_android_main_activity_kt',f'{PKG_PATH}/MainActivity.kt','/// UI Compose do cofre.\n'),
 ('vault_android_core_kt',f'{PKG_PATH}/VaultCore.kt','/// ViewModel + chave no Android Keystore.\n'),
 ('vault_android_tests_kt',f'{TEST_PATH}/NativeLifecycleTest.kt','/// Testes instrumentados (ViewModel, Keystore e UI) contra o núcleo Rust real.\n'),
 ('vault_ios_content_view_swift','ios/App/Sources/ContentView.swift',''),
 ('vault_ios_tests_swift','ios/App/Tests/NativeLifecycleTests.swift',''),
]

HEADER = """//! Template "vault" (cofre de senhas). As funções de conteúdo abaixo são
//! geradas a partir de examples/secure-vault (validado em simulador/emulador)
//! por `scripts/sync-vault-template.py`; edite o exemplo e re-sincronize.

use super::{TemplateContext, versions};

pub fn vault_core_cargo_toml(ctx: &TemplateContext) -> String {
    format!(
        r#"[package]
name = "{slug}-core"
version = "0.1.0"
edition = "{edition}"

[dependencies]
serde = {{ version = "{serde}", features = ["derive"] }}
serde_json = "{serde_json}"
rusqlite = {{ version = "{rusqlite}", features = ["bundled"] }}
uuid = {{ version = "{uuid}", features = ["v4", "fast-rng"] }}
thiserror = "{thiserror}"
aws-lc-rs = "{aws_lc_rs}"
"#,
        slug = ctx.slug,
        edition = versions::RUST_EDITION,
        serde = versions::SERDE,
        serde_json = versions::SERDE_JSON,
        rusqlite = versions::RUSQLITE,
        uuid = versions::UUID,
        thiserror = versions::THISERROR,
        aws_lc_rs = versions::AWS_LC_RS,
    )
}
"""
s = HEADER + '\n' + '\n'.join(make_fn(*f) for f in FNS)
open(T,'w').write(s)
print('src/templates/vault.rs sincronizado com examples/secure-vault')
