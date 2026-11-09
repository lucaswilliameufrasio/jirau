// As traduções (locales/*.yml) são embutidas no binário pelo rust-i18n em
// tempo de compilação; sem isto, editar um .yml não recompilaria o jirau.
fn main() {
    println!("cargo:rerun-if-changed=locales");
}
