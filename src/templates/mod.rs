pub mod android;
pub mod desktop;
pub mod ios;
pub mod root;
pub mod rust_core;
pub mod rust_ffi;
pub mod vault;

pub struct TemplateContext {
    pub name: String,
    pub slug: String,                 // ex: "meu-app"
    pub rust_name: String,            // ex: "meu_app"
    pub pascal_name: String,          // ex: "MeuApp"
    pub package_id: String,           // ex: "com.example.meuapp"
    pub package_path: String,         // ex: "com/example/meuapp"
    pub platforms: Vec<String>,       // ["android", "ios", "desktop"]
    pub storage: String,              // "none" | "sqlite"
    pub android_build_system: String, // "gradle" | "amper"
    pub ui_theme: String,             // "default" | "material3-extended" | "minimal"
    pub plugins: Vec<String>,         // ["logger", "analytics"]
    pub app_template: String,         // "starter" | "vault"
    pub ci_workflow: bool,
    pub template_version: String, // "0.3.0"
}

impl TemplateContext {
    #[allow(dead_code)]
    pub fn new(name: &str, package_id: &str, platforms: Vec<String>) -> Self {
        Self::with_options(name, package_id, platforms, "none", "gradle", "0.3.0")
    }

    pub fn with_options(
        name: &str,
        package_id: &str,
        platforms: Vec<String>,
        storage: &str,
        android_build_system: &str,
        template_version: &str,
    ) -> Self {
        Self::with_all_options(
            name,
            package_id,
            platforms,
            storage,
            android_build_system,
            "default",
            Vec::new(),
            template_version,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_all_options(
        name: &str,
        package_id: &str,
        platforms: Vec<String>,
        storage: &str,
        android_build_system: &str,
        ui_theme: &str,
        plugins: Vec<String>,
        template_version: &str,
    ) -> Self {
        Self::with_full_spec(
            name,
            package_id,
            platforms,
            storage,
            android_build_system,
            ui_theme,
            plugins,
            "starter",
            false,
            template_version,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_full_spec(
        name: &str,
        package_id: &str,
        platforms: Vec<String>,
        storage: &str,
        android_build_system: &str,
        ui_theme: &str,
        plugins: Vec<String>,
        app_template: &str,
        ci_workflow: bool,
        template_version: &str,
    ) -> Self {
        let slug = name.to_lowercase().replace(['_', ' '], "-");
        let rust_name = slug.replace('-', "_");

        let pascal_name = slug
            .split('-')
            .filter(|s| !s.is_empty())
            .map(|word| {
                let mut c = word.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>();

        let package_path = package_id.replace('.', "/");

        Self {
            name: name.to_string(),
            slug,
            rust_name,
            pascal_name,
            package_id: package_id.to_string(),
            package_path,
            platforms,
            storage: storage.to_string(),
            android_build_system: android_build_system.to_string(),
            ui_theme: ui_theme.to_string(),
            plugins,
            app_template: app_template.to_string(),
            ci_workflow,
            template_version: template_version.to_string(),
        }
    }
}
