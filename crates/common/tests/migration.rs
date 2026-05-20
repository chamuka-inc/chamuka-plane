#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;

#[test]
fn migration_file_exists_and_sorts_after_plane() {
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent")
        .join("plane/schema/migrations");

    assert!(
        migrations_dir.exists(),
        "migrations directory does not exist: {migrations_dir:?}"
    );

    let our_migration = migrations_dir.join("20260601000000_add-sandbox-platform.sql");
    assert!(
        our_migration.exists(),
        "our migration file does not exist: {our_migration:?}"
    );

    let mut filenames: Vec<String> = fs::read_dir(&migrations_dir)
        .expect("read migrations dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".sql"))
        .collect();
    filenames.sort();

    let our_filename = "20260601000000_add-sandbox-platform.sql";
    let our_pos = filenames
        .iter()
        .position(|f| f == our_filename)
        .expect("our migration in sorted list");

    // Our base migration must sort after all upstream Plane migrations
    // (timestamps before 2026). Our own subsequent migrations may follow it.
    let last_upstream_pos = filenames
        .iter()
        .rposition(|f| f.as_str() < our_filename && f.starts_with("202") && !f.starts_with("2026"))
        .expect("at least one upstream Plane migration");

    assert!(
        our_pos > last_upstream_pos,
        "our migration must sort after all upstream Plane migrations"
    );
}

#[test]
fn migration_creates_expected_tables() {
    let migration_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent")
        .join("plane/schema/migrations/20260601000000_add-sandbox-platform.sql");

    let sql = fs::read_to_string(&migration_path).expect("read migration");

    let expected_tables = [
        "tenant",
        "sandbox",
        "snapshot",
        "network_policy",
        "secret",
        "credential_binding",
        "extension_action",
        "usage_record",
        "port_route",
        "audit_egress",
    ];

    for table in expected_tables {
        assert!(
            sql.contains(&format!("CREATE TABLE {table}")),
            "migration must create table '{table}'"
        );
    }
}

#[test]
fn migration_references_plane_tables() {
    let migration_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent")
        .join("plane/schema/migrations/20260601000000_add-sandbox-platform.sql");

    let sql = fs::read_to_string(&migration_path).expect("read migration");

    assert!(
        sql.contains("REFERENCES backend(id)"),
        "sandbox must reference Plane's backend table"
    );
    assert!(
        sql.contains("REFERENCES drone(id)"),
        "extension_action must reference Plane's drone table"
    );
}
