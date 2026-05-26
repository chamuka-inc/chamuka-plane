use serde_json::Value;
use std::collections::BTreeMap;

/// Intermediate representation for a single type definition.
#[derive(Debug, Clone)]
pub enum TypeDef {
    /// `{"type": "string", "enum": ["a", "b"]}` or documented oneOf enum
    StringEnum { name: String, variants: Vec<String> },
    /// `{"oneOf": [...]}` where each variant is an object with a single-value
    /// enum discriminator property
    TaggedUnion {
        name: String,
        discriminator: String,
        variants: Vec<UnionVariant>,
    },
    /// `{"type": "object", "properties": {...}}`
    Struct { name: String, fields: Vec<FieldDef> },
}

impl TypeDef {
    pub fn name(&self) -> &str {
        match self {
            TypeDef::StringEnum { name, .. }
            | TypeDef::TaggedUnion { name, .. }
            | TypeDef::Struct { name, .. } => name,
        }
    }

    /// Collect names of other types this definition references.
    pub fn dependencies(&self) -> Vec<String> {
        match self {
            TypeDef::StringEnum { .. } => vec![],
            TypeDef::TaggedUnion { variants, .. } => {
                let mut deps = Vec::new();
                for v in variants {
                    for f in &v.fields {
                        collect_type_refs(&f.ty, &mut deps);
                    }
                }
                deps
            }
            TypeDef::Struct { fields, .. } => {
                let mut deps = Vec::new();
                for f in fields {
                    collect_type_refs(&f.ty, &mut deps);
                }
                deps
            }
        }
    }
}

fn collect_type_refs(ty: &FieldType, out: &mut Vec<String>) {
    match ty {
        FieldType::Ref(name) => out.push(name.clone()),
        FieldType::Nullable(inner) => collect_type_refs(inner, out),
        FieldType::Array(inner) => collect_type_refs(inner, out),
        FieldType::Map(inner) => collect_type_refs(inner, out),
        _ => {}
    }
}

#[derive(Debug, Clone)]
pub struct UnionVariant {
    pub tag_value: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
    pub required: bool,
    pub has_default: bool,
    pub default_value: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Bool,
    Int,
    Float,
    Ref(String),
    Array(Box<FieldType>),
    Map(Box<FieldType>),
    Nullable(Box<FieldType>),
}

/// Parse a top-level JSON Schema object into a `TypeDef`.
pub fn parse_schema(name: &str, schema: &Value) -> TypeDef {
    // Pattern 1: Simple string enum
    if is_simple_string_enum(schema) {
        return parse_simple_string_enum(name, schema);
    }

    // Pattern 2: Documented string enum (oneOf of single-value enums, all strings)
    if is_documented_string_enum(schema) {
        return parse_documented_string_enum(name, schema);
    }

    // Pattern 3: Tagged union (oneOf with discriminator objects)
    if is_tagged_union(schema) {
        return parse_tagged_union(name, schema);
    }

    // Pattern 9: Map (object with additionalProperties, no properties/required)
    if is_map(schema) {
        // Maps at the top level are unusual; treat as a struct with a single
        // unnamed field. In practice none of the manifest schemas are bare maps
        // at root level, but definitions can be.
        panic!("Top-level map schemas are not supported: {name}");
    }

    // Pattern 4: Plain struct (object with properties)
    parse_struct(name, schema)
}

/// Parse a schema that appears in a `definitions` block. Same logic as
/// `parse_schema` but takes ownership of the name from the definitions key.
pub fn parse_definition(name: &str, schema: &Value) -> TypeDef {
    parse_schema(name, schema)
}

// ---------------------------------------------------------------------------
// Detection helpers
// ---------------------------------------------------------------------------

fn is_simple_string_enum(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("string") && schema.get("enum").is_some()
}

fn is_documented_string_enum(schema: &Value) -> bool {
    let one_of = match schema.get("oneOf").and_then(Value::as_array) {
        Some(arr) => arr,
        None => return false,
    };
    // Every element must be {type: "string", enum: ["single_value"]}
    one_of.iter().all(|v| {
        v.get("type").and_then(Value::as_str) == Some("string")
            && v.get("enum")
                .and_then(Value::as_array)
                .is_some_and(|e| e.len() == 1)
    })
}

fn is_tagged_union(schema: &Value) -> bool {
    let one_of = match schema.get("oneOf").and_then(Value::as_array) {
        Some(arr) => arr,
        None => return false,
    };
    // Each element is an object with a property that has enum: ["single_value"]
    one_of.iter().all(|v| {
        v.get("type").and_then(Value::as_str) == Some("object") && find_discriminator(v).is_some()
    })
}

fn is_map(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("object")
        && schema.get("additionalProperties").is_some()
        && schema.get("properties").is_none()
        && schema.get("required").is_none()
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_simple_string_enum(name: &str, schema: &Value) -> TypeDef {
    let variants = schema["enum"]
        .as_array()
        .expect("enum must be an array")
        .iter()
        .map(|v| v.as_str().expect("enum values must be strings").to_string())
        .collect();
    TypeDef::StringEnum {
        name: name.to_string(),
        variants,
    }
}

fn parse_documented_string_enum(name: &str, schema: &Value) -> TypeDef {
    let variants = schema["oneOf"]
        .as_array()
        .expect("oneOf must be an array")
        .iter()
        .map(|v| {
            v["enum"].as_array().expect("enum must be array")[0]
                .as_str()
                .expect("enum value must be string")
                .to_string()
        })
        .collect();
    TypeDef::StringEnum {
        name: name.to_string(),
        variants,
    }
}

/// Find the discriminator property name in a tagged-union variant object.
/// Returns `Some(property_name)` if exactly one property has `enum: ["single_value"]`.
fn find_discriminator(variant_schema: &Value) -> Option<String> {
    let props = variant_schema.get("properties")?.as_object()?;
    for (key, val) in props {
        if let Some(arr) = val.get("enum").and_then(Value::as_array) {
            if arr.len() == 1 {
                return Some(key.clone());
            }
        }
    }
    None
}

fn parse_tagged_union(name: &str, schema: &Value) -> TypeDef {
    let one_of = schema["oneOf"].as_array().expect("oneOf must be array");

    // Determine the discriminator name from the first variant.
    let discriminator = find_discriminator(&one_of[0]).expect("could not find discriminator");

    let variants: Vec<UnionVariant> = one_of
        .iter()
        .map(|variant_schema| {
            let props = variant_schema["properties"]
                .as_object()
                .expect("variant must have properties");
            let required: Vec<String> = variant_schema
                .get("required")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .map(|v| v.as_str().expect("required must be string").to_string())
                        .collect()
                })
                .unwrap_or_default();

            let tag_value = props[&discriminator]["enum"]
                .as_array()
                .expect("enum array")[0]
                .as_str()
                .expect("enum value must be string")
                .to_string();

            let mut fields: Vec<FieldDef> = Vec::new();
            let mut prop_names: Vec<&String> = props.keys().collect();
            prop_names.sort();

            for key in prop_names {
                if key == &discriminator {
                    continue;
                }
                let prop_schema = &props[key];
                let is_required = required.contains(key);
                let (ty, has_default, default_value) = resolve_field_type(prop_schema, is_required);
                fields.push(FieldDef {
                    name: key.clone(),
                    ty,
                    required: is_required,
                    has_default,
                    default_value,
                });
            }

            UnionVariant { tag_value, fields }
        })
        .collect();

    TypeDef::TaggedUnion {
        name: name.to_string(),
        discriminator,
        variants,
    }
}

fn parse_struct(name: &str, schema: &Value) -> TypeDef {
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required: Vec<String> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().expect("required must be string").to_string())
                .collect()
        })
        .unwrap_or_default();

    let mut sorted_keys: Vec<&String> = properties.keys().collect();
    sorted_keys.sort();

    let fields: Vec<FieldDef> = sorted_keys
        .iter()
        .map(|key| {
            let prop_schema = &properties[*key];
            let is_required = required.contains(key);
            let (ty, has_default, default_value) = resolve_field_type(prop_schema, is_required);
            FieldDef {
                name: (*key).clone(),
                ty,
                required: is_required,
                has_default,
                default_value,
            }
        })
        .collect();

    TypeDef::Struct {
        name: name.to_string(),
        fields,
    }
}

/// Resolve a property schema to a `FieldType`, accounting for `$ref`, `allOf`,
/// `anyOf`, nullable types, arrays, maps, and primitives.
///
/// Returns `(field_type, has_default, default_value)`.
fn resolve_field_type(schema: &Value, is_required: bool) -> (FieldType, bool, Option<Value>) {
    let has_default = schema.get("default").is_some();
    let default_value = schema.get("default").cloned();

    // Pattern 7: allOf with $ref (default ref)
    if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
        if let Some(first) = all_of.first() {
            if let Some(ref_path) = first.get("$ref").and_then(Value::as_str) {
                let ref_name = ref_name_from_path(ref_path);
                return (FieldType::Ref(ref_name), has_default, default_value);
            }
        }
    }

    // Pattern 6: anyOf with $ref and null (nullable ref)
    if let Some(any_of) = schema.get("anyOf").and_then(Value::as_array) {
        if any_of.len() == 2 {
            let has_null = any_of
                .iter()
                .any(|v| v.get("type").and_then(Value::as_str) == Some("null"));
            let ref_entry = any_of.iter().find(|v| v.get("$ref").is_some());
            if has_null {
                if let Some(ref_entry) = ref_entry {
                    let ref_name =
                        ref_name_from_path(ref_entry["$ref"].as_str().expect("$ref is string"));
                    return (
                        FieldType::Nullable(Box::new(FieldType::Ref(ref_name))),
                        has_default,
                        default_value,
                    );
                }
            }
        }
    }

    // Pattern 10: Direct $ref
    if let Some(ref_path) = schema.get("$ref").and_then(Value::as_str) {
        let ref_name = ref_name_from_path(ref_path);
        return (FieldType::Ref(ref_name), has_default, default_value);
    }

    // Handle type field (can be string or array for nullable primitives)
    let ty = schema.get("type");

    // Pattern 5: Nullable primitive — type is an array like ["string", "null"]
    if let Some(type_arr) = ty.and_then(Value::as_array) {
        let types: Vec<&str> = type_arr.iter().filter_map(Value::as_str).collect();
        let has_null = types.contains(&"null");
        let non_null: Vec<&&str> = types.iter().filter(|t| **t != "null").collect();
        if has_null && non_null.len() == 1 {
            let inner = resolve_primitive_type(non_null[0], schema);
            return (
                FieldType::Nullable(Box::new(inner)),
                has_default,
                default_value,
            );
        }
        // For nullable array type like ["array", "null"]
        if has_null && non_null.len() == 1 && **non_null[0] == *"array" {
            let items_type = resolve_items_type(schema);
            return (
                FieldType::Nullable(Box::new(FieldType::Array(Box::new(items_type)))),
                has_default,
                default_value,
            );
        }
    }

    if let Some(type_str) = ty.and_then(Value::as_str) {
        // Pattern 9: Map — object with additionalProperties, no properties
        if type_str == "object"
            && schema.get("additionalProperties").is_some()
            && schema.get("properties").is_none()
        {
            let value_type = resolve_additional_properties_type(schema);
            return (
                FieldType::Map(Box::new(value_type)),
                has_default,
                default_value,
            );
        }

        // Array
        if type_str == "array" {
            let items_type = resolve_items_type(schema);
            return (
                FieldType::Array(Box::new(items_type)),
                has_default,
                default_value,
            );
        }

        // Primitive
        let ft = resolve_primitive_type(type_str, schema);
        // If not required and no default and not already nullable, wrap in nullable
        if !is_required && !has_default {
            return (
                FieldType::Nullable(Box::new(ft)),
                has_default,
                default_value,
            );
        }
        return (ft, has_default, default_value);
    }

    // Fallback — should not reach here for well-formed schemas
    panic!(
        "Could not resolve field type from schema: {}",
        serde_json::to_string_pretty(schema).unwrap_or_default()
    );
}

fn resolve_primitive_type(type_str: &str, schema: &Value) -> FieldType {
    match type_str {
        "string" => FieldType::String,
        "boolean" => FieldType::Bool,
        "integer" => FieldType::Int,
        "number" => FieldType::Float,
        "object" => {
            // Check if it's a map via additionalProperties
            if schema.get("additionalProperties").is_some() {
                let value_type = resolve_additional_properties_type(schema);
                FieldType::Map(Box::new(value_type))
            } else {
                // Bare object without schema — use map of any (string fallback)
                FieldType::Map(Box::new(FieldType::String))
            }
        }
        "array" => {
            let items_type = resolve_items_type(schema);
            FieldType::Array(Box::new(items_type))
        }
        other => panic!("Unknown primitive type: {other}"),
    }
}

fn resolve_items_type(schema: &Value) -> FieldType {
    match schema.get("items") {
        Some(items) => {
            if let Some(ref_path) = items.get("$ref").and_then(Value::as_str) {
                FieldType::Ref(ref_name_from_path(ref_path))
            } else {
                let item_type = items
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("string");
                resolve_primitive_type(item_type, items)
            }
        }
        None => FieldType::String,
    }
}

fn resolve_additional_properties_type(schema: &Value) -> FieldType {
    match schema.get("additionalProperties") {
        Some(ap) => {
            if let Some(type_str) = ap.get("type").and_then(Value::as_str) {
                resolve_primitive_type(type_str, ap)
            } else {
                FieldType::String
            }
        }
        None => FieldType::String,
    }
}

fn ref_name_from_path(ref_path: &str) -> String {
    // "#/definitions/FooBar" -> "FooBar"
    ref_path
        .rsplit('/')
        .next()
        .expect("$ref must have a path segment")
        .to_string()
}

/// Extract definitions from a schema's `definitions` block.
pub fn extract_definitions(schema: &Value) -> BTreeMap<String, Value> {
    schema
        .get("definitions")
        .and_then(Value::as_object)
        .map(|defs| defs.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}
