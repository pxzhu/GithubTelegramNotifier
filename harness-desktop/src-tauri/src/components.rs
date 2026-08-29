use crate::error::{HarnessError, HarnessResult};
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    App,
    ProviderAdapter,
    CliAgent,
    LocalRuntime,
    LocalModel,
    Plugin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallSource {
    AppManaged,
    VendorNative,
    Homebrew,
    Winget,
    SystemPackage,
    Manual,
    External,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePolicy {
    Automatic,
    NotifyOnly,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCandidate {
    pub component_id: String,
    pub component_type: ComponentType,
    pub installed_version: Option<String>,
    pub candidate_version: String,
    pub install_source: InstallSource,
    pub policy: UpdatePolicy,
    pub enterprise_managed: bool,
    pub requires_elevation: bool,
    pub requires_license_acceptance: bool,
    pub sha256: Option<String>,
    pub signature_verified: bool,
    pub release_trust_configured: bool,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateAction {
    NoAction,
    AutomaticEligible,
    DownloadEligible,
    NotifyOnly,
    UserApprovalRequired,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlan {
    pub component_id: String,
    pub action: UpdateAction,
    pub reasons: Vec<String>,
    pub rollback_available: bool,
}

pub fn plan_update(candidate: &UpdateCandidate) -> HarnessResult<UpdatePlan> {
    let candidate_version = Version::parse(&candidate.candidate_version)
        .map_err(|error| HarnessError::Validation(format!("invalid candidate version: {error}")))?;
    if let Some(installed) = candidate.installed_version.as_deref() {
        let installed = Version::parse(installed).map_err(|error| {
            HarnessError::Validation(format!("invalid installed version: {error}"))
        })?;
        if candidate_version <= installed {
            return Ok(UpdatePlan {
                component_id: candidate.component_id.clone(),
                action: UpdateAction::NoAction,
                reasons: vec!["Installed version is current or newer".into()],
                rollback_available: candidate.rollback_available,
            });
        }
    }
    if candidate.enterprise_managed {
        return Ok(UpdatePlan {
            component_id: candidate.component_id.clone(),
            action: UpdateAction::Blocked,
            reasons: vec!["Enterprise-managed components cannot be changed by Harness".into()],
            rollback_available: candidate.rollback_available,
        });
    }
    if candidate.requires_license_acceptance || candidate.requires_elevation {
        return Ok(UpdatePlan {
            component_id: candidate.component_id.clone(),
            action: UpdateAction::UserApprovalRequired,
            reasons: vec!["Elevation or explicit license acceptance requires user control".into()],
            rollback_available: candidate.rollback_available,
        });
    }
    if candidate.install_source != InstallSource::AppManaged {
        return Ok(UpdatePlan {
            component_id: candidate.component_id.clone(),
            action: UpdateAction::NotifyOnly,
            reasons: vec![
                "The detected vendor or package-manager install source retains ownership".into(),
            ],
            rollback_available: candidate.rollback_available,
        });
    }
    let hash_is_invalid = match candidate.sha256.as_deref() {
        Some(hash) => !valid_sha256(hash),
        None => true,
    };
    if hash_is_invalid || !candidate.signature_verified || !candidate.release_trust_configured {
        return Ok(UpdatePlan {
            component_id: candidate.component_id.clone(),
            action: UpdateAction::Blocked,
            reasons: vec!["App-managed updates require configured release trust, signature verification, and SHA-256".into()],
            rollback_available: candidate.rollback_available,
        });
    }
    let action = match candidate.policy {
        UpdatePolicy::Automatic => UpdateAction::AutomaticEligible,
        UpdatePolicy::NotifyOnly => UpdateAction::DownloadEligible,
        UpdatePolicy::Manual => UpdateAction::NotifyOnly,
    };
    Ok(UpdatePlan {
        component_id: candidate.component_id.clone(),
        action,
        reasons: vec!["Candidate passed the non-mutating update safety policy".into()],
        rollback_available: candidate.rollback_available,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityRule {
    pub adapter_version: Version,
    pub minimum_cli: Version,
    pub tested_through_cli: Version,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    Compatible,
    UpdateRequired,
    UntestedNewer,
}

pub fn check_compatibility(rule: &CompatibilityRule, cli: &Version) -> CompatibilityStatus {
    if cli < &rule.minimum_cli {
        CompatibilityStatus::UpdateRequired
    } else if cli > &rule.tested_through_cli {
        CompatibilityStatus::UntestedNewer
    } else {
        CompatibilityStatus::Compatible
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> UpdateCandidate {
        UpdateCandidate {
            component_id: "runtime".into(),
            component_type: ComponentType::LocalRuntime,
            installed_version: Some("1.0.0".into()),
            candidate_version: "1.1.0".into(),
            install_source: InstallSource::AppManaged,
            policy: UpdatePolicy::Automatic,
            enterprise_managed: false,
            requires_elevation: false,
            requires_license_acceptance: false,
            sha256: Some("a".repeat(64)),
            signature_verified: true,
            release_trust_configured: true,
            rollback_available: true,
        }
    }

    #[test]
    fn verified_app_managed_update_can_be_automatic() {
        let plan = plan_update(&candidate()).unwrap();
        assert_eq!(plan.action, UpdateAction::AutomaticEligible);
    }

    #[test]
    fn package_manager_update_is_notification_only() {
        let mut value = candidate();
        value.install_source = InstallSource::Homebrew;
        let plan = plan_update(&value).unwrap();
        assert_eq!(plan.action, UpdateAction::NotifyOnly);
    }

    #[test]
    fn missing_release_trust_blocks_app_update() {
        let mut value = candidate();
        value.release_trust_configured = false;
        assert_eq!(plan_update(&value).unwrap().action, UpdateAction::Blocked);
    }
}
