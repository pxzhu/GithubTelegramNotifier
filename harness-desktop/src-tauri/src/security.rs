use crate::error::{HarnessError, HarnessResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Safe,
    Modify,
    External,
    Destructive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub automatically_allow_safe: bool,
    pub automatically_allow_modify_in_workspace: bool,
    pub automatically_allow_external: bool,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            automatically_allow_safe: true,
            automatically_allow_modify_in_workspace: false,
            automatically_allow_external: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecisionKind {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub risk: RiskLevel,
    pub action: String,
    pub workspace_path: Option<String>,
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub decision: PermissionDecisionKind,
    pub reason: String,
}

pub fn evaluate_permission(
    policy: &PermissionPolicy,
    request: &PermissionRequest,
) -> PermissionDecision {
    if request.action.trim().is_empty() {
        return PermissionDecision {
            decision: PermissionDecisionKind::Deny,
            reason: "An auditable action description is required".into(),
        };
    }
    match request.risk {
        RiskLevel::Destructive => PermissionDecision {
            decision: PermissionDecisionKind::Ask,
            reason: "Destructive actions always require separate confirmation".into(),
        },
        RiskLevel::External => PermissionDecision {
            decision: if policy.automatically_allow_external {
                PermissionDecisionKind::Allow
            } else {
                PermissionDecisionKind::Ask
            },
            reason: "External side effects are governed by the project approval policy".into(),
        },
        RiskLevel::Modify => {
            let contained = match (
                request.workspace_path.as_deref(),
                request.target_path.as_deref(),
            ) {
                (Some(workspace), Some(target)) => {
                    path_is_within(workspace, target).unwrap_or(false)
                }
                _ => false,
            };
            PermissionDecision {
                decision: if policy.automatically_allow_modify_in_workspace && contained {
                    PermissionDecisionKind::Allow
                } else {
                    PermissionDecisionKind::Ask
                },
                reason: if contained {
                    "Modification target is contained by the canonical project workspace".into()
                } else {
                    "Modification target is outside or cannot be proven inside the workspace".into()
                },
            }
        }
        RiskLevel::Safe => PermissionDecision {
            decision: if policy.automatically_allow_safe {
                PermissionDecisionKind::Allow
            } else {
                PermissionDecisionKind::Ask
            },
            reason: "Read-only action evaluated by the safe-action policy".into(),
        },
    }
}

pub fn path_is_within(
    workspace: impl AsRef<Path>,
    target: impl AsRef<Path>,
) -> HarnessResult<bool> {
    let workspace = workspace.as_ref().canonicalize()?;
    let target = canonicalize_allow_missing(target.as_ref())?;
    Ok(target.starts_with(workspace))
}

fn canonicalize_allow_missing(path: &Path) -> HarnessResult<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let parent = path.parent().ok_or_else(|| {
        HarnessError::Validation("target path must have a canonical parent".into())
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| HarnessError::Validation("target path must have a file name".into()))?;
    Ok(parent.canonicalize()?.join(file_name))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretReference {
    pub service: String,
    pub account: String,
}

pub trait SecureStore: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn put(&self, reference: &SecretReference, secret: &[u8]) -> HarnessResult<()>;
    fn get(&self, reference: &SecretReference) -> HarnessResult<Vec<u8>>;
    fn delete(&self, reference: &SecretReference) -> HarnessResult<()>;
}

/// Explicit unavailable backend used until the platform Keychain/DPAPI integration is installed.
/// It never falls back to plaintext files or SQLite.
#[derive(Default)]
pub struct UnavailableSecureStore;

impl SecureStore for UnavailableSecureStore {
    fn backend_name(&self) -> &'static str {
        "unavailable"
    }

    fn put(&self, _: &SecretReference, _: &[u8]) -> HarnessResult<()> {
        Err(HarnessError::Unavailable(
            "OS secure credential storage adapter is not configured".into(),
        ))
    }

    fn get(&self, _: &SecretReference) -> HarnessResult<Vec<u8>> {
        Err(HarnessError::Unavailable(
            "OS secure credential storage adapter is not configured".into(),
        ))
    }

    fn delete(&self, _: &SecretReference) -> HarnessResult<()> {
        Err(HarnessError::Unavailable(
            "OS secure credential storage adapter is not configured".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn destructive_action_always_asks() {
        let policy = PermissionPolicy {
            automatically_allow_safe: true,
            automatically_allow_modify_in_workspace: true,
            automatically_allow_external: true,
        };
        let decision = evaluate_permission(
            &policy,
            &PermissionRequest {
                risk: RiskLevel::Destructive,
                action: "remove workspace".into(),
                workspace_path: None,
                target_path: None,
            },
        );
        assert_eq!(decision.decision, PermissionDecisionKind::Ask);
    }

    #[test]
    fn modification_must_remain_in_canonical_workspace() {
        let directory = tempdir().unwrap();
        let inside = directory.path().join("new-file.txt");
        let outside = directory.path().parent().unwrap().join("outside.txt");
        assert!(path_is_within(directory.path(), inside).unwrap());
        assert!(!path_is_within(directory.path(), outside).unwrap());
    }

    #[test]
    fn secure_store_never_falls_back_to_plaintext() {
        let store = UnavailableSecureStore;
        let reference = SecretReference {
            service: "m365".into(),
            account: "default".into(),
        };
        assert!(matches!(
            store.put(&reference, b"token"),
            Err(HarnessError::Unavailable(_))
        ));
    }
}
