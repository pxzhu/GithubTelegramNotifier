use crate::domain::Capability;
use crate::error::{HarnessError, HarnessResult};
use crate::routing::AdaptiveSignals;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PairingStatus {
    Unpaired,
    Pending,
    Paired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerNode {
    pub id: String,
    pub display_name: String,
    pub endpoint: Option<String>,
    pub pairing_status: PairingStatus,
    pub tls_certificate_fingerprint: Option<String>,
    pub capabilities: BTreeSet<Capability>,
    pub available: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl WorkerNode {
    pub fn eligible_for(&self, required: &BTreeSet<Capability>) -> bool {
        self.available
            && self.pairing_status == PairingStatus::Paired
            && self.tls_certificate_fingerprint.is_some()
            && required.is_subset(&self.capabilities)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingOffer {
    pub offer_id: String,
    pub one_time_code: String,
    pub code_sha256: String,
    pub expires_at: DateTime<Utc>,
    pub tls_required: bool,
}

pub fn create_pairing_offer() -> PairingOffer {
    let offer_id = Uuid::new_v4().to_string();
    let one_time_code = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>()
        .to_uppercase();
    let code_sha256 = hex_sha256(one_time_code.as_bytes());
    PairingOffer {
        offer_id,
        one_time_code,
        code_sha256,
        expires_at: Utc::now() + chrono::Duration::minutes(10),
        tls_required: true,
    }
}

pub fn verify_pairing_code(offer: &PairingOffer, supplied: &str) -> HarnessResult<()> {
    if Utc::now() > offer.expires_at {
        return Err(HarnessError::PermissionDenied(
            "pairing offer has expired".into(),
        ));
    }
    let supplied = hex_sha256(supplied.trim().to_uppercase().as_bytes());
    if constant_time_eq(offer.code_sha256.as_bytes(), supplied.as_bytes()) {
        Ok(())
    } else {
        Err(HarnessError::PermissionDenied(
            "pairing code did not match".into(),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingOutcome {
    pub provider_id: String,
    pub task_type: String,
    pub succeeded: bool,
    pub retried: bool,
    pub user_accepted: Option<bool>,
    pub duration_ms: Option<u64>,
}

/// Produces conservative, bounded routing signals only after enough outcomes exist.
/// Prompt or conversation contents are never part of this model.
pub fn learn_adaptive_signals(outcomes: &[RoutingOutcome]) -> AdaptiveSignals {
    let mut by_provider: HashMap<&str, Vec<&RoutingOutcome>> = HashMap::new();
    for outcome in outcomes {
        by_provider
            .entry(outcome.provider_id.as_str())
            .or_default()
            .push(outcome);
    }
    let provider_adjustments = by_provider
        .into_iter()
        .filter_map(|(provider, values)| {
            if values.len() < 3 {
                return None;
            }
            let successes = values.iter().filter(|value| value.succeeded).count() as f32;
            let retries = values.iter().filter(|value| value.retried).count() as f32;
            let accepted = values
                .iter()
                .filter_map(|value| value.user_accepted)
                .filter(|accepted| *accepted)
                .count() as f32;
            let total = values.len() as f32;
            let score = ((successes / total - 0.5) * 20.0 - retries / total * 5.0
                + accepted / total * 2.0)
                .clamp(-15.0, 15.0);
            Some((provider.to_owned(), score))
        })
        .collect();
    AdaptiveSignals {
        provider_adjustments,
    }
}

fn hex_sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_requires_pairing_tls_and_capabilities() {
        let required: BTreeSet<Capability> = [Capability::Coding].into_iter().collect();
        let mut worker = WorkerNode {
            id: "worker".into(),
            display_name: "Worker".into(),
            endpoint: Some("https://worker.local".into()),
            pairing_status: PairingStatus::Paired,
            tls_certificate_fingerprint: None,
            capabilities: required.clone(),
            available: true,
            last_seen_at: Some(Utc::now()),
        };
        assert!(!worker.eligible_for(&required));
        worker.tls_certificate_fingerprint = Some("sha256:fingerprint".into());
        assert!(worker.eligible_for(&required));
    }

    #[test]
    fn pairing_offer_verifies_without_persisting_raw_code() {
        let offer = create_pairing_offer();
        assert!(offer.tls_required);
        assert!(verify_pairing_code(&offer, &offer.one_time_code).is_ok());
        assert!(verify_pairing_code(&offer, "WRONG000").is_err());
    }

    #[test]
    fn adaptive_signal_requires_three_samples_and_is_bounded() {
        let outcomes: Vec<RoutingOutcome> = (0..4)
            .map(|_| RoutingOutcome {
                provider_id: "provider".into(),
                task_type: "coding".into(),
                succeeded: true,
                retried: false,
                user_accepted: Some(true),
                duration_ms: Some(10),
            })
            .collect();
        let signals = learn_adaptive_signals(&outcomes);
        let score = signals.provider_adjustments["provider"];
        assert!((0.0..=15.0).contains(&score));
    }
}
