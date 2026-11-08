//! Teste de integração end-to-end simulando o ciclo de vida nativo (App Launch, State Mutation, Multi-threading)
use desktop_first_sqlite_ffi as native;
use std::sync::Arc;
use std::thread;

#[test]
fn test_native_app_lifecycle_and_state_flow() {
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

    // 4. Teste de Concorrência Simulando UI Threads Concorrentes
    let engine_arc = Arc::new(engine);
    let mut handles = Vec::new();

    for _ in 0..10 {
        let engine_clone = Arc::clone(&engine_arc);
        handles.push(thread::spawn(move || {
            engine_clone.increment();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_state = engine_arc.get_state();
    assert_eq!(final_state.counter, 10);
}
