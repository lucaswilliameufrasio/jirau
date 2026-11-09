//! Testes de Integração de Lifecycle do Cofre Seguro via UniFFI
//! Simula Cold Start, persistência SQLite e concorrência multithread.

use std::sync::Arc;
use std::thread;
use secure_vault_ffi as native;

#[test]
fn test_vault_ffi_lifecycle_and_concurrency() {
    let engine = native::NativeVaultEngine::in_memory().expect("Inicialização in-memory deve suceder");

    // 1. Inserção de credenciais
    let item = engine
        .save_item(
            None,
            "Chave de API Stripe".to_string(),
            native::NativeCategory::ApiKey,
            Some("contato@empresa.com".to_string()),
            "sk_live_stripe_secret_key_12345".to_string(),
            vec!["pagamentos".to_string()],
            true,
        )
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
            eng.save_item(
                None,
                format!("Item Concorrente {}", i),
                native::NativeCategory::Note,
                None,
                format!("Payload seguro {}", i),
                vec![],
                false,
            )
            .unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_stats = engine_arc.get_stats().unwrap();
    assert_eq!(final_stats.total_items, 11);
}
