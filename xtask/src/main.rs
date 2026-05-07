use anyhow::{anyhow, bail, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

const TRACE_CATEGORY: &str = "app";

struct Scenario {
    name: &'static str,
    test_binary: &'static str,
    pinpoint_marker: &'static str,
    features: &'static [&'static str],
    fixture: &'static str,
}

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "basic",
        test_binary: "ohos_trace_basic",
        pinpoint_marker: "H:hitrace_xtask_basic_sync",
        features: &[],
        fixture: "hitrace/tests/ohos_trace_basic.expected",
    },
    Scenario {
        name: "api19",
        test_binary: "ohos_trace_api19",
        pinpoint_marker: "H:hitrace_xtask_api19_span",
        features: &["api-19"],
        fixture: "hitrace/tests/ohos_trace_api19.expected",
    },
    Scenario {
        name: "scoped",
        test_binary: "ohos_trace_scoped",
        pinpoint_marker: "H:hitrace_xtask_scoped_default",
        features: &["api-19"],
        fixture: "hitrace/tests/ohos_trace_scoped.expected",
    },
    Scenario {
        name: "macro",
        test_binary: "ohos_trace_macro",
        pinpoint_marker: "H:ohos_trace_macro::hitrace_xtask_macro_target",
        features: &[],
        fixture: "hitrace/tests/ohos_trace_macro.expected",
    },
];

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

    let outcome = SCENARIOS
        .iter()
        .try_for_each(|scenario| run_scenario(&repo_root, &target, &linker, scenario));

    let restore_result = hdc_hitrace(format!("--trace_level {previous_level}"));

    outcome?;
    restore_result?;

    println!("Verified all hitrace scenarios.");
    Ok(())
}

fn run_scenario(repo_root: &Path, target: &str, linker: &Path, scenario: &Scenario) -> Result<()> {
    println!("=== running scenario `{}` ===", scenario.name);

    let trace_path_on_device = format!("/data/local/tmp/hitrace-xtask-{}.ftrace", scenario.name);
    let trace_dir = repo_root.join("target/xtask");
    fs::create_dir_all(&trace_dir).context("failed to create target/xtask")?;
    let local_trace_path = trace_dir.join(format!("hitrace-xtask-{}.ftrace", scenario.name));
    let filtered_trace_path = trace_dir.join(format!("hitrace-xtask-{}.filtered", scenario.name));

    hdc_shell(format!("rm -f {trace_path_on_device}"))?;
    hdc_hitrace(format!("--trace_begin {TRACE_CATEGORY}"))?;

    let test_result = run_cargo_ohos_test(repo_root, target, linker, scenario);
    let finish_result = hdc_hitrace(format!(
        "--trace_finish -o {trace_path_on_device} {TRACE_CATEGORY}"
    ));

    test_result?;
    finish_result?;

    hdc_file_recv(&trace_path_on_device, &local_trace_path)?;
    assert_trace_matches_fixture(repo_root, &local_trace_path, &filtered_trace_path, scenario)?;

    println!(
        "Verified `{}` markers in {} using fixture {}",
        scenario.name,
        local_trace_path.display(),
        repo_root.join(scenario.fixture).display()
    );
    Ok(())
}

fn run_cargo_ohos_test(
    repo_root: &Path,
    target: &str,
    linker: &Path,
    scenario: &Scenario,
) -> Result<()> {
    let linker_env = cargo_target_env_var(target, "LINKER");
    let runner_env = cargo_target_env_var(target, "RUNNER");
    let mut cmd = Command::new("cargo");
    cmd.arg("test")
        .arg("-p")
        .arg("hitrace")
        .arg("--test")
        .arg(scenario.test_binary);
    if !scenario.features.is_empty() {
        cmd.arg("--features").arg(scenario.features.join(","));
    }
    cmd.arg("--target")
        .arg(target)
        .arg("--")
        .arg("--nocapture")
        .env(&linker_env, linker.as_os_str())
        .env(&runner_env, "ohos-test-runner")
        .current_dir(repo_root);

    let status = cmd
        .status()
        .context("failed to run cargo test for OpenHarmony")?;

    ensure_success(
        status,
        &format!(
            "cargo test --test {} through ohos-test-runner",
            scenario.test_binary
        ),
    )
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
    scenario: &Scenario,
) -> Result<()> {
    let trace = fs::read_to_string(trace_path)
        .with_context(|| format!("failed to read {}", trace_path.display()))?;
    let filtered = filter_trace_output(&trace, scenario.pinpoint_marker)?;
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

    let expected_path = repo_root.join(scenario.fixture);
    let expected = fs::read_to_string(&expected_path)
        .with_context(|| format!("failed to read {}", expected_path.display()))?;

    if filtered_with_newline != expected {
        bail!(
            "filtered trace output for `{}` did not match fixture {}\nactual filtered output written to {}",
            scenario.name,
            expected_path.display(),
            filtered_trace_path.display()
        );
    }

    Ok(())
}

fn filter_trace_output(trace: &str, pinpoint_marker: &str) -> Result<Vec<String>> {
    let trace_lines: Vec<&str> = trace.lines().filter_map(extract_trace_payload).collect();

    let pid = trace_lines
        .iter()
        .find(|payload| payload.contains(pinpoint_marker))
        .and_then(|payload| payload.split('|').nth(1))
        .with_context(|| {
            format!("failed to find the pinpoint marker `{pinpoint_marker}` in the captured trace")
        })?;

    let filtered: Vec<String> = trace_lines
        .into_iter()
        .filter(|payload| payload.split('|').nth(1) == Some(pid))
        .map(normalize_trace_payload)
        .collect();

    if filtered.is_empty() {
        bail!("did not find any tracing_mark_write output for the test process");
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
    if segment.starts_with("H:") || segment.chars().all(|ch| ch.is_ascii_digit()) {
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
