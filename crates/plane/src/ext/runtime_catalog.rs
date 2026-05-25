use std::collections::HashMap;

use super::error::ApiError;

/// Logical runtime presets → local repository names (tag form used by dev fleet).
const RUNTIME_REPOS: &[(&str, &str)] = &[
    ("node22", "sandbox-runtime:node22-latest"),
    ("node24", "sandbox-runtime:node24-latest"),
    ("python313", "sandbox-runtime:python313-latest"),
    ("bun1", "sandbox-runtime:bun1-latest"),
    ("agent", "sandbox-runtime:agent-latest"),
    ("desktop", "sandbox-runtime:desktop-latest"),
];

pub(crate) fn qualify_image(repository: &str, registry: Option<&str>) -> String {
    match registry.filter(|r| !r.is_empty()) {
        Some(prefix) => format!("{}/{}", prefix.trim_end_matches('/'), repository),
        None => repository.to_string(),
    }
}

pub(crate) fn resolve_runtime_image(
    runtime: &str,
    explicit_image: Option<&str>,
    registry: Option<&str>,
) -> Result<String, ApiError> {
    if runtime == "custom" {
        return explicit_image.map(str::to_string).ok_or_else(|| {
            ApiError::invalid_request("image is required when runtime is 'custom'")
        });
    }

    if let Some((_, repository)) = RUNTIME_REPOS.iter().find(|(name, _)| *name == runtime) {
        return Ok(qualify_image(repository, registry));
    }

    if let Some(image) = explicit_image {
        return Ok(image.to_string());
    }

    Err(ApiError::invalid_request(format!(
        "unknown runtime '{runtime}'; use a supported runtime or runtime \"custom\" with an explicit image"
    )))
}

/// Runtime-specific env defaults merged before spawn. Explicit request env wins on conflict.
pub(crate) fn merge_runtime_env(
    runtime: &str,
    explicit: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut env = HashMap::new();
    if runtime == "desktop" {
        env.insert("VNC_ENABLED".to_string(), "1".to_string());
        // Spec §13: read-only VNC by default; callers opt into write with VNC_VIEWONLY=0.
        env.insert("VNC_VIEWONLY".to_string(), "1".to_string());
    }
    env.extend(explicit.clone());
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualify_with_registry_prefix() {
        assert_eq!(
            qualify_image("sandbox-runtime:node24-latest", Some("192.168.56.1:5000")),
            "192.168.56.1:5000/sandbox-runtime:node24-latest"
        );
    }

    #[test]
    fn qualify_without_registry() {
        assert_eq!(
            qualify_image("sandbox-runtime:node24-latest", None),
            "sandbox-runtime:node24-latest"
        );
    }

    #[test]
    fn desktop_runtime_env_defaults_enable_vnc() {
        let env = merge_runtime_env("desktop", &HashMap::new());
        assert_eq!(env.get("VNC_ENABLED"), Some(&"1".to_string()));
        assert_eq!(env.get("VNC_VIEWONLY"), Some(&"1".to_string()));
    }

    #[test]
    fn desktop_runtime_env_explicit_overrides_defaults() {
        let mut explicit = HashMap::new();
        explicit.insert("VNC_VIEWONLY".to_string(), "0".to_string());
        explicit.insert("VNC_ENABLED".to_string(), "0".to_string());
        let env = merge_runtime_env("desktop", &explicit);
        assert_eq!(env.get("VNC_ENABLED"), Some(&"0".to_string()));
        assert_eq!(env.get("VNC_VIEWONLY"), Some(&"0".to_string()));
    }

    #[test]
    fn non_desktop_runtime_has_no_env_defaults() {
        let env = merge_runtime_env("node24", &HashMap::new());
        assert!(env.is_empty());
    }
}
