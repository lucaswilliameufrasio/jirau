pub mod android;
pub mod ios;
pub mod root;
pub mod rust_core;
pub mod rust_ffi;

pub struct TemplateContext {
    pub name: String,
    pub slug: String,           // ex: "meu-app"
    pub rust_name: String,      // ex: "meu_app"
    pub pascal_name: String,    // ex: "MeuApp"
    pub package_id: String,     // ex: "com.example.meuapp"
    pub package_path: String,   // ex: "com/example/meuapp"
    pub platforms: Vec<String>, // ["android", "ios"]
}

impl TemplateContext {
    pub fn new(name: &str, package_id: &str, platforms: Vec<String>) -> Self {
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
        }
    }
}
