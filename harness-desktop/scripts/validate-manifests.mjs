import { readdir, readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const providersDirectory = resolve(projectRoot, "providers");
const ignoredSchemas = new Set([
  "manifest.schema.json",
  "jsonrpc-envelope.schema.json",
]);

const providerTypes = new Set([
  "CLI_PROVIDER",
  "HTTP_PROVIDER",
  "LOCAL_RUNTIME_PROVIDER",
  "CUSTOM_PROVIDER",
]);
const authStrategies = new Set([
  "INHERIT_CLI_SESSION",
  "OAUTH_PKCE_OS_STORE",
  "NONE",
  "CUSTOM",
]);
const knownOperatingSystems = new Set(["macos", "windows", "linux"]);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function validateManifest(fileName, manifest) {
  assert(manifest.schema_version === 1, `${fileName}: unsupported schema_version`);
  assert(
    typeof manifest.id === "string" && /^[a-z][a-z0-9.-]{2,63}$/.test(manifest.id),
    `${fileName}: invalid provider id`,
  );
  assert(providerTypes.has(manifest.provider_type), `${fileName}: invalid provider_type`);
  assert(authStrategies.has(manifest.auth_strategy), `${fileName}: invalid auth_strategy`);
  assert(
    Array.isArray(manifest.supported_os) &&
      manifest.supported_os.length > 0 &&
      manifest.supported_os.every((os) => knownOperatingSystems.has(os)),
    `${fileName}: invalid supported_os`,
  );
  assert(Array.isArray(manifest.capabilities), `${fileName}: capabilities must be an array`);
  assert(
    new Set(manifest.capabilities).size === manifest.capabilities.length,
    `${fileName}: duplicate capability`,
  );
  assert(
    manifest.auth_strategy !== "INHERIT_CLI_SESSION" ||
      manifest.metadata?.credential_access === "forbidden",
    `${fileName}: inherited CLI sessions must forbid credential access`,
  );
  assert(
    !manifest.metadata?.forbidden_flags ||
      manifest.metadata.forbidden_flags.includes("--yolo"),
    `${fileName}: Codex dangerous aliases must be blocked together`,
  );
}

const files = (await readdir(providersDirectory))
  .filter((fileName) => fileName.endsWith(".json") && !ignoredSchemas.has(fileName))
  .sort()
  .map((fileName) => ({
    displayName: fileName,
    path: resolve(providersDirectory, fileName),
  }));
files.push({
  displayName: "examples/generic-provider/provider.manifest.json",
  path: resolve(projectRoot, "examples/generic-provider/provider.manifest.json"),
});
const identifiers = new Set();

for (const file of files) {
  const contents = await readFile(file.path, "utf8");
  const manifest = JSON.parse(contents);
  validateManifest(file.displayName, manifest);
  assert(!identifiers.has(manifest.id), `${file.displayName}: duplicate id ${manifest.id}`);
  identifiers.add(manifest.id);
}

assert(files.length > 0, "No provider manifests found");
process.stdout.write(`Validated ${files.length} provider manifests.\n`);
