use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("O valor não pode ser negativo")]
    NegativeValue,
    #[error("O título não pode ser vazio")]
    EmptyTitle,

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppState {
    pub counter: i32,
    pub title: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            counter: 0,
            title: "Jirau Native App".to_string(),
        }
    }
}

pub struct StateEngine {
    state: AppState,
}

impl StateEngine {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
        }
    }

    pub fn get_state(&self) -> AppState {
        self.state.clone()
    }

    pub fn increment(&mut self) -> AppState {
        self.state.counter += 1;
        self.state.clone()
    }

    pub fn decrement(&mut self) -> Result<AppState, CoreError> {
        if self.state.counter <= 0 {
            return Err(CoreError::NegativeValue);
        }
        self.state.counter -= 1;
        Ok(self.state.clone())
    }

    pub fn update_title(&mut self, new_title: &str) -> Result<AppState, CoreError> {
        let trimmed = new_title.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptyTitle);
        }
        self.state.title = trimmed.to_string();
        Ok(self.state.clone())
    }
}

impl Default for StateEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn greet(name: &str) -> String {
    format!("Olá, {}! Rodando diretamente do núcleo em Rust.", name.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(
            greet("Desenvolvedor"),
            "Olá, Desenvolvedor! Rodando diretamente do núcleo em Rust."
        );
    }

    #[test]
    fn test_increment_and_decrement() {
        let mut engine = StateEngine::new();
        assert_eq!(engine.get_state().counter, 0);

        let s1 = engine.increment();
        assert_eq!(s1.counter, 1);

        let s2 = engine.decrement().expect("decrement deve funcionar acima de zero");
        assert_eq!(s2.counter, 0);

        let err = engine.decrement().unwrap_err();
        assert_eq!(err, CoreError::NegativeValue);
    }

    #[test]
    fn test_update_title() {
        let mut engine = StateEngine::new();
        assert!(engine.update_title("").is_err());
        assert!(engine.update_title("   ").is_err());
        let s = engine.update_title("Novo App").expect("título válido");
        assert_eq!(s.title, "Novo App");
    }

}
