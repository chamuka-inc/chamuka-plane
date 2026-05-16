use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkPolicyMode {
    AllowAll,
    DenyAll,
    Rules,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetworkRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cidrs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<RuleAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetworkPolicy {
    pub mode: NetworkPolicyMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<NetworkRule>>,
}
