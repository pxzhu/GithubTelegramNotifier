use harness_core::providers::ProviderManifest;
use std::fs;
use std::path::{Path, PathBuf};

fn parse_manifest(path: &Path) {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    ProviderManifest::parse_and_validate(&json)
        .unwrap_or_else(|error| panic!("invalid manifest {}: {error}", path.display()));
}

#[test]
fn repository_provider_manifests_match_the_rust_contract() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let providers = project_root.join("providers");
    for entry in fs::read_dir(&providers).expect("provider manifest directory") {
        let path = entry.expect("provider manifest entry").path();
        let is_schema = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains("schema"));
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") && !is_schema {
            parse_manifest(&path);
        }
    }

    parse_manifest(
        &project_root
            .join("examples")
            .join("generic-provider")
            .join("provider.manifest.json"),
    );
}
