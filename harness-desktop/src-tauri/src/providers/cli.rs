use super::AgentProvider;
use crate::domain::{Capability, ProviderKind, ProviderSnapshot, ProviderStatus};
use crate::error::{HarnessError, HarnessResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

pub const CLAUDE_PROVIDER_ID: &str = "anthropic.claude-code";
pub const CODEX_PROVIDER_ID: &str = "openai.codex-cli";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_COMMAND_OUTPUT: u64 = 128 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionAccess {
    ReadOnly,
    WorkspaceWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeCliInvocation {
    pub provider_id: String,
    pub executable: String,
    pub args: Vec<String>,
    pub working_directory: String,
    pub prompt_via_stdin: bool,
    pub access: ExecutionAccess,
}

impl SafeCliInvocation {
    pub fn validate(&self) -> HarnessResult<()> {
        if !matches!(
            self.provider_id.as_str(),
            CLAUDE_PROVIDER_ID | CODEX_PROVIDER_ID
        ) {
            return Err(HarnessError::Validation(
                "only built-in constrained CLI invocations are accepted".into(),
            ));
        }
        if self.working_directory.trim().is_empty() {
            return Err(HarnessError::Validation(
                "a workspace directory is required".into(),
            ));
        }
        const FORBIDDEN: &[&str] = &[
            "--dangerously-skip-permissions",
            "--dangerously-bypass-approvals-and-sandbox",
            "--yolo",
            "--sandbox=danger-full-access",
        ];
        if self
            .args
            .iter()
            .any(|arg| FORBIDDEN.iter().any(|forbidden| arg == forbidden))
        {
            return Err(HarnessError::PermissionDenied(
                "unsafe CLI bypass flags are forbidden".into(),
            ));
        }
        if !self.prompt_via_stdin {
            return Err(HarnessError::Validation(
                "prompts must be sent via stdin to avoid process-list disclosure".into(),
            ));
        }
        Ok(())
    }
}

pub fn claude_invocation(
    executable: &Path,
    workspace: &Path,
    access: ExecutionAccess,
    resume_session: Option<&str>,
) -> HarnessResult<SafeCliInvocation> {
    let mut args = vec![
        "-p".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--include-partial-messages".into(),
        "--permission-mode".into(),
        match access {
            ExecutionAccess::ReadOnly => "plan",
            ExecutionAccess::WorkspaceWrite => "default",
        }
        .into(),
    ];
    if let Some(session) = validated_session_id(resume_session)? {
        args.push("--resume".into());
        args.push(session);
    }
    let invocation = SafeCliInvocation {
        provider_id: CLAUDE_PROVIDER_ID.into(),
        executable: path_string(executable)?,
        args,
        working_directory: path_string(workspace)?,
        prompt_via_stdin: true,
        access,
    };
    invocation.validate()?;
    Ok(invocation)
}

pub fn codex_invocation(
    executable: &Path,
    workspace: &Path,
    access: ExecutionAccess,
    resume_session: Option<&str>,
) -> HarnessResult<SafeCliInvocation> {
    let mut args = vec![
        "exec".into(),
        "--json".into(),
        "--cd".into(),
        path_string(workspace)?,
        "--sandbox".into(),
        match access {
            ExecutionAccess::ReadOnly => "read-only",
            ExecutionAccess::WorkspaceWrite => "workspace-write",
        }
        .into(),
    ];
    if let Some(session) = validated_session_id(resume_session)? {
        args.push("resume".into());
        args.push(session);
    }
    args.push("-".into());
    let invocation = SafeCliInvocation {
        provider_id: CODEX_PROVIDER_ID.into(),
        executable: path_string(executable)?,
        args,
        working_directory: path_string(workspace)?,
        prompt_via_stdin: true,
        access,
    };
    invocation.validate()?;
    Ok(invocation)
}

pub fn detect_builtin_cli_providers() -> Vec<ProviderSnapshot> {
    [CliProvider::claude(), CliProvider::codex()]
        .into_iter()
        .map(|provider| {
            provider
                .detect()
                .unwrap_or_else(|error| provider.error_snapshot(error))
        })
        .collect()
}

#[derive(Clone)]
struct CliProvider {
    id: &'static str,
    name: &'static str,
    candidates: &'static [&'static str],
    version_args: &'static [&'static str],
    auth_args: &'static [&'static str],
    capabilities: &'static [Capability],
}

impl CliProvider {
    fn claude() -> Self {
        Self {
            id: CLAUDE_PROVIDER_ID,
            name: "Claude Code",
            candidates: &["claude"],
            version_args: &["--version"],
            auth_args: &["auth", "status"],
            capabilities: &[
                Capability::Coding,
                Capability::Reasoning,
                Capability::FileAccess,
                Capability::Shell,
                Capability::Git,
                Capability::ToolCalling,
                Capability::LongContext,
                Capability::ParallelAgents,
                Capability::SessionResume,
                Capability::TextGeneration,
            ],
        }
    }

    fn codex() -> Self {
        Self {
            id: CODEX_PROVIDER_ID,
            name: "Codex CLI",
            candidates: &["codex"],
            version_args: &["--version"],
            auth_args: &["login", "status"],
            capabilities: &[
                Capability::Coding,
                Capability::Reasoning,
                Capability::FileAccess,
                Capability::Shell,
                Capability::Git,
                Capability::ToolCalling,
                Capability::ParallelAgents,
                Capability::SessionResume,
                Capability::TextGeneration,
            ],
        }
    }

    fn error_snapshot(&self, error: HarnessError) -> ProviderSnapshot {
        ProviderSnapshot {
            id: self.id.into(),
            name: self.name.into(),
            kind: ProviderKind::Cli,
            status: ProviderStatus::Error,
            enabled: true,
            version: None,
            capabilities: self.capabilities.iter().copied().collect(),
            models: vec![],
            status_detail: Some(error.to_string()),
            executable_path: None,
            detected_at: Utc::now(),
        }
    }
}

impl AgentProvider for CliProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn detect(&self) -> HarnessResult<ProviderSnapshot> {
        let detected_at = Utc::now();
        let Some(executable) = find_executable(self.candidates) else {
            return Ok(ProviderSnapshot {
                id: self.id.into(),
                name: self.name.into(),
                kind: ProviderKind::Cli,
                status: ProviderStatus::Unavailable,
                enabled: true,
                version: None,
                capabilities: self.capabilities.iter().copied().collect(),
                models: vec![],
                status_detail: Some("CLI executable was not found on PATH".into()),
                executable_path: None,
                detected_at,
            });
        };

        let version_result = run_bounded(&executable, self.version_args, COMMAND_TIMEOUT)?;
        let version = if version_result.status.success() {
            sanitized_first_line(&version_result.stdout)
                .or_else(|| sanitized_first_line(&version_result.stderr))
        } else {
            None
        };
        let auth_result = run_bounded(&executable, self.auth_args, COMMAND_TIMEOUT)?;
        // Auth output is intentionally neither returned nor logged: it can contain account data.
        let status = if auth_result.status.success() {
            ProviderStatus::Connected
        } else {
            ProviderStatus::LoginRequired
        };
        let status_detail = match status {
            ProviderStatus::Connected => {
                Some("Authenticated through the vendor CLI session".into())
            }
            _ => Some(
                "CLI is installed but its official auth-status command was not successful".into(),
            ),
        };
        Ok(ProviderSnapshot {
            id: self.id.into(),
            name: self.name.into(),
            kind: ProviderKind::Cli,
            status,
            enabled: true,
            version,
            capabilities: self.capabilities.iter().copied().collect(),
            models: vec![],
            status_detail,
            executable_path: Some(path_string(&executable)?),
            detected_at,
        })
    }
}

struct BoundedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_bounded(
    executable: &Path,
    args: &[&str],
    timeout: Duration,
) -> HarnessResult<BoundedOutput> {
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout_reader = child.stdout.take().map(read_bounded_in_thread);
    let stderr_reader = child.stderr.take().map(read_bounded_in_thread);
    let status = match child.wait_timeout(timeout)? {
        Some(status) => status,
        None => {
            child.kill()?;
            let _ = child.wait();
            return Err(HarnessError::Unavailable(format!(
                "{} did not respond within {} seconds",
                executable.display(),
                timeout.as_secs()
            )));
        }
    };
    let stdout = join_reader(stdout_reader)?;
    let stderr = join_reader(stderr_reader)?;
    Ok(BoundedOutput {
        status,
        stdout,
        stderr,
    })
}

fn read_bounded_in_thread(mut file: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let _ = file
            .by_ref()
            .take(MAX_COMMAND_OUTPUT)
            .read_to_end(&mut output);
        output
    })
}

fn join_reader(reader: Option<thread::JoinHandle<Vec<u8>>>) -> HarnessResult<Vec<u8>> {
    match reader {
        Some(reader) => reader
            .join()
            .map_err(|_| HarnessError::Unavailable("CLI output reader panicked".into())),
        None => Ok(vec![]),
    }
}

fn find_executable(candidates: &[&str]) -> Option<PathBuf> {
    let extensions: Vec<OsString> = if cfg!(windows) {
        env::var_os("PATHEXT")
            .unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"))
            .to_string_lossy()
            .split(';')
            .map(OsString::from)
            .collect()
    } else {
        vec![OsString::new()]
    };
    for directory in executable_search_directories() {
        for candidate in candidates {
            for extension in &extensions {
                let mut filename = OsString::from(candidate);
                filename.push(extension);
                let path = directory.join(filename);
                if is_executable(&path) {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn executable_search_directories() -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();

    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        let home = PathBuf::from(home);
        directories.extend([
            home.join(".local/bin"),
            home.join(".npm-global/bin"),
            home.join("Library/pnpm"),
        ]);
        #[cfg(target_os = "macos")]
        directories.push(home.join("Applications/ChatGPT.app/Contents/Resources"));
    }

    #[cfg(target_os = "macos")]
    directories.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/Applications/ChatGPT.app/Contents/Resources"),
    ]);

    #[cfg(windows)]
    {
        if let Some(app_data) = env::var_os("APPDATA") {
            directories.push(PathBuf::from(app_data).join("npm"));
        }
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            directories.push(PathBuf::from(local_app_data).join("Programs/nodejs"));
        }
    }

    let mut unique = Vec::with_capacity(directories.len());
    for directory in directories {
        if !unique.contains(&directory) {
            unique.push(directory);
        }
    }
    unique
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn sanitized_first_line(output: &[u8]) -> Option<String> {
    let line = String::from_utf8_lossy(output)
        .lines()
        .next()?
        .chars()
        .filter(|character| !character.is_control())
        .take(200)
        .collect::<String>();
    (!line.trim().is_empty()).then(|| line.trim().to_owned())
}

fn validated_session_id(value: Option<&str>) -> HarnessResult<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let valid = !value.is_empty()
        && value.len() <= 200
        && !value.starts_with('-')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character));
    if !valid {
        return Err(HarnessError::Validation(
            "invalid provider session id".into(),
        ));
    }
    Ok(Some(value.into()))
}

fn path_string(path: &Path) -> HarnessResult<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| HarnessError::Validation("path is not valid UTF-8".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_plan_uses_documented_sandbox_and_stdin() {
        let plan = codex_invocation(
            Path::new("/usr/local/bin/codex"),
            Path::new("/work/project"),
            ExecutionAccess::ReadOnly,
            Some("session-1"),
        )
        .unwrap();
        assert!(plan
            .args
            .windows(2)
            .any(|args| args == ["--sandbox", "read-only"]));
        assert!(plan
            .args
            .windows(2)
            .any(|args| args == ["resume", "session-1"]));
        assert_eq!(plan.args.last().unwrap(), "-");
        assert!(plan.prompt_via_stdin);
    }

    #[test]
    fn claude_plan_uses_stream_json_and_no_prompt_argument() {
        let plan = claude_invocation(
            Path::new("/usr/local/bin/claude"),
            Path::new("/work/project"),
            ExecutionAccess::WorkspaceWrite,
            None,
        )
        .unwrap();
        assert!(plan
            .args
            .windows(2)
            .any(|args| args == ["--output-format", "stream-json"]));
        assert!(plan
            .args
            .windows(2)
            .any(|args| args == ["--permission-mode", "default"]));
        assert!(!plan.args.iter().any(|arg| arg.contains("prompt")));
    }

    #[test]
    fn rejects_session_ids_that_could_become_flags() {
        let result = codex_invocation(
            Path::new("codex"),
            Path::new("/work"),
            ExecutionAccess::ReadOnly,
            Some("--dangerous"),
        );
        assert!(matches!(result, Err(HarnessError::Validation(_))));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn gui_search_includes_standard_macos_cli_locations() {
        let directories = executable_search_directories();
        assert!(directories.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(directories.contains(&PathBuf::from(
            "/Applications/ChatGPT.app/Contents/Resources"
        )));
    }
}
