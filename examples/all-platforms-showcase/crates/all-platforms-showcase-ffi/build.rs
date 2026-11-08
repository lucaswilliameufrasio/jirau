fn main() {
    uniffi::generate_scaffolding("src/lib.rs").unwrap_or_default();
}
