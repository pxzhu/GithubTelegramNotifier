use crate::domain::{Capability, ProviderKind, ProviderSnapshot, ProviderStatus};
use crate::error::{HarnessError, HarnessResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const M365_REQUIRED_DELEGATED_PERMISSIONS: &[&str] = &[
    "Sites.Read.All",
    "Mail.Read",
    "People.Read.All",
    "OnlineMeetingTranscript.Read.All",
    "Chat.Read",
    "ChannelMessage.Read.All",
    "ExternalItem.Read.All",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M365Configuration {
    pub tenant_id: String,
    pub public_client_id: String,
    #[serde(default = "default_graph_endpoint")]
    pub graph_endpoint: String,
    #[serde(default)]
    pub granted_permissions: BTreeSet<String>,
    #[serde(default)]
    pub copilot_license_confirmed: bool,
}

impl M365Configuration {
    pub fn validate(&self) -> HarnessResult<()> {
        validate_identifier("tenant id", &self.tenant_id)?;
        validate_identifier("public client id", &self.public_client_id)?;
        if self.graph_endpoint != "https://graph.microsoft.com/beta" {
            return Err(HarnessError::Validation(
                "M365 Copilot preview endpoint must be the Microsoft Graph beta endpoint".into(),
            ));
        }
        Ok(())
    }

    pub fn missing_permissions(&self) -> Vec<String> {
        M365_REQUIRED_DELEGATED_PERMISSIONS
            .iter()
            .filter(|permission| !self.granted_permissions.contains(**permission))
            .map(|permission| (*permission).to_owned())
            .collect()
    }
}

#[derive(Default)]
pub struct M365ProviderBoundary {
    configuration: Option<M365Configuration>,
}

impl M365ProviderBoundary {
    pub fn configured(configuration: M365Configuration) -> HarnessResult<Self> {
        configuration.validate()?;
        Ok(Self {
            configuration: Some(configuration),
        })
    }

    pub fn snapshot(&self) -> ProviderSnapshot {
        let capabilities = [
            Capability::CompanySearch,
            Capability::Reasoning,
            Capability::LongContext,
            Capability::TextGeneration,
        ]
        .into_iter()
        .collect();
        let (status, detail) = match &self.configuration {
            None => (
                ProviderStatus::Unavailable,
                "Microsoft Graph Copilot Chat API (preview) is not configured. PKCE authentication and OS secure storage must be supplied by the desktop integration.",
            ),
            Some(configuration) if !configuration.copilot_license_confirmed => (
                ProviderStatus::PermissionUnavailable,
                "A Microsoft 365 Copilot license has not been confirmed.",
            ),
            Some(configuration) if !configuration.missing_permissions().is_empty() => (
                ProviderStatus::PermissionUnavailable,
                "Required delegated Microsoft Graph permissions have not been granted.",
            ),
            Some(_) => (
                ProviderStatus::LoginRequired,
                "Configuration is valid; interactive Authorization Code + PKCE login is required.",
            ),
        };
        ProviderSnapshot {
            id: "microsoft.m365-copilot".into(),
            name: "Microsoft 365 Copilot".into(),
            kind: ProviderKind::Http,
            status,
            enabled: true,
            version: Some("graph-beta-preview".into()),
            capabilities,
            models: vec![],
            status_detail: Some(detail.into()),
            executable_path: None,
            detected_at: Utc::now(),
        }
    }

    pub fn create_conversation_endpoint(&self) -> HarnessResult<String> {
        let configuration = self.configuration.as_ref().ok_or_else(|| {
            HarnessError::Unavailable("M365 provider has not been configured".into())
        })?;
        configuration.validate()?;
        if !configuration.copilot_license_confirmed
            || !configuration.missing_permissions().is_empty()
        {
            return Err(HarnessError::PermissionDenied(
                "M365 license or delegated permissions are unavailable".into(),
            ));
        }
        Ok(format!(
            "{}/copilot/conversations",
            configuration.graph_endpoint
        ))
    }
}

fn default_graph_endpoint() -> String {
    "https://graph.microsoft.com/beta".into()
}

fn validate_identifier(label: &str, value: &str) -> HarnessResult<()> {
    if value.is_empty()
        || value.len() > 200
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return Err(HarnessError::Validation(format!("invalid {label}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_provider_is_explicitly_unavailable() {
        let snapshot = M365ProviderBoundary::default().snapshot();
        assert_eq!(snapshot.status, ProviderStatus::Unavailable);
        assert!(snapshot.status_detail.unwrap().contains("preview"));
    }

    #[test]
    fn permission_gaps_disable_execution() {
        let provider = M365ProviderBoundary::configured(M365Configuration {
            tenant_id: "common".into(),
            public_client_id: "client-id".into(),
            graph_endpoint: default_graph_endpoint(),
            granted_permissions: BTreeSet::new(),
            copilot_license_confirmed: true,
        })
        .unwrap();
        assert_eq!(
            provider.snapshot().status,
            ProviderStatus::PermissionUnavailable
        );
        assert!(provider.create_conversation_endpoint().is_err());
    }
}
