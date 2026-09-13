use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::errors::FittingError;
use super::path_model::feffpath;
use super::types::{
    FeffExecutionMode, FeffFlavor, FeffModuleCommand, FeffPathModel, FeffResolvedCommands,
    FeffRunRequest, FeffRunResult,
};

const FEFF85L_MODULES: [(&str, &str); 6] = [
    ("rdinp", "feff8l_rdinp"),
    ("pot", "feff8l_pot"),
    ("xsph", "feff8l_xsph"),
    ("pathfinder", "feff8l_pathfinder"),
    ("genfmt", "feff8l_genfmt"),
    ("ff2x", "feff8l_ff2x"),
];
#[cfg(all(feature = "feff10-runner", not(windows)))]
const FEFF10_MODULE_PREFIX: &str = "feff10::";
/// Environment variable that names the FEFF10 command-line helper explicitly.
/// It overrides the bundled helper on Windows; an empty value is ignored.
pub const FEFF10_HELPER_ENV: &str = "REXAFS_FEFF10_EXECUTABLE";
/// File name of the FEFF10 command-line helper bundled with Windows packages.
/// It is the upstream `feff10-rs` executable built for the MinGW toolchain.
pub const FEFF10_HELPER_EXECUTABLE: &str = "feff10-rs.exe";
/// Directory of the bundled helper and its runtime libraries, relative to the
/// directory containing the rexafs executable.
pub const FEFF10_HELPER_DIRECTORY: &str = "resources/feff10";
/// Stage name recorded for the single helper process on Windows.
pub const FEFF10_HELPER_MODULE: &str = "feff10-rs";
/// Upper bound on FEFF10 stages, used to bound the complete helper run.
#[cfg(all(feature = "feff10-runner", windows))]
const FEFF10_MAX_STAGES: u64 = 18;
#[cfg(feature = "refeff-runner")]
const REFEFF_MODULE_PREFIX: &str = "refeff::";
#[cfg(feature = "refeff-runner")]
const REFEFF_MODULES: [&str; 24] = [
    "rdinp",
    "atomic",
    "pot",
    "ldos",
    "screen",
    "crpa",
    "opconsat",
    "xsph",
    "fms",
    "mkgtr",
    "path",
    "genfmt",
    "ff2x",
    "sfconv",
    "compton",
    "eels",
    "eelsmdff",
    "rhorrp",
    "dmdw",
    "band",
    "fullspectrum",
    "rixs",
    "self",
    "wpot",
];
const NO_EXTERNAL_MODULES: [(&str, &str); 0] = [];

/// Resolve requested external executables or internal stages without running them.
/// Missing executables and disabled backend features return [`FittingError`].
pub fn resolve_feff_commands(
    request: &FeffRunRequest,
) -> Result<FeffResolvedCommands, FittingError> {
    match request.mode {
        FeffExecutionMode::Feff85LModules => resolve_feff85l_commands(request),
        FeffExecutionMode::Feff10Pipeline => resolve_feff10_commands(request.mode),
        FeffExecutionMode::RefeffPipeline => resolve_refeff_commands(request.mode),
    }
}

/// Run a scattering calculation, writing generated files/logs into its workspace.
/// See [`FeffRunRequest`] for backend-specific timeouts and artifact retention.
/// Invalid inputs, unsupported backends, timeout, or calculation failure return errors.
pub fn run_feff(request: &FeffRunRequest) -> Result<FeffRunResult, FittingError> {
    match request.mode {
        FeffExecutionMode::Feff85LModules => run_feff85l_modules(request),
        FeffExecutionMode::Feff10Pipeline => run_feff10_pipeline(request),
        FeffExecutionMode::RefeffPipeline => run_refeff_pipeline(request),
    }
}

fn resolve_feff85l_commands(
    request: &FeffRunRequest,
) -> Result<FeffResolvedCommands, FittingError> {
    let executable_path = validate_executable_path(&request.executable_path)?;
    let base_dir = executable_path
        .parent()
        .ok_or_else(|| FittingError::InvalidExecutablePath {
            path: request.executable_path.display().to_string(),
            reason: "executable has no parent directory".to_string(),
        })?;

    let mut resolved_modules = Vec::with_capacity(FEFF85L_MODULES.len());
    for &(module_label, module_bin) in required_modules(request.mode) {
        let module_file = platform_executable_name(module_bin);
        let sibling_candidate = base_dir.join(&module_file);
        if is_executable_file(&sibling_candidate) {
            resolved_modules.push(FeffModuleCommand {
                module: module_label.to_string(),
                executable: sibling_candidate,
            });
            continue;
        }

        if let Some(path_candidate) = lookup_in_path(&module_file) {
            resolved_modules.push(FeffModuleCommand {
                module: module_label.to_string(),
                executable: path_candidate,
            });
            continue;
        }

        return Err(FittingError::ExecutableNotFound {
            module: module_label.to_string(),
        });
    }

    Ok(FeffResolvedCommands {
        mode: request.mode,
        modules: resolved_modules,
    })
}

fn run_feff85l_modules(request: &FeffRunRequest) -> Result<FeffRunResult, FittingError> {
    let workspace_dir = validate_workspace_dir(&request.workspace_dir)?;
    let feffinp_path = resolve_feffinp_path(request, &workspace_dir)?;
    let resolved = resolve_feff85l_commands(request)?;

    let staged_input = stage_feff_input(&workspace_dir, &feffinp_path)?;

    let run_result = (|| {
        let mut logs = Vec::with_capacity(resolved.modules.len());
        for module in &resolved.modules {
            let log_path = run_single_module(module, &workspace_dir, request.timeout_sec)?;
            logs.push(log_path);
        }

        let path_files = discover_path_files(&workspace_dir)?;
        if path_files.is_empty() {
            return Err(FittingError::NoPathOutputs {
                workspace: workspace_dir.display().to_string(),
            });
        }

        Ok(FeffRunResult {
            mode: request.mode,
            workspace_dir: workspace_dir.clone(),
            feffinp_path: feffinp_path.clone(),
            resolved: resolved.clone(),
            logs,
            path_files,
        })
    })();

    let restore_result = staged_input.restore();
    match (run_result, restore_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(run_err), Ok(())) => Err(run_err),
        (Ok(_), Err(restore_err)) => Err(restore_err),
        (Err(run_err), Err(_restore_err)) => Err(run_err),
    }
}

#[cfg(all(feature = "feff10-runner", not(windows)))]
fn resolve_feff10_commands(mode: FeffExecutionMode) -> Result<FeffResolvedCommands, FittingError> {
    let modules = feff10::Stage::all()
        .iter()
        .map(|stage| FeffModuleCommand {
            module: stage.executable_name().to_string(),
            executable: PathBuf::from(format!("{FEFF10_MODULE_PREFIX}{}", stage.executable_name())),
        })
        .collect();

    Ok(FeffResolvedCommands { mode, modules })
}

#[cfg(not(feature = "feff10-runner"))]
fn resolve_feff10_commands(mode: FeffExecutionMode) -> Result<FeffResolvedCommands, FittingError> {
    Err(FittingError::UnsupportedExecutionMode {
        mode,
        reason: "enable the 'feff10-runner' crate feature to use the FEFFRS pipeline".to_string(),
    })
}

/// Locate the FEFF10 helper on Windows: the explicit environment override,
/// then the bundled `resources/feff10` directory beside the rexafs executable,
/// then `PATH`. The helper is the upstream MinGW `feff10-rs` build; the MSVC
/// desktop cannot link the MinGW FEFF10 archive directly.
#[cfg(all(feature = "feff10-runner", windows))]
fn feff10_helper_executable() -> Result<PathBuf, FittingError> {
    if let Some(explicit) = env::var_os(FEFF10_HELPER_ENV).filter(|value| !value.is_empty()) {
        return validate_executable_path(Path::new(&explicit));
    }
    if let Some(directory) = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        let bundled = directory
            .join(FEFF10_HELPER_DIRECTORY)
            .join(FEFF10_HELPER_EXECUTABLE);
        if is_executable_file(&bundled) {
            return validate_executable_path(&bundled);
        }
    }
    lookup_in_path(FEFF10_HELPER_EXECUTABLE).ok_or_else(|| FittingError::ExecutableNotFound {
        module: FEFF10_HELPER_MODULE.to_string(),
    })
}

/// Strip the `\\?\` verbatim prefix that `canonicalize` adds on Windows.
/// `CreateProcess` does not accept verbatim paths for the working directory,
/// so the helper receives ordinary drive or UNC paths.
#[cfg(all(feature = "feff10-runner", windows))]
fn without_verbatim_prefix(path: &Path) -> PathBuf {
    use std::path::{Component, Prefix};
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path.to_path_buf();
    };
    let root = match prefix.kind() {
        Prefix::VerbatimDisk(letter) => format!("{}:\\", letter as char),
        Prefix::VerbatimUNC(server, share) => format!(
            "\\\\{}\\{}\\",
            server.to_string_lossy(),
            share.to_string_lossy()
        ),
        _ => return path.to_path_buf(),
    };
    let mut simplified = PathBuf::from(root);
    simplified.extend(components.filter(|component| !matches!(component, Component::RootDir)));
    simplified
}

#[cfg(all(feature = "feff10-runner", windows))]
fn resolve_feff10_commands(mode: FeffExecutionMode) -> Result<FeffResolvedCommands, FittingError> {
    let executable = without_verbatim_prefix(&feff10_helper_executable()?);
    Ok(FeffResolvedCommands {
        mode,
        modules: vec![FeffModuleCommand {
            module: FEFF10_HELPER_MODULE.to_string(),
            executable,
        }],
    })
}

/// Run FEFF10 through the bundled helper process. The helper parses the
/// prepared `feff.inp`, derives the stages from its CONTROL card, runs each
/// stage in a fresh process with the requested per-stage timeout, and writes
/// the same `feffNNNN.dat`, `paths.dat` and `logN.dat` files as the embedded
/// pipeline. rexafs additionally bounds the complete run to the per-stage
/// timeout multiplied by the maximum stage count.
#[cfg(all(feature = "feff10-runner", windows))]
fn run_feff10_pipeline(request: &FeffRunRequest) -> Result<FeffRunResult, FittingError> {
    let workspace_dir = validate_workspace_dir(&request.workspace_dir)?;
    let feffinp_path = resolve_feffinp_path(request, &workspace_dir)?;
    let executable = without_verbatim_prefix(&feff10_helper_executable()?);
    let helper_dir = without_verbatim_prefix(&workspace_dir);
    let original = fs::read_to_string(&feffinp_path).map_err(|error| FittingError::IOFailed {
        action: "read FEFF10 input".to_string(),
        path: feffinp_path.display().to_string(),
        reason: error.to_string(),
    })?;
    let prepared = prepare_feff10_input(&original, request.use_sfconv);
    let helper_input = helper_dir.join("feff.inp");
    fs::write(&helper_input, prepared).map_err(|error| FittingError::IOFailed {
        action: "write prepared FEFF10 input".to_string(),
        path: helper_input.display().to_string(),
        reason: error.to_string(),
    })?;

    let module = FeffModuleCommand {
        module: FEFF10_HELPER_MODULE.to_string(),
        executable,
    };
    let mut args: Vec<std::ffi::OsString> = vec![
        "run".into(),
        helper_input.clone().into(),
        "--work-dir".into(),
        helper_dir.clone().into(),
        "--quiet".into(),
    ];
    if let Some(timeout_sec) = request.timeout_sec {
        args.push("--timeout".into());
        args.push(timeout_sec.to_string().into());
    }
    let overall_timeout = request
        .timeout_sec
        .map(|timeout_sec| timeout_sec.saturating_mul(FEFF10_MAX_STAGES));
    let helper_log = run_module_command(&module, &args, &helper_dir, overall_timeout)?;

    let resolved = FeffResolvedCommands {
        mode: request.mode,
        modules: vec![module],
    };
    let mut logs = vec![helper_log];
    logs.extend(discover_feff10_logs(&workspace_dir)?);
    let path_files = discover_path_files(&workspace_dir)?;
    if path_files.is_empty() {
        return Err(FittingError::NoPathOutputs {
            workspace: workspace_dir.display().to_string(),
        });
    }

    Ok(FeffRunResult {
        mode: request.mode,
        workspace_dir,
        feffinp_path,
        resolved,
        logs,
        path_files,
    })
}

/// Return the FEFF card keyword of a line, ignoring blank and `*` comment lines.
#[cfg(any(test, all(feature = "feff10-runner", windows)))]
fn feff_card_keyword(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('*') {
        return None;
    }
    trimmed.split_whitespace().next()
}

/// Parse the six PRINT flags of a card line; missing flags are 0.
#[cfg(any(test, all(feature = "feff10-runner", windows)))]
fn parse_print_flags(line: &str) -> [i64; 6] {
    let mut flags = [0i64; 6];
    let values = line
        .split_whitespace()
        .skip(1)
        .take_while(|token| !token.starts_with('*'))
        .map_while(|token| token.parse::<i64>().ok());
    for (slot, value) in flags.iter_mut().zip(values) {
        *slot = value;
    }
    flags
}

/// Insert a card before the POTENTIALS, ATOMS or END block, or append it.
#[cfg(any(test, all(feature = "feff10-runner", windows)))]
fn insert_feff_card(lines: &mut Vec<String>, card: &str) {
    let position = lines
        .iter()
        .position(|line| {
            feff_card_keyword(line).is_some_and(|keyword| {
                ["POTENTIALS", "ATOMS", "END"]
                    .iter()
                    .any(|block| keyword.eq_ignore_ascii_case(block))
            })
        })
        .unwrap_or(lines.len());
    lines.insert(position, card.to_string());
}

/// Prepare input text for the FEFF10 helper. The sixth PRINT flag is raised
/// to at least 3 so FEFF10 writes individual path files, and SFCONV is added
/// when requested. Other cards, comments and atom lists are unchanged. This
/// mirrors the card edits applied to the parsed input by the embedded route.
#[cfg(any(test, all(feature = "feff10-runner", windows)))]
fn prepare_feff10_input(input: &str, use_sfconv: bool) -> String {
    let mut lines: Vec<String> = input.lines().map(str::to_string).collect();
    let print_index = lines
        .iter()
        .position(|line| feff_card_keyword(line).is_some_and(|k| k.eq_ignore_ascii_case("PRINT")));
    match print_index {
        Some(index) => {
            let mut flags = parse_print_flags(&lines[index]);
            if flags[5] < 3 {
                flags[5] = 3;
            }
            lines[index] = format!(
                "PRINT {} {} {} {} {} {}",
                flags[0], flags[1], flags[2], flags[3], flags[4], flags[5]
            );
        }
        None => insert_feff_card(&mut lines, "PRINT 0 0 0 0 0 3"),
    }
    if use_sfconv
        && !lines
            .iter()
            .any(|line| feff_card_keyword(line).is_some_and(|k| k.eq_ignore_ascii_case("SFCONV")))
    {
        insert_feff_card(&mut lines, "SFCONV");
    }
    let mut prepared = lines.join("\n");
    prepared.push('\n');
    prepared
}

#[cfg(all(feature = "feff10-runner", not(windows)))]
fn run_feff10_pipeline(request: &FeffRunRequest) -> Result<FeffRunResult, FittingError> {
    let workspace_dir = validate_workspace_dir(&request.workspace_dir)?;
    let feffinp_path = resolve_feffinp_path(request, &workspace_dir)?;
    let mut input = feff10::FeffInput::from_file(&feffinp_path).map_err(|error| {
        let error_text = error.to_string();
        let strict_hint = if error_text
            .to_ascii_lowercase()
            .contains("unrecognized keyword")
        {
            " (feff10 >= 0.2 uses strict card parsing; verify card keywords)"
        } else {
            ""
        };

        FittingError::Feff10PipelineFailed {
            reason: format!(
                "failed to parse FEFF10 input '{}': {error_text}{strict_hint}",
                feffinp_path.display()
            ),
        }
    })?;

    // FEFF10 must keep ipr6 >= 3 to emit feffNNNN.dat path files for fitting.
    if input.print_flags[5] < 3 {
        input.print_flags[5] = 3;
    }
    if request.use_sfconv {
        ensure_other_card_present(&mut input.other_cards, "SFCONV");
    }

    let mut builder = feff10::FeffConfigBuilder::new()
        .work_dir(&workspace_dir)
        .input(input);
    if let Some(timeout_sec) = request.timeout_sec {
        builder = builder.stage_timeout(Duration::from_secs(timeout_sec));
    }

    let config = builder
        .build()
        .map_err(|error| FittingError::Feff10PipelineFailed {
            reason: format!("failed to build FEFF10 configuration: {error}"),
        })?;

    let result = feff10::FeffPipeline::new(config).run().map_err(|error| {
        FittingError::Feff10PipelineFailed {
            reason: error.to_string(),
        }
    })?;

    let resolved = FeffResolvedCommands {
        mode: request.mode,
        modules: result
            .stages
            .iter()
            .map(|stage_result| {
                let stage_name = stage_result.stage.executable_name();
                FeffModuleCommand {
                    module: stage_name.to_string(),
                    executable: PathBuf::from(format!("{FEFF10_MODULE_PREFIX}{stage_name}")),
                }
            })
            .collect(),
    };

    let logs = discover_feff10_logs(&workspace_dir)?;
    let path_files = discover_path_files(&workspace_dir)?;
    if path_files.is_empty() {
        return Err(FittingError::NoPathOutputs {
            workspace: workspace_dir.display().to_string(),
        });
    }

    Ok(FeffRunResult {
        mode: request.mode,
        workspace_dir,
        feffinp_path,
        resolved,
        logs,
        path_files,
    })
}

#[cfg(all(feature = "feff10-runner", not(windows)))]
fn ensure_other_card_present(other_cards: &mut Vec<String>, keyword: &str) {
    if other_cards.iter().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|first| first.eq_ignore_ascii_case(keyword))
    }) {
        return;
    }
    other_cards.push(keyword.to_string());
}

#[cfg(not(feature = "feff10-runner"))]
fn run_feff10_pipeline(request: &FeffRunRequest) -> Result<FeffRunResult, FittingError> {
    Err(FittingError::UnsupportedExecutionMode {
        mode: request.mode,
        reason: "enable the 'feff10-runner' crate feature to use the FEFFRS pipeline".to_string(),
    })
}

#[cfg(feature = "refeff-runner")]
fn resolve_refeff_commands(mode: FeffExecutionMode) -> Result<FeffResolvedCommands, FittingError> {
    let modules = REFEFF_MODULES
        .iter()
        .map(|module| FeffModuleCommand {
            module: (*module).to_string(),
            executable: PathBuf::from(format!("{REFEFF_MODULE_PREFIX}{module}")),
        })
        .collect();

    Ok(FeffResolvedCommands { mode, modules })
}

#[cfg(not(feature = "refeff-runner"))]
fn resolve_refeff_commands(mode: FeffExecutionMode) -> Result<FeffResolvedCommands, FittingError> {
    Err(FittingError::UnsupportedExecutionMode {
        mode,
        reason:
            "enable the 'refeff-runner' crate feature to use pure-Rust FEFF10 pipeline execution"
                .to_string(),
    })
}

#[cfg(feature = "refeff-runner")]
fn run_refeff_pipeline(request: &FeffRunRequest) -> Result<FeffRunResult, FittingError> {
    let workspace_dir = validate_workspace_dir(&request.workspace_dir)?;
    let feffinp_path = resolve_feffinp_path(request, &workspace_dir)?;
    let original_input =
        fs::read_to_string(&feffinp_path).map_err(|error| FittingError::IOFailed {
            action: "read FEFF input for refeff".to_string(),
            path: feffinp_path.display().to_string(),
            reason: error.to_string(),
        })?;
    let prepared_input = prepare_refeff_input(&original_input, request.use_sfconv);

    // Refeff executes in-process and does not bundle or spawn FEFF executables.
    // Its memory facade uses a private temporary compatibility workspace; only
    // the path files needed by rexafs are materialized for the caller.
    // ReFEFF 0.3 checks a cooperative deadline throughout the calculation.
    let mut runner = refeff::Runner::new();
    if let Some(seconds) = request.timeout_sec {
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(seconds))
            .ok_or_else(|| FittingError::RefeffPipelineFailed {
                reason: "timeout exceeds the platform's monotonic clock range".into(),
            })?;
        runner = runner.with_deadline(deadline);
    }
    let result = runner
        .run_in_memory(refeff::MemoryRunRequest::new(prepared_input.into_bytes()))
        .map_err(|error| {
            if let (
                Some(timeout_sec),
                refeff::Error::Pipeline {
                    code: "interrupted",
                    module,
                    ..
                },
            ) = (request.timeout_sec, &error)
            {
                FittingError::ProcessTimedOut {
                    module: module.clone().unwrap_or_else(|| "refeff".into()),
                    timeout_sec,
                }
            } else {
                FittingError::RefeffPipelineFailed {
                    reason: error.to_string(),
                }
            }
        })?;

    let resolved = FeffResolvedCommands {
        mode: request.mode,
        modules: result
            .report
            .stages
            .iter()
            .map(|stage| FeffModuleCommand {
                module: stage.name.clone(),
                executable: PathBuf::from(format!("{REFEFF_MODULE_PREFIX}{}", stage.name)),
            })
            .collect(),
    };

    let mut path_files = Vec::new();
    for artifact in result.artifacts.iter() {
        let Some(file_name) = artifact.path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let is_path_file = is_feff_path_file_name(file_name);
        if !is_path_file && (!request.keep_all_outputs || artifact.path == Path::new("feff.inp")) {
            continue;
        }
        let destination = if request.keep_all_outputs {
            workspace_dir.join(artifact.path)
        } else {
            workspace_dir.join(file_name)
        };
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| FittingError::IOFailed {
                action: "create ReFEFF output directory".to_string(),
                path: parent.display().to_string(),
                reason: error.to_string(),
            })?;
        }
        fs::write(&destination, artifact.bytes).map_err(|error| FittingError::IOFailed {
            action: "write ReFEFF output".to_string(),
            path: destination.display().to_string(),
            reason: error.to_string(),
        })?;
        if is_path_file {
            path_files.push(destination);
        }
    }
    path_files.sort();

    if path_files.is_empty() {
        return Err(FittingError::NoPathOutputs {
            workspace: workspace_dir.display().to_string(),
        });
    }

    Ok(FeffRunResult {
        mode: request.mode,
        workspace_dir,
        feffinp_path,
        resolved,
        logs: Vec::new(),
        path_files,
    })
}

#[cfg(feature = "refeff-runner")]
fn prepare_refeff_input(input: &str, use_sfconv: bool) -> String {
    let mut output = Vec::new();
    let mut found_print = false;
    let mut found_sfconv = false;
    let mut additions_inserted = false;

    for line in input.lines() {
        let trimmed = line.trim_start();
        let active = !trimmed.starts_with('*') && !trimmed.starts_with('#');
        let keyword = active
            .then(|| trimmed.split_whitespace().next())
            .flatten()
            .unwrap_or_default();

        if keyword.eq_ignore_ascii_case("PRINT") {
            let mut flags: Vec<i32> = trimmed
                .split('*')
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .skip(1)
                .filter_map(|value| value.parse().ok())
                .collect();
            flags.resize(6, 0);
            flags[5] = flags[5].max(3);
            output.push(format!(
                "PRINT {} {} {} {} {} {}",
                flags[0], flags[1], flags[2], flags[3], flags[4], flags[5]
            ));
            found_print = true;
            continue;
        }
        if keyword.eq_ignore_ascii_case("SFCONV") {
            found_sfconv = true;
        }
        if keyword.eq_ignore_ascii_case("END") && !additions_inserted {
            if !found_print {
                output.push("PRINT 0 0 0 0 0 3".to_string());
            }
            if use_sfconv && !found_sfconv {
                output.push("SFCONV".to_string());
            }
            additions_inserted = true;
        }
        output.push(line.to_string());
    }

    if !additions_inserted {
        if !found_print {
            output.push("PRINT 0 0 0 0 0 3".to_string());
        }
        if use_sfconv && !found_sfconv {
            output.push("SFCONV".to_string());
        }
    }

    let mut prepared = output.join("\n");
    prepared.push('\n');
    prepared
}

#[cfg(not(feature = "refeff-runner"))]
fn run_refeff_pipeline(request: &FeffRunRequest) -> Result<FeffRunResult, FittingError> {
    Err(FittingError::UnsupportedExecutionMode {
        mode: request.mode,
        reason:
            "enable the 'refeff-runner' crate feature to use pure-Rust FEFF10 pipeline execution"
                .to_string(),
    })
}

/// Run the requested backend, then read its output files into owned path models.
///
/// This has the filesystem/process side effects of run_feff. Use Feff85L for
/// current compatible path output, including FEFF10/ReFEFF calculations; the
/// Feff10 parser flavor remains unsupported. Returns the first execution or
/// parse error rather than a partial list of successful paths.
pub fn run_feff_and_load_paths(
    request: &FeffRunRequest,
    flavor: FeffFlavor,
) -> Result<Vec<FeffPathModel>, FittingError> {
    let result = run_feff(request)?;
    load_paths_from_run_result(&result, flavor)
}

/// Parse the recorded path-file list in order without rerunning the calculation.
/// The files must still exist. An empty list returns an empty vector; any file
/// or format failure returns an error instead of a partial result. Loaded
/// models own their data and use the parser's documented correction defaults.
pub fn load_paths_from_run_result(
    result: &FeffRunResult,
    flavor: FeffFlavor,
) -> Result<Vec<FeffPathModel>, FittingError> {
    result
        .path_files
        .iter()
        .map(|path| feffpath(path, flavor))
        .collect()
}

fn required_modules(mode: FeffExecutionMode) -> &'static [(&'static str, &'static str)] {
    match mode {
        FeffExecutionMode::Feff85LModules => &FEFF85L_MODULES,
        FeffExecutionMode::Feff10Pipeline | FeffExecutionMode::RefeffPipeline => {
            &NO_EXTERNAL_MODULES
        }
    }
}

fn validate_workspace_dir(workspace_dir: &Path) -> Result<PathBuf, FittingError> {
    if !workspace_dir.exists() || !workspace_dir.is_dir() {
        return Err(FittingError::WorkspaceNotFound {
            path: workspace_dir.display().to_string(),
        });
    }

    fs::canonicalize(workspace_dir).map_err(|error| FittingError::IOFailed {
        action: "canonicalize workspace".to_string(),
        path: workspace_dir.display().to_string(),
        reason: error.to_string(),
    })
}

fn validate_executable_path(path: &Path) -> Result<PathBuf, FittingError> {
    if path.as_os_str().is_empty() {
        return Err(FittingError::InvalidExecutablePath {
            path: path.display().to_string(),
            reason: "path is empty".to_string(),
        });
    }

    if !is_executable_file(path) {
        return Err(FittingError::InvalidExecutablePath {
            path: path.display().to_string(),
            reason: "path does not point to an executable file".to_string(),
        });
    }

    fs::canonicalize(path).map_err(|error| FittingError::InvalidExecutablePath {
        path: path.display().to_string(),
        reason: error.to_string(),
    })
}

fn resolve_feffinp_path(
    request: &FeffRunRequest,
    workspace_dir: &Path,
) -> Result<PathBuf, FittingError> {
    let feffinp = request
        .feffinp
        .clone()
        .unwrap_or_else(|| PathBuf::from("feff.inp"));
    let resolved = if feffinp.is_absolute() {
        feffinp
    } else {
        workspace_dir.join(feffinp)
    };

    if !resolved.exists() || !resolved.is_file() {
        return Err(FittingError::FeffInputNotFound {
            path: resolved.display().to_string(),
        });
    }

    Ok(resolved)
}

fn run_single_module(
    module: &FeffModuleCommand,
    workspace_dir: &Path,
    timeout_sec: Option<u64>,
) -> Result<PathBuf, FittingError> {
    run_module_command(module, &[], workspace_dir, timeout_sec)
}

/// Run one external command with arguments in the workspace, apply the timeout,
/// and record its exit status and output in `feffrun_<module>.log`.
fn run_module_command(
    module: &FeffModuleCommand,
    args: &[std::ffi::OsString],
    workspace_dir: &Path,
    timeout_sec: Option<u64>,
) -> Result<PathBuf, FittingError> {
    let mut child = Command::new(&module.executable)
        .args(args)
        .current_dir(workspace_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| FittingError::ProcessSpawnFailed {
            module: module.module.clone(),
            executable: module.executable.display().to_string(),
            reason: error.to_string(),
        })?;

    let timeout = timeout_sec.map(Duration::from_secs);
    let start = Instant::now();
    let status = wait_for_exit(&mut child, start, timeout, &module.module)?;

    let stdout = read_pipe(child.stdout.take(), &module.module, "stdout")?;
    let stderr = read_pipe(child.stderr.take(), &module.module, "stderr")?;

    let log_name = format!("feffrun_{}.log", module.module);
    let log_path = workspace_dir.join(log_name);
    write_module_log(&log_path, module, status, &stdout, &stderr)?;

    if !status.success() {
        return Err(FittingError::ProcessFailed {
            module: module.module.clone(),
            code: status.code().unwrap_or(-1),
        });
    }

    Ok(log_path)
}

fn wait_for_exit(
    child: &mut std::process::Child,
    start: Instant,
    timeout: Option<Duration>,
    module: &str,
) -> Result<ExitStatus, FittingError> {
    loop {
        match child
            .try_wait()
            .map_err(|error| FittingError::ProcessSpawnFailed {
                module: module.to_string(),
                executable: "<running process>".to_string(),
                reason: error.to_string(),
            })? {
            Some(status) => return Ok(status),
            None => {
                if let Some(limit) = timeout {
                    if start.elapsed() > limit {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(FittingError::ProcessTimedOut {
                            module: module.to_string(),
                            timeout_sec: limit.as_secs(),
                        });
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn write_module_log(
    log_path: &Path,
    module: &FeffModuleCommand,
    status: ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(), FittingError> {
    let mut log = String::new();
    log.push_str(&format!(
        "# module={} executable={} status={}\n",
        module.module,
        module.executable.display(),
        status.code().unwrap_or(-1)
    ));
    log.push_str("\n# stdout\n");
    log.push_str(&String::from_utf8_lossy(stdout));
    log.push_str("\n# stderr\n");
    log.push_str(&String::from_utf8_lossy(stderr));

    fs::write(log_path, log).map_err(|error| FittingError::IOFailed {
        action: "write module log".to_string(),
        path: log_path.display().to_string(),
        reason: error.to_string(),
    })
}

fn read_pipe(
    mut pipe: Option<impl Read>,
    module: &str,
    stream_name: &str,
) -> Result<Vec<u8>, FittingError> {
    let Some(mut stream) = pipe.take() else {
        return Ok(Vec::new());
    };

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|error| FittingError::OutputReadFailed {
            module: module.to_string(),
            reason: format!("{stream_name}: {error}"),
        })?;
    Ok(buf)
}

fn discover_path_files(workspace_dir: &Path) -> Result<Vec<PathBuf>, FittingError> {
    let mut output_paths = Vec::new();

    let entries = fs::read_dir(workspace_dir).map_err(|error| FittingError::IOFailed {
        action: "read workspace directory".to_string(),
        path: workspace_dir.display().to_string(),
        reason: error.to_string(),
    })?;

    for entry_result in entries {
        let entry = entry_result.map_err(|error| FittingError::IOFailed {
            action: "iterate workspace entries".to_string(),
            path: workspace_dir.display().to_string(),
            reason: error.to_string(),
        })?;

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if is_feff_path_file_name(file_name) {
            output_paths.push(path);
        }
    }

    output_paths.sort_by(|lhs, rhs| {
        lhs.file_name()
            .unwrap_or_default()
            .cmp(rhs.file_name().unwrap_or_default())
    });

    Ok(output_paths)
}

fn discover_feff10_logs(workspace_dir: &Path) -> Result<Vec<PathBuf>, FittingError> {
    let mut logs = Vec::new();

    let entries = fs::read_dir(workspace_dir).map_err(|error| FittingError::IOFailed {
        action: "read workspace directory for FEFF10 logs".to_string(),
        path: workspace_dir.display().to_string(),
        reason: error.to_string(),
    })?;

    for entry_result in entries {
        let entry = entry_result.map_err(|error| FittingError::IOFailed {
            action: "iterate workspace entries for FEFF10 logs".to_string(),
            path: workspace_dir.display().to_string(),
            reason: error.to_string(),
        })?;

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if is_feff10_log_file_name(file_name) {
            logs.push(path);
        }
    }

    logs.sort_by(|lhs, rhs| {
        lhs.file_name()
            .unwrap_or_default()
            .cmp(rhs.file_name().unwrap_or_default())
    });

    Ok(logs)
}

fn is_feff_path_file_name(file_name: &str) -> bool {
    if !file_name.starts_with("feff") || !file_name.ends_with(".dat") {
        return false;
    }

    let digits = &file_name[4..file_name.len() - 4];
    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
}

fn is_feff10_log_file_name(file_name: &str) -> bool {
    (file_name.starts_with("log") && file_name.ends_with(".dat")) || file_name == ".feff.error"
}

fn platform_executable_name(base_name: &str) -> String {
    #[cfg(windows)]
    {
        if base_name.to_ascii_lowercase().ends_with(".exe") {
            base_name.to_string()
        } else {
            format!("{base_name}.exe")
        }
    }
    #[cfg(not(windows))]
    {
        base_name.to_string()
    }
}

fn lookup_in_path(executable_name: &str) -> Option<PathBuf> {
    let path_env = env::var_os("PATH")?;
    for directory in env::split_paths(&path_env) {
        let candidate = directory.join(executable_name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }

        #[cfg(windows)]
        {
            if candidate.extension().is_some() {
                continue;
            }
            let pathext = env::var_os("PATHEXT")
                .unwrap_or_else(|| ".EXE;.BAT;.CMD;.COM".into())
                .to_string_lossy()
                .to_string();
            for ext in pathext.split(';') {
                let trimmed = ext.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let ext_clean = trimmed.trim_start_matches('.').to_ascii_lowercase();
                let candidate_with_ext = directory.join(format!("{executable_name}.{ext_clean}"));
                if is_executable_file(&candidate_with_ext) {
                    return Some(candidate_with_ext);
                }
            }
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    if !path.exists() || !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

struct StagedFeffInput {
    staged_path: Option<PathBuf>,
    backup_path: Option<PathBuf>,
    target_path: PathBuf,
}

impl StagedFeffInput {
    fn restore(self) -> Result<(), FittingError> {
        if self.staged_path.is_some() && self.target_path.exists() {
            fs::remove_file(&self.target_path).map_err(|error| FittingError::IOFailed {
                action: "remove staged feff.inp".to_string(),
                path: self.target_path.display().to_string(),
                reason: error.to_string(),
            })?;
        }

        if let Some(backup_path) = self.backup_path {
            fs::rename(&backup_path, &self.target_path).map_err(|error| {
                FittingError::IOFailed {
                    action: "restore original feff.inp".to_string(),
                    path: self.target_path.display().to_string(),
                    reason: error.to_string(),
                }
            })?;
        }

        Ok(())
    }
}

fn stage_feff_input(
    workspace_dir: &Path,
    feffinp_path: &Path,
) -> Result<StagedFeffInput, FittingError> {
    let target_path = workspace_dir.join("feff.inp");
    if feffinp_path == target_path {
        if !target_path.exists() || !target_path.is_file() {
            return Err(FittingError::FeffInputNotFound {
                path: target_path.display().to_string(),
            });
        }
        return Ok(StagedFeffInput {
            staged_path: None,
            backup_path: None,
            target_path,
        });
    }

    let backup_path = if target_path.exists() {
        let backup = workspace_dir.join(".rexafs_feff_inp_backup");
        if backup.exists() {
            fs::remove_file(&backup).map_err(|error| FittingError::IOFailed {
                action: "clear existing feff.inp backup".to_string(),
                path: backup.display().to_string(),
                reason: error.to_string(),
            })?;
        }
        fs::rename(&target_path, &backup).map_err(|error| FittingError::IOFailed {
            action: "backup existing feff.inp".to_string(),
            path: target_path.display().to_string(),
            reason: error.to_string(),
        })?;
        Some(backup)
    } else {
        None
    };

    if let Err(error) = fs::copy(feffinp_path, &target_path) {
        if let Some(backup) = backup_path.as_ref() {
            let _ = fs::rename(backup, &target_path);
        }
        return Err(FittingError::IOFailed {
            action: "stage feff input file".to_string(),
            path: target_path.display().to_string(),
            reason: error.to_string(),
        });
    }

    Ok(StagedFeffInput {
        staged_path: Some(feffinp_path.to_path_buf()),
        backup_path,
        target_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xafs::fitting::ff2chi;
    use crate::xafs::fitting::{FeffFlavor, FitVariables};
    use nalgebra::DVector;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    const TOP_DIR: &str = env!("CARGO_MANIFEST_DIR");

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "{}-{}-{}",
                prefix,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn join(&self, file_name: &str) -> PathBuf {
            self.path.join(file_name)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[cfg(unix)]
    fn write_exec(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;

        let script = format!("#!/bin/sh\nset -eu\n{body}\n");
        fs::write(path, script).unwrap();

        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[cfg(unix)]
    fn install_mock_modules(bin_dir: &Path, ff2x_payload: &str, fail_module: Option<&str>) {
        fs::create_dir_all(bin_dir).unwrap();

        for &(label, executable_name) in FEFF85L_MODULES.iter() {
            let executable_path = bin_dir.join(executable_name);
            let payload = if Some(label) == fail_module {
                "echo intentional failure >&2\nexit 9".to_string()
            } else if label == "ff2x" {
                format!(
                    "if [ ! -f feff.inp ]; then echo missing feff.inp >&2; exit 8; fi\n{}\necho module ff2x done",
                    ff2x_payload
                )
            } else {
                format!(
                    "if [ ! -f feff.inp ]; then echo missing feff.inp >&2; exit 8; fi\necho module {label} done"
                )
            };
            write_exec(&executable_path, &payload);
        }
    }

    fn build_request(executable_path: PathBuf, workspace_dir: PathBuf) -> FeffRunRequest {
        FeffRunRequest {
            executable_path,
            workspace_dir,
            feffinp: None,
            mode: FeffExecutionMode::Feff85LModules,
            timeout_sec: Some(30),
            use_sfconv: false,
            keep_all_outputs: false,
        }
    }

    #[test]
    fn test_platform_executable_name_has_expected_shape() {
        let executable = platform_executable_name("feff8l_rdinp");

        #[cfg(windows)]
        assert!(executable.ends_with(".exe"));

        #[cfg(not(windows))]
        assert_eq!(executable, "feff8l_rdinp");
    }

    #[test]
    fn test_is_feff_path_file_name() {
        assert!(is_feff_path_file_name("feff0001.dat"));
        assert!(is_feff_path_file_name("feff1234.dat"));
        assert!(!is_feff_path_file_name("feff.dat"));
        assert!(!is_feff_path_file_name("feffABCD.dat"));
        assert!(!is_feff_path_file_name("notfeff0001.dat"));
    }

    #[test]
    fn test_is_feff10_log_file_name() {
        assert!(is_feff10_log_file_name("log.dat"));
        assert!(is_feff10_log_file_name("log1.dat"));
        assert!(is_feff10_log_file_name(".feff.error"));
        assert!(!is_feff10_log_file_name("feff0001.dat"));
        assert!(!is_feff10_log_file_name("log.txt"));
    }

    #[cfg(feature = "refeff-runner")]
    #[test]
    fn test_prepare_refeff_input_enables_path_output_and_sfconv_once() {
        let input = "TITLE test\nPRINT 1 0 0 0 0 0\nEXAFS 20\nEND\n";
        let prepared = prepare_refeff_input(input, true);

        assert!(prepared.contains("PRINT 1 0 0 0 0 3"));
        assert_eq!(
            prepared
                .lines()
                .filter(|line| line.trim().eq_ignore_ascii_case("SFCONV"))
                .count(),
            1
        );
    }

    #[cfg(feature = "feff10-runner")]
    #[test]
    fn test_ensure_other_card_present_adds_card_once() {
        let mut cards = vec!["EXAFS 20".to_string()];
        ensure_other_card_present(&mut cards, "SFCONV");
        ensure_other_card_present(&mut cards, "sfconv");
        assert_eq!(
            cards
                .iter()
                .filter(|line| {
                    line.split_whitespace()
                        .next()
                        .is_some_and(|first| first.eq_ignore_ascii_case("SFCONV"))
                })
                .count(),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_commands_from_sibling_modules() {
        let bin_dir = TestDir::new("xfeff-bin");
        install_mock_modules(&bin_dir.path, "", None);

        let request = build_request(bin_dir.join("feff8l_rdinp"), bin_dir.path.clone());
        let resolved = resolve_feff_commands(&request).unwrap();
        let canonical_bin_dir = fs::canonicalize(&bin_dir.path).unwrap();

        assert_eq!(resolved.modules.len(), FEFF85L_MODULES.len());
        assert!(resolved
            .modules
            .iter()
            .all(|module| module.executable.starts_with(&canonical_bin_dir)));
    }

    #[test]
    fn test_resolve_missing_executable_fails() {
        let workspace = TestDir::new("xfeff-work");
        let request = build_request(workspace.join("missing-feff"), workspace.path.clone());

        let err = resolve_feff_commands(&request).unwrap_err();
        assert!(matches!(err, FittingError::InvalidExecutablePath { .. }));
    }

    #[cfg(not(feature = "feff10-runner"))]
    #[test]
    fn test_resolve_feff10_mode_requires_feature() {
        let workspace = TestDir::new("xfeff-work-feff10-resolve");
        let request = FeffRunRequest {
            executable_path: PathBuf::new(),
            workspace_dir: workspace.path.clone(),
            feffinp: None,
            mode: FeffExecutionMode::Feff10Pipeline,
            timeout_sec: None,
            use_sfconv: false,
            keep_all_outputs: false,
        };

        let err = resolve_feff_commands(&request).unwrap_err();
        assert!(matches!(
            err,
            FittingError::UnsupportedExecutionMode {
                mode: FeffExecutionMode::Feff10Pipeline,
                ..
            }
        ));
    }

    #[cfg(not(feature = "feff10-runner"))]
    #[test]
    fn test_run_feff10_mode_requires_feature() {
        let workspace = TestDir::new("xfeff-work-feff10-run");
        let request = FeffRunRequest {
            executable_path: PathBuf::new(),
            workspace_dir: workspace.path.clone(),
            feffinp: None,
            mode: FeffExecutionMode::Feff10Pipeline,
            timeout_sec: None,
            use_sfconv: false,
            keep_all_outputs: false,
        };

        let err = run_feff(&request).unwrap_err();
        assert!(matches!(
            err,
            FittingError::UnsupportedExecutionMode {
                mode: FeffExecutionMode::Feff10Pipeline,
                ..
            }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_falls_back_to_path() {
        let _lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let previous_path = env::var_os("PATH");

        let primary_dir = TestDir::new("xfeff-primary");
        let fallback_dir = TestDir::new("xfeff-fallback");

        fs::create_dir_all(&primary_dir.path).unwrap();
        write_exec(
            &primary_dir.join("feff8l_rdinp"),
            "if [ ! -f feff.inp ]; then :; fi\necho only primary",
        );
        install_mock_modules(&fallback_dir.path, "", None);

        env::set_var("PATH", fallback_dir.path.display().to_string());
        let request = build_request(primary_dir.join("feff8l_rdinp"), primary_dir.path.clone());
        let resolved = resolve_feff_commands(&request).unwrap();

        if let Some(path) = previous_path {
            env::set_var("PATH", path);
        } else {
            env::remove_var("PATH");
        }

        assert_eq!(resolved.modules.len(), FEFF85L_MODULES.len());
        assert_eq!(resolved.modules[0].module, "rdinp");
        assert!(resolved
            .modules
            .iter()
            .skip(1)
            .all(|module| module.executable.starts_with(&fallback_dir.path)));
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_errors_for_missing_module() {
        let _lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let previous_path = env::var_os("PATH");

        let primary_dir = TestDir::new("xfeff-primary-missing");
        fs::create_dir_all(&primary_dir.path).unwrap();
        write_exec(
            &primary_dir.join("feff8l_rdinp"),
            "if [ ! -f feff.inp ]; then :; fi\necho only rdinp",
        );

        env::set_var("PATH", "");

        let request = build_request(primary_dir.join("feff8l_rdinp"), primary_dir.path.clone());
        let err = resolve_feff_commands(&request).unwrap_err();

        if let Some(path) = previous_path {
            env::set_var("PATH", path);
        } else {
            env::remove_var("PATH");
        }

        assert!(matches!(err, FittingError::ExecutableNotFound { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn test_runner_success_discovers_sorted_outputs() {
        let workspace = TestDir::new("xfeff-work-success");
        fs::write(workspace.join("feff.inp"), "TITLE\n").unwrap();

        let bin_dir = TestDir::new("xfeff-bin-success");
        install_mock_modules(
            &bin_dir.path,
            "echo 'dummy' > feff0010.dat\necho 'dummy' > feff0002.dat",
            None,
        );

        let request = build_request(bin_dir.join("feff8l_rdinp"), workspace.path.clone());
        let result = run_feff(&request).unwrap();

        assert_eq!(result.logs.len(), FEFF85L_MODULES.len());
        assert_eq!(result.path_files.len(), 2);

        let names = result
            .path_files
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["feff0002.dat", "feff0010.dat"]);
    }

    #[cfg(unix)]
    #[test]
    fn test_runner_missing_feffinp_fails() {
        let workspace = TestDir::new("xfeff-work-missing-inp");
        let bin_dir = TestDir::new("xfeff-bin-missing-inp");
        install_mock_modules(&bin_dir.path, "", None);

        let request = build_request(bin_dir.join("feff8l_rdinp"), workspace.path.clone());
        let err = run_feff(&request).unwrap_err();

        assert!(matches!(err, FittingError::FeffInputNotFound { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn test_runner_nonzero_exit_fails_fast() {
        let workspace = TestDir::new("xfeff-work-fail");
        fs::write(workspace.join("feff.inp"), "TITLE\n").unwrap();

        let bin_dir = TestDir::new("xfeff-bin-fail");
        install_mock_modules(&bin_dir.path, "", Some("pot"));

        let request = build_request(bin_dir.join("feff8l_rdinp"), workspace.path.clone());
        let err = run_feff(&request).unwrap_err();

        assert!(matches!(
            err,
            FittingError::ProcessFailed {
                module,
                code: 9
            } if module == "pot"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_runner_empty_output_set_errors() {
        let workspace = TestDir::new("xfeff-work-empty");
        fs::write(workspace.join("feff.inp"), "TITLE\n").unwrap();

        let bin_dir = TestDir::new("xfeff-bin-empty");
        install_mock_modules(&bin_dir.path, "", None);

        let request = build_request(bin_dir.join("feff8l_rdinp"), workspace.path.clone());
        let err = run_feff(&request).unwrap_err();

        assert!(matches!(err, FittingError::NoPathOutputs { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_and_load_paths_interoperates_with_modeling() {
        let workspace = TestDir::new("xfeff-work-load");
        fs::write(workspace.join("feff.inp"), "TITLE\n").unwrap();

        let fixture1 = format!("{TOP_DIR}/tests/testfiles/feffcu01.dat");
        let fixture2 = format!("{TOP_DIR}/tests/testfiles/feff0002.dat");

        let bin_dir = TestDir::new("xfeff-bin-load");
        let ff2x_payload = format!(
            "cat '{}' > feff0010.dat\ncat '{}' > feff0002.dat",
            fixture1, fixture2
        );
        install_mock_modules(&bin_dir.path, &ff2x_payload, None);

        let request = build_request(bin_dir.join("feff8l_rdinp"), workspace.path.clone());
        let paths = run_feff_and_load_paths(&request, FeffFlavor::Feff85L).unwrap();

        assert_eq!(paths.len(), 2);

        let k = DVector::from_iterator(120, (0..120).map(|i| 0.05 * (i as f64 + 1.0)));
        let modeled = ff2chi(&paths, &FitVariables::new(), &k).unwrap();

        assert_eq!(modeled.chi.len(), k.len());
        assert_eq!(modeled.path_chi.len(), 2);
    }
}

#[cfg(test)]
mod feff10_input_tests {
    use super::{parse_print_flags, prepare_feff10_input};

    const INPUT: &str = "TITLE Cu\nHOLE 1 1.0\nCONTROL 1 1 1 1 1 1\nPRINT 1 0 0 0 0 0 * comment\nRMAX 5.0\n\nPOTENTIALS\n 0 29 Cu\nATOMS\n 0.0 0.0 0.0 0 Cu\nEND\n";

    #[test]
    fn raises_the_path_output_flag_and_keeps_other_flags() {
        let prepared = prepare_feff10_input(INPUT, false);
        assert!(prepared.contains("PRINT 1 0 0 0 0 3\n"));
        assert!(!prepared.contains("SFCONV"));
        assert!(prepared.contains("HOLE 1 1.0\n"));
        assert!(prepared.ends_with("END\n"));
        assert_eq!(parse_print_flags("PRINT 1 0 0 0 0 3"), [1, 0, 0, 0, 0, 3]);
        assert_eq!(parse_print_flags("  print 2"), [2, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn keeps_a_larger_existing_flag_and_adds_a_missing_card_before_potentials() {
        let prepared = prepare_feff10_input("PRINT 0 0 0 0 0 5\nATOMS\nEND\n", true);
        assert!(prepared.starts_with("PRINT 0 0 0 0 0 5\nSFCONV\nATOMS\n"));

        let without_print = prepare_feff10_input(
            "TITLE x\n* PRINT in a comment\nPOTENTIALS\n 0 29 Cu\nEND\n",
            false,
        );
        assert_eq!(
            without_print,
            "TITLE x\n* PRINT in a comment\nPRINT 0 0 0 0 0 3\nPOTENTIALS\n 0 29 Cu\nEND\n"
        );
    }

    #[test]
    fn preserves_an_existing_sfconv_card() {
        let prepared = prepare_feff10_input("sfconv\nPRINT 0 0 0 0 0 3\n", true);
        assert_eq!(
            prepared.matches("sfconv").count() + prepared.matches("SFCONV").count(),
            1
        );
    }
}
