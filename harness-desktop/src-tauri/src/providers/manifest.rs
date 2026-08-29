use crate::domain::Capability;
use crate::error::{HarnessError, HarnessResult};
use regex::Regex;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub provider_type: ManifestProviderType,
    pub adapter_version: semver::Version,
    #[serde(default)]
    pub binary_candidates: Vec<String>,
    pub supported_os: BTreeSet<String>,
    #[serde(default)]
    pub capabilities: BTreeSet<Capability>,
    pub auth_strategy: AuthStrategy,
    pub install_strategy: InstallStrategy,
    pub update_strategy: UpdateStrategy,
    pub usage_strategy: UsageStrategy,
    #[serde(default)]
    pub models: Vec<ManifestModel>,
    pub minimum_version: Option<VersionReq>,
    pub tested_through: Option<semver::Version>,
    #[serde(default)]
    pub metadata: serde_json::Map<String, Value>,
}

impl ProviderManifest {
    pub fn parse_and_validate(json: &str) -> HarnessResult<Self> {
        if json.len() > 1024 * 1024 {
            return Err(HarnessError::Validation(
                "provider manifest exceeds 1 MiB".into(),
            ));
        }
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> HarnessResult<()> {
        let id_pattern =
            Regex::new(r"^[a-z0-9][a-z0-9._-]{1,99}$").expect("static provider id regex");
        if !id_pattern.is_match(&self.id) {
            return Err(HarnessError::Validation(
                "provider id must be 2-100 lowercase safe characters".into(),
            ));
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 200 {
            return Err(HarnessError::Validation("invalid provider name".into()));
        }
        if self.schema_version != 1 {
            return Err(HarnessError::Unavailable(format!(
                "unsupported provider protocol version {}",
                self.schema_version
            )));
        }
        if matches!(
            self.provider_type,
            ManifestProviderType::CliProvider | ManifestProviderType::CustomProvider
        ) && self.binary_candidates.is_empty()
        {
            return Err(HarnessError::Validation(
                "CLI/custom providers require a binary candidate".into(),
            ));
        }
        for candidate in &self.binary_candidates {
            let path = Path::new(candidate);
            if candidate.is_empty()
                || candidate.len() > 200
                || path.components().count() != 1
                || candidate.contains(['/', '\\'])
                || candidate.starts_with('-')
                || !candidate.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "._-+@".contains(character)
                })
            {
                return Err(HarnessError::Validation(format!(
                    "binary candidate {candidate:?} must be a basename, not a path or command"
                )));
            }
        }
        let supported = ["macos", "windows", "linux"];
        if self.supported_os.is_empty()
            || self
                .supported_os
                .iter()
                .any(|os| !supported.contains(&os.as_str()))
        {
            return Err(HarnessError::Validation(
                "supported_os contains an unknown platform".into(),
            ));
        }
        if self.models.iter().any(|model| model.id.trim().is_empty()) {
            return Err(HarnessError::Validation(
                "manifest model ids cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ManifestProviderType {
    #[serde(rename = "CLI_PROVIDER")]
    CliProvider,
    #[serde(rename = "HTTP_PROVIDER")]
    HttpProvider,
    #[serde(rename = "LOCAL_RUNTIME_PROVIDER")]
    LocalRuntimeProvider,
    #[serde(rename = "CUSTOM_PROVIDER")]
    CustomProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestModel {
    pub id: String,
    pub display_name: Option<String>,
    pub discovery: ModelDiscovery,
    #[serde(default)]
    pub capabilities: BTreeSet<Capability>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelDiscovery {
    #[serde(rename = "STATIC")]
    Static,
    #[serde(rename = "DYNAMIC")]
    Dynamic,
    #[serde(rename = "USER_CONFIGURED")]
    UserConfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthStrategy {
    #[serde(rename = "INHERIT_CLI_SESSION")]
    InheritCliSession,
    #[serde(rename = "OAUTH_PKCE_OS_STORE")]
    OauthPkceOsStore,
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "CUSTOM")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallStrategy {
    #[serde(rename = "APP_MANAGED")]
    AppManaged,
    #[serde(rename = "VENDOR_NATIVE")]
    VendorNative,
    #[serde(rename = "HOMEBREW")]
    Homebrew,
    #[serde(rename = "WINGET")]
    Winget,
    #[serde(rename = "EXTERNAL")]
    External,
    #[serde(rename = "CUSTOM")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStrategy {
    #[serde(rename = "SIGNED_APP_MANAGED")]
    SignedAppManaged,
    #[serde(rename = "VENDOR_OR_PACKAGE_MANAGER")]
    VendorOrPackageManager,
    #[serde(rename = "NOTIFY_ONLY")]
    NotifyOnly,
    #[serde(rename = "CUSTOM")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageStrategy {
    #[serde(rename = "EVENT_STREAM")]
    EventStream,
    #[serde(rename = "REQUEST_COUNT")]
    RequestCount,
    #[serde(rename = "LOCAL_RUNTIME_METRICS")]
    LocalRuntimeMetrics,
    #[serde(rename = "UNAVAILABLE")]
    Unavailable,
    #[serde(rename = "CUSTOM")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl JsonRpcRequest {
    pub fn new(id: impl Into<String>, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }

    pub fn validate(&self) -> HarnessResult<()> {
        if self.jsonrpc != "2.0" || self.id.is_empty() {
            return Err(HarnessError::Validation("invalid JSON-RPC envelope".into()));
        }
        let allowed = [
            "provider.initialize",
            "provider.getStatus",
            "provider.getCapabilities",
            "provider.getModels",
            "provider.execute",
            "provider.cancel",
            "provider.resume",
            "provider.getUsage",
            "provider.shutdown",
        ];
        if !allowed.contains(&self.method.as_str()) {
            return Err(HarnessError::PermissionDenied(format!(
                "JSON-RPC method {} is not part of protocol v1",
                self.method
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> String {
        serde_json::json!({
            "schema_version": 1,
            "id": "example-cli",
            "name": "Example CLI",
            "provider_type": "CLI_PROVIDER",
            "adapter_version": "1.2.0",
            "binary_candidates": ["example"],
            "supported_os": ["macos", "windows"],
            "capabilities": ["coding"],
            "auth_strategy": "INHERIT_CLI_SESSION",
            "install_strategy": "VENDOR_NATIVE",
            "update_strategy": "NOTIFY_ONLY",
            "usage_strategy": "EVENT_STREAM",
            "models": [],
            "minimum_version": ">=1.0",
            "tested_through": "2.0.0",
            "metadata": {}
        })
        .to_string()
    }

    #[test]
    fn validates_safe_manifest() {
        let manifest = ProviderManifest::parse_and_validate(&valid_manifest()).unwrap();
        assert_eq!(manifest.id, "example-cli");
    }

    #[test]
    fn rejects_shell_like_binary_candidate() {
        let mut value: Value = serde_json::from_str(&valid_manifest()).unwrap();
        value["binary_candidates"] = serde_json::json!(["sh -c evil"]);
        assert!(ProviderManifest::parse_and_validate(&value.to_string()).is_err());
    }

    #[test]
    fn only_protocol_methods_are_allowed() {
        let request = JsonRpcRequest::new("1", "os.shell", serde_json::json!({}));
        assert!(matches!(
            request.validate(),
            Err(HarnessError::PermissionDenied(_))
        ));
    }
}
