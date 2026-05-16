#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::sandbox::Sandbox;
use common::test_helpers::{fixtures_dir, load_fixture, load_fixture_as, load_schema, schemas_dir};

#[test]
fn fixtures_dir_exists() {
    assert!(fixtures_dir().exists());
    assert!(fixtures_dir().is_dir());
}

#[test]
fn schemas_dir_exists() {
    assert!(schemas_dir().exists());
    assert!(schemas_dir().is_dir());
}

#[test]
fn load_fixture_returns_value() {
    let val = load_fixture("sandbox/minimal.json");
    assert_eq!(val["id"], "sb-min001");
}

#[test]
fn load_fixture_as_typed() {
    let s: Sandbox = load_fixture_as("sandbox/full.json");
    assert_eq!(s.id, "sb-full789");
}

#[test]
fn load_schema_returns_value() {
    let schema = load_schema("sandbox");
    assert_eq!(schema["title"], "Sandbox");
    assert_eq!(schema["type"], "object");
}

#[test]
#[should_panic(expected = "Failed to read fixture")]
fn load_fixture_panics_on_missing() {
    let _ = load_fixture("does-not-exist.json");
}

#[test]
#[should_panic(expected = "Failed to parse fixture")]
fn load_fixture_as_panics_on_wrong_type() {
    let _: common::ext::usage::UsageRecord = load_fixture_as("sandbox/minimal.json");
}
