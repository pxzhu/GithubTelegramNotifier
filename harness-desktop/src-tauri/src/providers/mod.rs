mod cli;
mod m365;
mod manifest;

pub use cli::{
    claude_invocation, codex_invocation, detect_builtin_cli_providers, ExecutionAccess,
    SafeCliInvocation, CLAUDE_PROVIDER_ID, CODEX_PROVIDER_ID,
};
pub use m365::{M365Configuration, M365ProviderBoundary, M365_REQUIRED_DELEGATED_PERMISSIONS};
pub use manifest::{
    AuthStrategy, InstallStrategy, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ManifestModel,
    ManifestProviderType, ModelDiscovery, ProviderManifest, UpdateStrategy, UsageStrategy,
};

use crate::domain::ProviderSnapshot;
use crate::error::HarnessResult;

pub trait AgentProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn detect(&self) -> HarnessResult<ProviderSnapshot>;
}

#[derive(Default)]
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn detect_all(&self) -> Vec<ProviderSnapshot> {
        let mut providers = detect_builtin_cli_providers();
        providers.push(M365ProviderBoundary::default().snapshot());
        providers
    }
}
