use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

fn contracts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("could not find repo root")
        .join("contracts")
}

pub fn fixtures_dir() -> PathBuf {
    contracts_dir().join("fixtures")
}

pub fn schemas_dir() -> PathBuf {
    contracts_dir().join("schemas")
}

pub fn load_fixture(rel_path: &str) -> serde_json::Value {
    let path = fixtures_dir().join(rel_path);
    let data = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {e}", path.display()))
}

pub fn load_fixture_as<T: DeserializeOwned>(rel_path: &str) -> T {
    let path = fixtures_dir().join(rel_path);
    let data = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&data).unwrap_or_else(|e| {
        panic!(
            "Failed to parse fixture {} as {}: {e}",
            path.display(),
            std::any::type_name::<T>()
        )
    })
}

pub fn load_schema(schema_name: &str) -> serde_json::Value {
    let path = schemas_dir().join(format!("{schema_name}.schema.json"));
    let data = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read schema {}: {e}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("Failed to parse schema {}: {e}", path.display()))
}
