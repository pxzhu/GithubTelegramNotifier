use crate::domain::*;
use crate::error::{HarnessError, HarnessResult};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Default)]
pub struct AdaptiveSignals {
    /// Bounded quality adjustments learned from explicit success outcomes, never prompt content.
    pub provider_adjustments: HashMap<String, f32>,
}

pub struct DeterministicRouter;

impl DeterministicRouter {
    pub fn route(
        request: &RouteRequest,
        providers: &[ProviderSnapshot],
        signals: Option<&AdaptiveSignals>,
    ) -> HarnessResult<RouteDecision> {
        let mut required = request.required_capabilities.clone();
        if request.requires_company_context {
            required.insert(Capability::CompanySearch);
        }
        if request.modifies_repository {
            required.insert(Capability::Coding);
            required.insert(Capability::FileAccess);
        }
        if request.offline_requested {
            required.insert(Capability::Offline);
            required.insert(Capability::Local);
        }

        if let Some(explicit) = request.explicit_provider.as_deref() {
            let provider = providers
                .iter()
                .find(|provider| provider.id == explicit)
                .ok_or_else(|| HarnessError::NotFound(format!("provider {explicit}")))?;
            ensure_eligible(provider, &required)?;
            return Ok(RouteDecision {
                provider_id: Some(provider.id.clone()),
                model_id: choose_model(provider, &required),
                confidence: 1.0,
                planner_required: false,
                reasons: vec!["The user explicitly selected this provider".into()],
                rejected: vec![],
            });
        }

        let mut rejected = vec![];
        let mut scored = vec![];
        for provider in providers {
            if let Err(reason) = eligibility_reason(provider, &required) {
                rejected.push(ProviderRejection {
                    provider_id: provider.id.clone(),
                    reason,
                });
                continue;
            }
            let mut score = 50.0_f32;
            let mut reasons = Vec::new();
            if request.requires_company_context && provider.id == "microsoft.m365-copilot" {
                score += 100.0;
                reasons.push("Company context maps deterministically to M365".into());
            }
            if request.modifies_repository && provider.capabilities.contains(&Capability::Coding) {
                score += 25.0;
                reasons.push("Provider supports repository coding work".into());
            }
            match request.mode {
                RoutingMode::QualityFirst => {
                    if provider.id == "anthropic.claude-code" {
                        score += 25.0;
                    }
                    if provider.capabilities.contains(&Capability::Reasoning) {
                        score += 15.0;
                    }
                }
                RoutingMode::Balanced => {
                    if provider.capabilities.contains(&Capability::Fast) {
                        score += 8.0;
                    }
                    if provider.capabilities.contains(&Capability::Cheap) {
                        score += 8.0;
                    }
                }
                RoutingMode::ClaudeSaver => {
                    if provider.id == "anthropic.claude-code" {
                        score -= 45.0;
                    } else {
                        score += 15.0;
                    }
                    if provider.capabilities.contains(&Capability::Local) {
                        score += 30.0;
                    }
                }
                RoutingMode::LocalFirst => {
                    if provider.capabilities.contains(&Capability::Local) {
                        score += 70.0;
                        reasons
                            .push("Local-first policy prefers an eligible local provider".into());
                    } else {
                        score -= 20.0;
                    }
                }
                RoutingMode::Custom => {}
            }
            if matches!(request.complexity, Complexity::High)
                && provider.capabilities.contains(&Capability::Reasoning)
            {
                score += 15.0;
            }
            if let Some(adjustment) =
                signals.and_then(|signals| signals.provider_adjustments.get(&provider.id))
            {
                score += adjustment.clamp(-15.0, 15.0);
                reasons.push("Bounded historical outcome signal applied".into());
            }
            scored.push((provider, score, reasons));
        }

        scored.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.id.cmp(&right.0.id))
        });
        let Some((selected, selected_score, mut reasons)) = scored.first().cloned() else {
            return Ok(RouteDecision {
                provider_id: None,
                model_id: None,
                confidence: 0.0,
                planner_required: false,
                reasons: vec![
                    "No enabled, connected provider satisfies every required capability".into(),
                ],
                rejected,
            });
        };
        let gap = scored
            .get(1)
            .map(|second| (selected_score - second.1).max(0.0))
            .unwrap_or(50.0);
        let confidence = (0.55 + gap / 100.0).clamp(0.55, 0.95);
        if reasons.is_empty() {
            reasons.push("Selected by deterministic capability and policy score".into());
        }
        let planner_required = matches!(request.complexity, Complexity::Unknown)
            || (matches!(request.complexity, Complexity::High) && confidence < 0.75);
        Ok(RouteDecision {
            provider_id: Some(selected.id.clone()),
            model_id: choose_model(selected, &required),
            confidence,
            planner_required,
            reasons,
            rejected,
        })
    }
}

fn ensure_eligible(
    provider: &ProviderSnapshot,
    required: &BTreeSet<Capability>,
) -> HarnessResult<()> {
    eligibility_reason(provider, required)
        .map_err(|reason| HarnessError::Unavailable(format!("provider {}: {reason}", provider.id)))
}

fn eligibility_reason(
    provider: &ProviderSnapshot,
    required: &BTreeSet<Capability>,
) -> Result<(), String> {
    if !provider.enabled {
        return Err("provider is disabled".into());
    }
    if !provider.status.can_execute() {
        return Err(format!("provider status is {:?}", provider.status));
    }
    let missing: Vec<String> = required
        .difference(&provider.capabilities)
        .map(|capability| format!("{capability:?}"))
        .collect();
    if !missing.is_empty() {
        return Err(format!("missing capabilities: {}", missing.join(", ")));
    }
    Ok(())
}

fn choose_model(provider: &ProviderSnapshot, required: &BTreeSet<Capability>) -> Option<String> {
    provider
        .models
        .iter()
        .filter(|model| required.is_subset(&model.capabilities))
        .min_by(|left, right| left.id.cmp(&right.id))
        .map(|model| model.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn provider(id: &str, capabilities: &[Capability]) -> ProviderSnapshot {
        ProviderSnapshot {
            id: id.into(),
            name: id.into(),
            kind: ProviderKind::Cli,
            status: ProviderStatus::Connected,
            enabled: true,
            version: None,
            capabilities: capabilities.iter().copied().collect(),
            models: vec![],
            status_detail: None,
            executable_path: None,
            detected_at: Utc::now(),
        }
    }

    fn request(mode: RoutingMode) -> RouteRequest {
        RouteRequest {
            explicit_provider: None,
            required_capabilities: BTreeSet::new(),
            requires_company_context: false,
            modifies_repository: false,
            offline_requested: false,
            complexity: Complexity::Low,
            mode,
        }
    }

    #[test]
    fn company_context_routes_only_to_company_search() {
        let providers = vec![
            provider("anthropic.claude-code", &[Capability::Reasoning]),
            provider("microsoft.m365-copilot", &[Capability::CompanySearch]),
        ];
        let mut request = request(RoutingMode::Balanced);
        request.requires_company_context = true;
        let decision = DeterministicRouter::route(&request, &providers, None).unwrap();
        assert_eq!(
            decision.provider_id.as_deref(),
            Some("microsoft.m365-copilot")
        );
    }

    #[test]
    fn claude_saver_avoids_claude_when_capabilities_match() {
        let providers = vec![
            provider(
                "anthropic.claude-code",
                &[Capability::Coding, Capability::FileAccess],
            ),
            provider(
                "openai.codex-cli",
                &[Capability::Coding, Capability::FileAccess],
            ),
        ];
        let mut request = request(RoutingMode::ClaudeSaver);
        request.modifies_repository = true;
        let decision = DeterministicRouter::route(&request, &providers, None).unwrap();
        assert_eq!(decision.provider_id.as_deref(), Some("openai.codex-cli"));
    }

    #[test]
    fn offline_rejects_cloud_provider() {
        let providers = vec![provider("anthropic.claude-code", &[Capability::Reasoning])];
        let mut request = request(RoutingMode::LocalFirst);
        request.offline_requested = true;
        let decision = DeterministicRouter::route(&request, &providers, None).unwrap();
        assert_eq!(decision.provider_id, None);
        assert_eq!(decision.rejected.len(), 1);
    }
}
