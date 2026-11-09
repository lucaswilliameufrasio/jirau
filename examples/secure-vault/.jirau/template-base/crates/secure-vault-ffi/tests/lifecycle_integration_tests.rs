//! Testes de Integração de Lifecycle do Cofre Seguro via UniFFI
//! Simula Cold Start, persistência SQLite e concorrência multithread.

use secure_vault_ffi as native;
use std::sync::Arc;
use std::thread;

#[test]
fn test_vault_ffi_lifecycle_and_concurrency() {
    let engine =
        native::NativeVaultEngine::in_memory().expect("Inicialização in-memory deve suceder");

    // 1. Inserção de credenciais
    let item = engine
        .save_item(native::NativeVaultItemInput {
            id: None,
            title: "Chave de API Stripe".to_string(),
            category: native::NativeCategory::ApiKey,
            username: Some("contato@empresa.com".to_string()),
            secret_payload: "sk_live_stripe_secret_key_12345".to_string(),
            tags: vec!["pagamentos".to_string()],
            is_favorite: true,
        })
        .expect("Inserção de item no cofre");

    assert_eq!(item.title, "Chave de API Stripe");
    assert!(item.is_favorite);

    // 2. Consulta e métricas
    let fetched = engine.get_item(item.id.clone()).expect("Consulta por ID");
    assert_eq!(fetched.secret_payload, "sk_live_stripe_secret_key_12345");

    let stats = engine.get_stats().expect("Cálculo de estatísticas");
    assert_eq!(stats.total_items, 1);
    assert_eq!(stats.apikeys_count, 1);

    // 3. Teste do Gerador de Senhas
    let pwd = native::generate_secure_password(native::NativePasswordOptions {
        length: 20,
        include_uppercase: true,
        include_lowercase: true,
        include_numbers: true,
        include_symbols: true,
    });
    assert_eq!(pwd.len(), 20);

    let eval = native::evaluate_password(pwd);
    assert!(eval.is_secure);

    // 4. Teste de Concorrência Multithread
    let engine_arc = Arc::new(engine);
    let mut handles = Vec::new();

    for i in 0..10 {
        let eng = Arc::clone(&engine_arc);
        handles.push(thread::spawn(move || {
            eng.save_item(native::NativeVaultItemInput {
                id: None,
                title: format!("Item Concorrente {}", i),
                category: native::NativeCategory::Note,
                username: None,
                secret_payload: format!("Payload seguro {}", i),
                tags: vec![],
                is_favorite: false,
            })
            .unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_stats = engine_arc.get_stats().unwrap();
    assert_eq!(final_stats.total_items, 11);
}

#[test]
fn test_vault_ffi_encrypted_persistence_requires_the_same_key() {
    let path = std::env::temp_dir().join(format!("ffi-vault-{}.sqlite3", std::process::id()));
    let path_str = path.to_str().unwrap().to_string();
    let _ = std::fs::remove_file(&path);
    let key = native::generate_vault_key();
    assert_eq!(key.len(), 32);

    let id = {
        let engine = native::NativeVaultEngine::open(path_str.clone(), key.clone()).unwrap();
        engine
            .save_item(native::NativeVaultItemInput {
                id: None,
                title: "Banco".to_string(),
                category: native::NativeCategory::Login,
                username: None,
                secret_payload: "SegredoFFI#2026".to_string(),
                tags: vec![],
                is_favorite: false,
            })
            .unwrap()
            .id
    };

    let reopened = native::NativeVaultEngine::open(path_str.clone(), key).unwrap();
    assert_eq!(
        reopened.get_item(id).unwrap().secret_payload,
        "SegredoFFI#2026"
    );
    drop(reopened);

    assert!(matches!(
        native::NativeVaultEngine::open(path_str.clone(), native::generate_vault_key()),
        Err(native::NativeVaultError::WrongKeyOrCorrupted)
    ));
    assert!(matches!(
        native::NativeVaultEngine::open(path_str, vec![1, 2, 3]),
        Err(native::NativeVaultError::InvalidKey)
    ));
    let _ = std::fs::remove_file(&path);
}
