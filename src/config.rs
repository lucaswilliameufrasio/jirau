use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = "jirau.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JirauConfig {
    pub app: AppConfig,
    #[serde(default)]
    pub dev: DevConfig,
    #[serde(default)]
    pub meta: MetaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub package_id: String,
    pub version: String,
    #[serde(default = "default_platforms")]
    pub platforms: Vec<String>,
    #[serde(default = "default_storage")]
    pub storage: String,
    #[serde(default = "default_android_build_system")]
    pub android_build_system: String,
    #[serde(default = "default_ui_theme")]
    pub ui_theme: String,
    #[serde(default = "default_plugins")]
    pub plugins: Vec<String>,
    #[serde(default = "default_app_template")]
    pub template: String,
}

fn default_app_template() -> String {
    "starter".to_string()
}

fn default_platforms() -> Vec<String> {
    vec![
        "android".to_string(),
        "ios".to_string(),
        "desktop".to_string(),
    ]
}

fn default_storage() -> String {
    "none".to_string()
}

fn default_android_build_system() -> String {
    "gradle".to_string()
}

fn default_ui_theme() -> String {
    "default".to_string()
}

fn default_plugins() -> Vec<String> {
    Vec::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevConfig {
    #[serde(default = "default_android_abi")]
    pub default_android_abi: String,
    #[serde(default = "default_ios_sim")]
    pub default_ios_simulator: String,
    #[serde(default = "default_sccache")]
    pub use_sccache: bool,
}

fn default_android_abi() -> String {
    "arm64-v8a".to_string()
}

fn default_ios_sim() -> String {
    "iPhone 16".to_string()
}

fn default_sccache() -> bool {
    true
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            default_android_abi: default_android_abi(),
            default_ios_simulator: default_ios_sim(),
            use_sccache: default_sccache(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    #[serde(default = "default_template_version")]
    pub template_version: String,
}

fn default_template_version() -> String {
    "0.3.0".to_string()
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            template_version: default_template_version(),
        }
    }
}

impl JirauConfig {
    #[allow(dead_code)]
    pub fn new(
        name: &str,
        package_id: &str,
        platforms: Vec<String>,
        storage: &str,
        android_build_system: &str,
    ) -> Self {
        Self {
            app: AppConfig {
                name: name.to_string(),
                package_id: package_id.to_string(),
                version: "0.1.0".to_string(),
                platforms,
                storage: storage.to_string(),
                android_build_system: android_build_system.to_string(),
                ui_theme: "default".to_string(),
                plugins: Vec::new(),
                template: "starter".to_string(),
            },
            dev: DevConfig::default(),
            meta: MetaConfig::default(),
        }
    }

    pub fn find_in_current_or_parent() -> Result<(PathBuf, Self)> {
        let mut curr =
            std::env::current_dir().context("Não foi possível obter o diretório atual")?;
        loop {
            let candidate = curr.join(CONFIG_FILE_NAME);
            if candidate.exists() {
                let content = fs::read_to_string(&candidate)
                    .with_context(|| format!("Falha ao ler {}", candidate.display()))?;
                let config: Self = toml::from_str(&content)
                    .with_context(|| format!("Falha ao parsear {}", candidate.display()))?;
                return Ok((curr, config));
            }

            if !curr.pop() {
                break;
            }
        }
        anyhow::bail!(
            "Nenhum arquivo '{}' foi encontrado no diretório atual nem nos superiores.\nExecute este comando na raiz de um projeto jirau ou use 'jirau new <nome>' para criar um novo projeto.",
            CONFIG_FILE_NAME
        );
    }

    #[allow(dead_code)]
    pub fn write_to_dir(&self, dir: &Path) -> Result<()> {
        let target = dir.join(CONFIG_FILE_NAME);
        let content =
            toml::to_string_pretty(self).context("Falha ao serializar configuração jirau.toml")?;
        fs::write(&target, content)
            .with_context(|| format!("Falha ao escrever {}", target.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_serialization_roundtrip() {
        let dir = tempdir().unwrap();
        let cfg = JirauConfig::new(
            "meu-app",
            "com.example.meuapp",
            vec!["android".into(), "ios".into(), "desktop".into()],
            "sqlite",
            "amper",
        );
        cfg.write_to_dir(dir.path()).unwrap();

        let read_back = fs::read_to_string(dir.path().join(CONFIG_FILE_NAME)).unwrap();
        let parsed: JirauConfig = toml::from_str(&read_back).unwrap();
        assert_eq!(parsed.app.name, "meu-app");
        assert_eq!(parsed.app.package_id, "com.example.meuapp");
        assert_eq!(parsed.app.platforms, vec!["android", "ios", "desktop"]);
        assert_eq!(parsed.app.storage, "sqlite");
        assert_eq!(parsed.app.android_build_system, "amper");
        assert_eq!(parsed.app.ui_theme, "default");
        assert_eq!(parsed.dev.default_android_abi, "arm64-v8a");
        assert_eq!(parsed.meta.template_version, "0.3.0");
    }
}
