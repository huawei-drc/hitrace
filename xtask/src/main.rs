use anyhow::{anyhow, bail, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

const TRACE_PATH_ON_DEVICE: &str = "/data/local/tmp/hitrace-xtask-smoke.ftrace";
const TRACE_CATEGORY: &str = "app";
const EXPECTED_TRACE_PATH: &str = "hitrace/tests/ohos_trace_smoke.expected";
const FILTERED_TRACE_PATH: &str = "target/xtask/hitrace-xtask-smoke.filtered";
const SYNC_SPAN_NAME: &str = "hitrace_xtask_sync_span";
const EX_SPAN_NAME: &str = "hitrace_xtask_ex_span";
const EX_CUSTOM_ARGS: &str = "phase=hitrace_xtask,result=ok";

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("ohos-trace-smoke") | None => run_ohos_trace_smoke(),
        Some(other) => bail!("unknown xtask subcommand: {other}"),
    }
}

fn run_ohos_trace_smoke() -> Result<()> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask is expected to live under the workspace root")?
        .to_path_buf();
    ensure_ohos_test_runner_installed()?;
    let target = ohos_target()?;
    let linker = discover_linker(&target)?;
    let previous_level = get_trace_level()?;

    hdc_hitrace("--trace_level Info")?;
    hdc_shell(format!("rm -f {TRACE_PATH_ON_DEVICE}"))?;
    hdc_hitrace(format!("--trace_begin {TRACE_CATEGORY}"))?;

    let test_result = run_cargo_ohos_test(&repo_root, &target, &linker);
    let finish_result = hdc_hitrace(format!(
        "--trace_finish -o {TRACE_PATH_ON_DEVICE} {TRACE_CATEGORY}"
    ));
    let restore_result = hdc_hitrace(format!("--trace_level {previous_level}"));

    test_result?;
    finish_result?;
    restore_result?;

    let trace_dir = repo_root.join("target/xtask");
    fs::create_dir_all(&trace_dir).context("failed to create target/xtask")?;
    let trace_path = trace_dir.join("hitrace-xtask-smoke.ftrace");

    hdc_file_recv(TRACE_PATH_ON_DEVICE, &trace_path)?;
    let filtered_trace_path = repo_root.join(FILTERED_TRACE_PATH);
    assert_trace_matches_fixture(&repo_root, &trace_path, &filtered_trace_path)?;

    println!(
        "Verified HiTrace markers in {} using fixture {}",
        trace_path.display(),
        repo_root.join(EXPECTED_TRACE_PATH).display()
    );
    Ok(())
}

fn run_cargo_ohos_test(repo_root: &Path, target: &str, linker: &Path) -> Result<()> {
    let linker_env = cargo_target_env_var(target, "LINKER");
    let runner_env = cargo_target_env_var(target, "RUNNER");
    let status = Command::new("cargo")
        .arg("test")
        .arg("-p")
        .arg("hitrace")
        .arg("--test")
        .arg("ohos_trace_smoke")
        .arg("--features")
        .arg("api-19")
        .arg("--target")
        .arg(target)
        .arg("--")
        .arg("--nocapture")
        .env(&linker_env, linker.as_os_str())
        .env(&runner_env, "ohos-test-runner")
        .current_dir(repo_root)
        .status()
        .context("failed to run cargo test for OpenHarmony")?;

    ensure_success(status, "cargo test through ohos-test-runner")
}

fn ensure_ohos_test_runner_installed() -> Result<()> {
    let path = env::var_os("PATH").context("PATH is not set")?;

    for dir in env::split_paths(&path) {
        let candidate = dir.join("ohos-test-runner");
        if candidate.is_file() {
            return Ok(());
        }
    }

    bail!(
        "ohos-test-runner was not found in PATH.\n\
install it with `cargo install --locked ohos-test-runner`, then rerun `cargo xtask ohos-trace-smoke`."
    )
}

fn ohos_target() -> Result<String> {
    if let Ok(target) = env::var("OHOS_TARGET") {
        return Ok(target);
    }

    let arch = hdc_shell_output("uname -m")
        .context("failed to probe device architecture via hdc; set OHOS_TARGET explicitly")?;
    let arch = arch.trim();

    match arch {
        "aarch64" | "arm64" => Ok("aarch64-unknown-linux-ohos".to_owned()),
        "x86_64" => Ok("x86_64-unknown-linux-ohos".to_owned()),
        "armv7l" | "armv7" => Ok("armv7-unknown-linux-ohos".to_owned()),
        other => bail!(
            "unsupported OpenHarmony device architecture `{other}` reported by `hdc shell uname -m`; set OHOS_TARGET explicitly"
        ),
    }
}

fn cargo_target_env_var(target: &str, suffix: &str) -> String {
    format!(
        "CARGO_TARGET_{}_{}",
        target.replace('-', "_").to_uppercase(),
        suffix
    )
}

fn discover_linker(target: &str) -> Result<PathBuf> {
    let linker_env = cargo_target_env_var(target, "LINKER");
    if let Some(linker) = env::var_os(&linker_env) {
        return Ok(PathBuf::from(linker));
    }

    let home = PathBuf::from(env::var_os("HOME").context("HOME is not set")?);
    let sdk_root = home.join("Library/OpenHarmony/Sdk");
    let mut best: Option<(u32, PathBuf)> = None;

    for entry in
        fs::read_dir(&sdk_root).with_context(|| format!("failed to read {}", sdk_root.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Ok(version) = name.parse::<u32>() else {
            continue;
        };
        let candidate = entry.path().join(format!("native/llvm/bin/{target}-clang"));
        if candidate.is_file() {
            match &best {
                Some((best_version, _)) if *best_version >= version => {}
                _ => best = Some((version, candidate)),
            }
        }
    }

    best.map(|(_, path)| path).ok_or_else(|| {
        anyhow!("could not find the OpenHarmony clang linker for target {target}; set {linker_env}")
    })
}

fn get_trace_level() -> Result<String> {
    let output = hdc_shell_output("hitrace --get_level")?;
    let line = output
        .lines()
        .find(|line| line.contains("current trace level threshold"))
        .context("failed to parse current hitrace trace level")?;
    line.split_whitespace()
        .last()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("failed to extract current hitrace trace level from: {line}"))
}

fn hdc_shell<S: AsRef<str>>(command: S) -> Result<()> {
    let status = Command::new("hdc")
        .arg("shell")
        .arg(command.as_ref())
        .status()
        .with_context(|| format!("failed to run hdc shell command: {}", command.as_ref()))?;
    ensure_success(status, &format!("hdc shell {}", command.as_ref()))
}

// The hitrace CLI exits 0 even when recording setup fails, only signalling the
// failure via lines like ` error: OpenRecording failed` or `[Fail]…`. Capture
// both streams so we can detect those and fail loudly.
fn hdc_hitrace<S: AsRef<str>>(args: S) -> Result<()> {
    use std::io::Write;
    let args = args.as_ref();
    let cmd = format!("hitrace {args}");
    let output = Command::new("hdc")
        .arg("shell")
        .arg(&cmd)
        .output()
        .with_context(|| format!("failed to run hdc shell command: {cmd}"))?;

    let _ = std::io::stdout().write_all(&output.stdout);
    let _ = std::io::stderr().write_all(&output.stderr);

    ensure_success(output.status, &format!("hdc shell {cmd}"))?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(line) = combined
        .lines()
        .find(|line| line.contains(" error:") || line.contains("[Fail]"))
    {
        bail!("`hitrace {args}` reported a failure: {line}");
    }
    Ok(())
}

fn hdc_shell_output<S: AsRef<str>>(command: S) -> Result<String> {
    let output = Command::new("hdc")
        .arg("shell")
        .arg(command.as_ref())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("failed to run hdc shell command: {}", command.as_ref()))?;
    ensure_success(output.status, &format!("hdc shell {}", command.as_ref()))?;
    String::from_utf8(output.stdout).context("hdc shell output was not valid UTF-8")
}

fn hdc_file_recv(remote: &str, local: &Path) -> Result<()> {
    let status = Command::new("hdc")
        .arg("file")
        .arg("recv")
        .arg(remote)
        .arg(local)
        .status()
        .with_context(|| format!("failed to pull {remote} from the device"))?;
    ensure_success(
        status,
        &format!("hdc file recv {remote} {}", local.display()),
    )
}

fn assert_trace_matches_fixture(
    repo_root: &Path,
    trace_path: &Path,
    filtered_trace_path: &Path,
) -> Result<()> {
    let trace = fs::read_to_string(trace_path)
        .with_context(|| format!("failed to read {}", trace_path.display()))?;
    let filtered = filter_trace_output(&trace)?;
    let filtered_with_newline = format!("{}\n", filtered.join("\n"));

    if let Some(parent) = filtered_trace_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(filtered_trace_path, &filtered_with_newline).with_context(|| {
        format!(
            "failed to write filtered trace to {}",
            filtered_trace_path.display()
        )
    })?;

    let expected_path = repo_root.join(EXPECTED_TRACE_PATH);
    let expected = fs::read_to_string(&expected_path)
        .with_context(|| format!("failed to read {}", expected_path.display()))?;

    if filtered_with_newline != expected {
        bail!(
            "filtered trace output did not match fixture {}\nactual filtered output written to {}",
            expected_path.display(),
            filtered_trace_path.display()
        );
    }

    Ok(())
}

fn filter_trace_output(trace: &str) -> Result<Vec<String>> {
    let trace_lines: Vec<&str> = trace.lines().filter_map(extract_trace_payload).collect();

    let pid = trace_lines
        .iter()
        .find(|payload| payload.contains(&format!("H:{SYNC_SPAN_NAME}")))
        .and_then(|payload| payload.split('|').nth(1))
        .context("failed to find the smoke-test trace PID in the captured trace")?;

    let filtered: Vec<String> = trace_lines
        .into_iter()
        .filter(|payload| payload.split('|').nth(1) == Some(pid))
        .map(normalize_trace_payload)
        .collect();

    if filtered.is_empty() {
        bail!("did not find any tracing_mark_write output for the smoke-test process");
    }

    if !filtered
        .iter()
        .any(|payload| payload.contains(&format!("H:{EX_SPAN_NAME}")))
    {
        bail!("filtered trace output is missing the API-19 ex span marker");
    }

    Ok(filtered)
}

fn extract_trace_payload(line: &str) -> Option<&str> {
    line.split("tracing_mark_write: ").nth(1).map(str::trim)
}

fn normalize_trace_payload(payload: &str) -> String {
    let mut segments = payload.split('|');
    let mut normalized = Vec::new();

    if let Some(kind) = segments.next() {
        normalized.push(kind.to_owned());
    }

    if segments.next().is_some() {
        normalized.push("<PID>".to_owned());
    }

    for segment in segments {
        normalized.push(normalize_trace_segment(segment));
    }

    normalized.join("|")
}

fn normalize_trace_segment(segment: &str) -> String {
    if segment == EX_CUSTOM_ARGS
        || segment.starts_with("H:")
        || segment.chars().all(|ch| ch.is_ascii_digit())
    {
        return segment.to_owned();
    }

    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    if matches!(first, 'D' | 'I' | 'C' | 'M') && chars.clone().all(|ch| ch.is_ascii_digit()) {
        return format!("{first}<TAG>");
    }

    segment.to_owned()
}

fn ensure_success(status: ExitStatus, context: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("{context} failed with status {status}");
    }
}
