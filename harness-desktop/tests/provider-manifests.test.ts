import { readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

type ProviderManifest = {
  schema_version: number;
  id: string;
  provider_type: string;
  auth_strategy: string;
  capabilities: string[];
  supported_os: string[];
  metadata?: {
    credential_access?: string;
    forbidden_flags?: string[];
    catalog_must_be_signed?: boolean;
    artifacts_require_sha256?: boolean;
  };
};

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const providersDirectory = resolve(projectRoot, "providers");

function manifests(): ProviderManifest[] {
  return readdirSync(providersDirectory)
    .filter((name) => name.endsWith(".json") && !name.includes("schema"))
    .map((name) =>
      JSON.parse(readFileSync(resolve(providersDirectory, name), "utf8")),
    );
}

describe("provider manifests", () => {
  it("uses unique, stable identifiers", () => {
    const ids = manifests().map((manifest) => manifest.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.every((id) => /^[a-z][a-z0-9.-]{2,63}$/.test(id))).toBe(true);
  });

  it("never asks Harness to read inherited CLI credentials", () => {
    const inherited = manifests().filter(
      (manifest) => manifest.auth_strategy === "INHERIT_CLI_SESSION",
    );
    expect(inherited.length).toBeGreaterThan(0);
    for (const manifest of inherited) {
      expect(manifest.metadata?.credential_access).toBe("forbidden");
    }
  });

  it("blocks every Codex dangerous bypass alias", () => {
    const codex = manifests().find((manifest) => manifest.id === "openai.codex-cli");
    expect(codex?.metadata?.forbidden_flags).toEqual(
      expect.arrayContaining([
        "--dangerously-bypass-approvals-and-sandbox",
        "--yolo",
      ]),
    );
  });

  it("requires integrity metadata for managed local artifacts", () => {
    const managed = manifests().find(
      (manifest) => manifest.id === "local.managed-llamacpp",
    );
    expect(managed?.metadata?.catalog_must_be_signed).toBe(true);
    expect(managed?.metadata?.artifacts_require_sha256).toBe(true);
  });
});
