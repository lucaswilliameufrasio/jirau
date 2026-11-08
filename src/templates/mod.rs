pub mod android;
pub mod desktop;
pub mod ios;
pub mod root;
pub mod rust_core;
pub mod rust_ffi;

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
    pub template_version: String,     // "0.2.0"
}

impl TemplateContext {
    #[allow(dead_code)]
    pub fn new(name: &str, package_id: &str, platforms: Vec<String>) -> Self {
        Self::with_options(name, package_id, platforms, "none", "gradle", "0.2.0")
    }

    pub fn with_options(
        name: &str,
        package_id: &str,
        platforms: Vec<String>,
        storage: &str,
        android_build_system: &str,
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
            template_version: template_version.to_string(),
        }
    }
}
