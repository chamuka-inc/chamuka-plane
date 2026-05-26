#![allow(clippy::expect_used, clippy::print_stderr, clippy::print_stdout)]

mod manifest;
mod python;
mod registry;
mod schema;
mod typescript;

use clap::Parser;
use registry::TypeRegistry;
use schema::{extract_definitions, parse_definition, parse_schema};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "generate-sdk-types",
    about = "Generate TypeScript and Python SDK types from JSON Schema files"
)]
struct Cli {
    /// Generate only for a specific language: ts or py
    #[arg(long)]
    lang: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("could not find repo root");

    let schemas_dir = repo_root.join("contracts").join("schemas");
    let ts_output = repo_root
        .join("sdks")
        .join("typescript")
        .join("src")
        .join("generated")
        .join("types.ts");
    let py_output_dir = repo_root
        .join("sdks")
        .join("python")
        .join("src")
        .join("sandbox_platform")
        .join("generated");
    let py_output = py_output_dir.join("types.py");
    let py_init = py_output_dir.join("__init__.py");

    let emit_ts = cli.lang.as_deref() != Some("py");
    let emit_py = cli.lang.as_deref() != Some("ts");

    // Build a set of manifest schema filenames for lookup
    let manifest_filenames: BTreeSet<&str> = manifest::MANIFEST.iter().map(|(f, _)| *f).collect();

    let mut registry = TypeRegistry::new();

    // Process each manifest entry
    for (filename, type_name) in manifest::MANIFEST {
        let schema_path = schemas_dir.join(format!("{filename}.schema.json"));
        let content = fs::read_to_string(&schema_path).unwrap_or_else(|e| {
            panic!("Failed to read {}: {e}", schema_path.display());
        });
        let schema: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
            panic!("Failed to parse {}: {e}", schema_path.display());
        });

        // Parse the top-level schema as the manifest type name
        let typedef = parse_schema(type_name, &schema);
        registry.insert(typedef);

        // Parse definitions from this schema and add them to the registry
        let definitions = extract_definitions(&schema);
        for (def_name, def_schema) in &definitions {
            // Skip definitions that duplicate manifest top-level types
            // (they'll be inserted by their own schema file)
            if manifest_filenames.contains(def_name.to_ascii_lowercase().replace(' ', "-").as_str())
            {
                // This is a top-level type that has its own schema file; it
                // will be (or already was) inserted from there.
                continue;
            }
            let def = parse_definition(def_name, def_schema);
            registry.insert(def);
        }
    }

    // Get types in dependency order
    let sorted_types = registry.into_sorted();

    // Filter to only types that are either in the manifest or referenced by
    // manifest types (definitions). We already collected everything needed.

    if emit_ts {
        let ts_content = typescript::emit(&sorted_types);
        write_if_changed(&ts_output, &ts_content, "TypeScript");
    }

    if emit_py {
        fs::create_dir_all(&py_output_dir).unwrap_or_else(|e| {
            panic!(
                "Failed to create directory {}: {e}",
                py_output_dir.display()
            );
        });
        // Create __init__.py if it doesn't exist
        if !py_init.exists() {
            fs::write(&py_init, "").unwrap_or_else(|e| {
                panic!("Failed to write {}: {e}", py_init.display());
            });
        }
        let py_content = python::emit(&sorted_types);
        write_if_changed(&py_output, &py_content, "Python");
    }

    let lang_label = match cli.lang.as_deref() {
        Some("ts") => "TypeScript",
        Some("py") => "Python",
        _ => "TypeScript + Python",
    };
    let type_count = manifest::MANIFEST.len();
    eprintln!(
        "SDK types: {type_count} manifest types ({lang_label}) — {} total definitions emitted",
        sorted_types.len()
    );
}

fn write_if_changed(path: &Path, content: &str, label: &str) {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            panic!("Failed to create directory {}: {e}", parent.display());
        });
    }

    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        eprintln!("  {label}: unchanged");
    } else {
        fs::write(path, content).unwrap_or_else(|e| {
            panic!("Failed to write {}: {e}", path.display());
        });
        eprintln!("  {label}: written to {}", path.display());
    }
}
