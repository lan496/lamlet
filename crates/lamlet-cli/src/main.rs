use clap::{Args, Parser, Subcommand, ValueEnum};
use lamlet::{
    Language, Options, compile_with_options,
    diagnostic::{Diagnostic, Report, Severity},
    source::{Source, Span},
};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{ExitCode, Stdio},
};

#[derive(Parser)]
#[command(version, about = "Compile and conservatively check LAMMPS scripts")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check(InputOptions),
    Build {
        #[command(flatten)]
        common: InputOptions,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        source_map: Option<PathBuf>,
    },
    Run {
        #[command(flatten)]
        common: InputOptions,
        /// Executable to launch (or LAMLET_LAMMPS_BIN, then lmp).
        #[arg(long)]
        lammps_bin: Option<OsString>,
    },
    Inspect {
        #[command(subcommand)]
        query: Inspect,
    },
    Schema {
        query: Option<String>,
        #[arg(long, value_enum, default_value = "human")]
        format: Format,
    },
}
#[derive(Subcommand)]
enum Inspect {
    Dependencies(InputOptions),
}
#[derive(Args)]
struct InputOptions {
    input: PathBuf,
    #[arg(long, value_enum)]
    language: Option<LanguageArg>,
    #[arg(long, value_enum, default_value = "human")]
    format: Format,
    #[arg(long)]
    strict: bool,
    #[arg(long, default_value_t = 100_000)]
    max_expanded_lines: usize,
}

enum Action {
    Check,
    Build {
        output: Option<PathBuf>,
        map: Option<PathBuf>,
    },
    Inspect,
    Run {
        engine: OsString,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum LanguageArg {
    Lammps,
    Lamlet,
}
#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Json,
}

fn prepare_output(path: &Path, bytes: &[u8]) -> io::Result<tempfile::NamedTempFile> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    Ok(file)
}

fn write_json(value: &impl serde::Serialize) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)
}

fn emit_report(source: &Source, diagnostics: &[Diagnostic], format: Format) -> io::Result<()> {
    let report = Report::new(&source.path, diagnostics);
    match format {
        Format::Json => write_json(&report),
        Format::Human => {
            let mut stderr = io::stderr().lock();
            for d in diagnostics {
                writeln!(
                    stderr,
                    "{}:{}:{}: {:?} [{}]: {}",
                    d.file, d.span.start_line, d.span.start_column, d.severity, d.code, d.message
                )?;
                for note in &d.notes {
                    writeln!(stderr, "  note: {note}")?;
                }
                if let Some(suggestion) = &d.suggestion {
                    writeln!(stderr, "  suggestion: {suggestion}")?;
                }
            }
            writeln!(
                stderr,
                "{}: {} ({} errors, {} warnings, {} unknowns)",
                source.path,
                report.status,
                report.summary.errors,
                report.summary.warnings,
                report.summary.unknowns
            )
        }
    }
}

fn schema(query: Option<&str>, format: Format) -> io::Result<u8> {
    let commands: Vec<_> = lamlet::schema::SCHEMAS
        .iter()
        .filter(|s| query.is_none_or(|q| s.name == q))
        .collect();
    let codes: Vec<_> = lamlet::diagnostic::CODES.iter().map(|(code, description)| serde_json::json!({ "code": code, "description": description, "severity": match code.as_bytes()[0] { b'E' => "error", b'W' => "warning", _ => "unknown" } })).collect();
    let value = serde_json::json!({
        "schema_version": 1,
        "compiler_version": env!("CARGO_PKG_VERSION"),
        "query": query,
        "coverage": if commands.is_empty() { "opaque" } else { "partial" },
        "coverage_meaning": "Commands outside this registry are accepted by Lamlet with unknown validation; their LAMMPS availability is not determined",
        "typed_unit_styles": ["metal", "real"],
        "accepted_legacy_unit_styles": ["lj", "real", "metal", "si", "cgs", "electron", "micro", "nano"],
        "explicit_units": ["fs", "ps", "ns", "s", "angstrom", "nm", "m", "K", "Pa", "bar", "atm", "eV", "kcal/mol"],
        "types": ["Int", "Float", "Bool", "Quantity(Dimension)", "List(Type)", "InputFile", "OutputFile", "Directory", "InputScript", "PotentialFile", "InputPattern", "DynamicPath", "Unknown"],
        "resource_builtins": ["path", "output", "dir", "pattern", "dynamic"],
        "managed_resource_syntax": ["@scope", "@fix", "@compute", "@dump", "@drop"],
        "commands": commands,
        "diagnostic_codes": codes,
        "defaults": { "language": "lammps, except .lamlet", "units": "lj", "max_expanded_lines": 100_000 },
        "spans": { "byte_offsets": "zero-based, half-open", "lines": "one-based", "columns": "one-based Unicode scalar values" },
        "validation_guarantee": "No detected errors is not proof of runtime or physical correctness",
        "lammps_target_version": null
    });
    match format {
        Format::Json => write_json(&value)?,
        Format::Human => {
            let mut stdout = io::stdout().lock();
            writeln!(
                stdout,
                "Lamlet v{}: partial static validation",
                env!("CARGO_PKG_VERSION")
            )?;
            for s in &commands {
                writeln!(
                    stdout,
                    "{} [{}]: {}. {}",
                    s.name, s.coverage, s.state_effect, s.limitations
                )?;
            }
            if commands.is_empty() {
                writeln!(
                    stdout,
                    "{}: opaque to Lamlet; LAMMPS support is not determined",
                    query.unwrap_or("command")
                )?;
            }
            writeln!(
                stdout,
                "Typed physical conversion: metal, real. Use --format json for full capabilities."
            )?;
        }
    }
    Ok(0)
}

fn destination(path: &Path) -> io::Result<PathBuf> {
    lamlet::resources::resolved_destination(&std::env::current_dir()?.join(path))
}

fn validate_destinations(targets: &[PathBuf], protected: &[PathBuf]) -> io::Result<()> {
    let targets = targets
        .iter()
        .map(|p| destination(p))
        .collect::<io::Result<Vec<_>>>()?;
    for (i, path) in targets.iter().enumerate() {
        if protected
            .iter()
            .chain(targets[..i].iter())
            .any(|p| lamlet::resources::aliases(p, path))
        {
            return Err(io::Error::other(
                "Build artifacts must not overwrite sources, dependencies, simulation outputs, or each other",
            ));
        }
    }
    Ok(())
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    name.into()
}

#[derive(serde::Serialize)]
struct BuildInfo {
    script: String,
    manifest: String,
    /// Relative to the directory containing the script and manifest.
    project_root: String,
    working_directory: &'static str,
}

fn write_outputs(
    input: &Path,
    output: &Path,
    map: Option<&Path>,
    source: &Source,
    compilation: &lamlet::Compilation,
    project: Option<&lamlet::resources::Project>,
    extra_targets: &[PathBuf],
) -> io::Result<Option<BuildInfo>> {
    use lamlet::resources::{portable, relative};
    let mut protected = vec![fs::canonicalize(input)?];
    if let Some(project) = project {
        protected.push(lamlet::resources::resolved_destination(
            &project.root.join("lamlet.toml"),
        )?);
        for dependency in compilation
            .resources
            .inputs
            .iter()
            .chain(&compilation.resources.outputs)
        {
            protected.push(project.root.join(&dependency.path));
        }
    }
    let mut payloads = vec![(output.to_path_buf(), compilation.output.clone())];
    if let Some(path) = map {
        let map = lamlet::codegen::SourceMap {
            format_version: 1,
            original_source: &source.path,
            output: &output.to_string_lossy(),
            lines: &compilation.mappings,
        };
        payloads.push((path.to_path_buf(), serde_json::to_vec_pretty(&map)?));
    }
    let info = if let Some(project) = project {
        let script = destination(output)?;
        let manifest = sidecar(output, ".manifest.json");
        let info = BuildInfo {
            script: portable(&relative(&project.root, &script)),
            manifest: portable(&relative(&project.root, &destination(&manifest)?)),
            project_root: portable(&relative(script.parent().unwrap(), &project.root)),
            working_directory: ".",
        };
        let value = serde_json::json!({
            "schema_version": 1, "compiler_version": env!("CARGO_PKG_VERSION"),
            "build": info, "dependencies": compilation.resources,
        });
        payloads.push((manifest, serde_json::to_vec_pretty(&value)?));
        Some(info)
    } else {
        None
    };
    let targets: Vec<_> = payloads
        .iter()
        .map(|(p, _)| p.clone())
        .chain(extra_targets.iter().cloned())
        .collect();
    validate_destinations(&targets, &protected)?;
    // Prepare every payload before replacing any existing artifact.
    let prepared = payloads
        .iter()
        .map(|(path, bytes)| Ok((path, prepare_output(path, bytes)?)))
        .collect::<io::Result<Vec<_>>>()?;
    for (path, file) in prepared {
        file.persist(path).map_err(|e| e.error)?;
    }
    Ok(info)
}

fn launch_engine(
    engine: &OsString,
    project: &lamlet::resources::Project,
    script: &Path,
    compilation: &lamlet::Compilation,
) -> io::Result<serde_json::Value> {
    use lamlet::resources::{portable, relative};
    for dependency in &compilation.resources.outputs {
        let target = project.root.join(&dependency.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
    }
    let stdout_path = sidecar(script, ".stdout");
    let stderr_path = sidecar(script, ".stderr");
    // A relative executable containing separators is relative to the caller,
    // while a bare executable is searched in PATH by the operating system.
    let executable = Path::new(engine);
    let executable = if executable.is_relative() && executable.components().count() > 1 {
        std::env::current_dir()?.join(executable).into_os_string()
    } else {
        engine.clone()
    };
    let script = destination(script)?;
    let log = sidecar(&script, ".log");
    let status = std::process::Command::new(executable)
        .arg("-in")
        .arg(relative(&project.root, &script))
        .arg("-log")
        .arg(relative(&project.root, &log))
        .args(["-screen", "none"])
        .current_dir(&project.root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(fs::File::create(&stdout_path)?))
        .stderr(Stdio::from(fs::File::create(&stderr_path)?))
        .status()?;
    Ok(serde_json::json!({
        "exit_code": status.code(), "success": status.success(), "working_directory": ".",
        "stdout": portable(&relative(&project.root, &destination(&stdout_path)?)),
        "stderr": portable(&relative(&project.root, &destination(&stderr_path)?)),
        "log": portable(&relative(&project.root, &log)),
    }))
}

fn run(cli: Cli) -> io::Result<u8> {
    let (common, action) = match cli.command {
        Command::Check(common) => (common, Action::Check),
        Command::Build {
            common,
            output,
            source_map,
        } => (
            common,
            Action::Build {
                output,
                map: source_map,
            },
        ),
        Command::Run { common, lammps_bin } => (
            common,
            Action::Run {
                engine: lammps_bin
                    .or_else(|| std::env::var_os("LAMLET_LAMMPS_BIN"))
                    .unwrap_or_else(|| "lmp".into()),
            },
        ),
        Command::Inspect {
            query: Inspect::Dependencies(common),
        } => (common, Action::Inspect),
        Command::Schema { query, format } => return schema(query.as_deref(), format),
    };
    let InputOptions {
        input,
        language,
        format,
        strict,
        max_expanded_lines,
    } = common;
    let language = match language {
        Some(LanguageArg::Lammps) => Language::Lammps,
        Some(LanguageArg::Lamlet) => Language::Lamlet,
        None if input.extension().is_some_and(|ext| ext == "lamlet") => Language::Lamlet,
        None => Language::Lammps,
    };
    let bytes = match fs::read(&input) {
        Ok(bytes) => bytes,
        Err(error) => {
            let source = Source::new(input.to_string_lossy(), vec![]);
            emit_report(
                &source,
                &[Diagnostic::new(
                    &source,
                    Span::default(),
                    Severity::Error,
                    "E_IO",
                    format!("Cannot read input: {error}"),
                )],
                format,
            )?;
            return Ok(2);
        }
    };
    let source = Source::new(input.to_string_lossy(), bytes);
    let needs_project = language == Language::Lamlet
        || matches!(
            action,
            Action::Inspect | Action::Run { .. } | Action::Build { output: None, .. }
        );
    let project = if needs_project {
        match lamlet::resources::Project::discover(&input) {
            Ok(project) => Some(project),
            Err(error) => {
                emit_report(
                    &source,
                    &[Diagnostic::new(
                        &source,
                        Span::default(),
                        Severity::Error,
                        "E_PROJECT_CONFIG",
                        error,
                    )],
                    format,
                )?;
                return Ok(2);
            }
        }
    } else {
        None
    };
    let mut compilation = compile_with_options(
        &source,
        &Options {
            language,
            strict,
            max_expanded_lines,
            project: project.clone(),
        },
    );
    let mut code = if compilation.has_errors() { 1 } else { 0 };
    let mut build = None;
    let mut execution = None;
    if code == 0 && matches!(action, Action::Build { .. } | Action::Run { .. }) {
        let (explicit_output, map) = match &action {
            Action::Build { output, map } => (output.clone(), map.as_deref()),
            _ => (None, None),
        };
        let default_output = explicit_output.is_none();
        let output = explicit_output.unwrap_or_else(|| {
            let stem = input.file_stem().unwrap_or_default();
            sidecar(&project.as_ref().unwrap().build_directory.join(stem), ".in")
        });
        let engine = if let Action::Run { engine } = &action {
            Some(engine)
        } else {
            None
        };
        let extra_targets: Vec<_> = if engine.is_some() {
            [".stdout", ".stderr", ".log"]
                .into_iter()
                .map(|s| sidecar(&output, s))
                .collect()
        } else {
            vec![]
        };
        let metadata_project = if default_output || compilation.resources.typed {
            project.as_ref()
        } else {
            None
        };
        let result = (|| -> io::Result<Option<BuildInfo>> {
            if default_output {
                fs::create_dir_all(&project.as_ref().unwrap().build_directory)?;
            }
            write_outputs(
                &input,
                &output,
                map,
                &source,
                &compilation,
                metadata_project,
                &extra_targets,
            )
        })();
        match result {
            Ok(info) => {
                build = info;
                if let Some(engine) = engine {
                    match launch_engine(engine, project.as_ref().unwrap(), &output, &compilation) {
                        Ok(result) => {
                            if result["success"] != true {
                                compilation.diagnostics.push(Diagnostic::new(
                                    &source,
                                    Span::default(),
                                    Severity::Error,
                                    "E_EXECUTION",
                                    "LAMMPS returned a failure status; see captured engine output",
                                ));
                                code = 3;
                            }
                            execution = Some(result);
                        }
                        Err(error) => {
                            compilation.diagnostics.push(Diagnostic::new(
                                &source,
                                Span::default(),
                                Severity::Error,
                                "E_EXECUTION",
                                format!("Cannot execute LAMMPS: {error}"),
                            ));
                            code = 3;
                        }
                    }
                }
            }
            Err(error) => {
                compilation.diagnostics.push(Diagnostic::new(
                    &source,
                    Span::default(),
                    Severity::Error,
                    "E_IO",
                    format!("Cannot write output: {error}"),
                ));
                code = 2;
            }
        }
    }
    if matches!(format, Format::Json) {
        let mut report = serde_json::to_value(compilation.report(&source))?;
        if matches!(action, Action::Inspect) {
            report["dependencies"] = serde_json::to_value(&compilation.resources)?;
        }
        if let Some(build) = &build {
            report["build"] = serde_json::to_value(build)?;
        }
        if let Some(execution) = execution {
            report["execution"] = execution;
        }
        write_json(&report)?;
    } else {
        emit_report(&source, &compilation.diagnostics, format)?;
        if matches!(action, Action::Inspect) {
            write_json(&compilation.resources)?;
        }
        if let Some(build) = build {
            eprintln!(
                "Built {}; run from the project root ({} relative to the script directory); manifest: {}",
                build.script, build.project_root, build.manifest
            );
        }
    }
    Ok(code)
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let args: Vec<_> = std::env::args_os().collect();
            let json = args
                .windows(2)
                .any(|a| a[0] == "--format" && a[1] == "json")
                || args.iter().any(|a| a == "--format=json");
            if error.use_stderr() && json {
                let source = Source::new("", vec![]);
                let d = Diagnostic::new(
                    &source,
                    Span::default(),
                    Severity::Error,
                    "E_CLI",
                    error.to_string(),
                );
                if let Err(e) = emit_report(&source, &[d], Format::Json) {
                    eprintln!("lamlet: {e}");
                }
                return ExitCode::from(2);
            }
            let code = error.exit_code();
            let _ = error.print();
            return ExitCode::from(code as u8);
        }
    };
    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("lamlet: {error}");
            ExitCode::from(2)
        }
    }
}
