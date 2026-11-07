use clap::{Parser, Subcommand, ValueEnum};
use lamlet::{
    Language, Options, compile_with_options,
    diagnostic::{Diagnostic, Report, Severity},
    source::{Source, Span},
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(version, about = "Compile and conservatively check LAMMPS scripts")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check {
        input: PathBuf,
        #[arg(long, value_enum)]
        language: Option<LanguageArg>,
        #[arg(long, value_enum, default_value = "human")]
        format: Format,
        #[arg(long)]
        strict: bool,
        #[arg(long, default_value_t = 100_000)]
        max_expanded_lines: usize,
    },
    Build {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, value_enum)]
        language: Option<LanguageArg>,
        #[arg(long, value_enum, default_value = "human")]
        format: Format,
        #[arg(long)]
        strict: bool,
        #[arg(long, default_value_t = 100_000)]
        max_expanded_lines: usize,
        #[arg(long)]
        source_map: Option<PathBuf>,
    },
    Schema {
        query: Option<String>,
        #[arg(long, value_enum, default_value = "human")]
        format: Format,
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
        "types": ["Int", "Float", "Bool", "Quantity(Dimension)", "List(Type)", "Unknown"],
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
    if path.exists() {
        return fs::canonicalize(path);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(fs::canonicalize(parent)?.join(
        path.file_name()
            .ok_or_else(|| io::Error::other("Output must name a file"))?,
    ))
}

fn same_file(a: &Path, b: &Path) -> io::Result<bool> {
    if destination(a)? == destination(b)? {
        return Ok(true);
    }
    #[cfg(unix)]
    if a.exists() && b.exists() {
        use std::os::unix::fs::MetadataExt;
        let a = fs::metadata(a)?;
        let b = fs::metadata(b)?;
        return Ok(a.dev() == b.dev() && a.ino() == b.ino());
    }
    Ok(false)
}

fn write_outputs(
    input: &Path,
    output: &Path,
    map: Option<&Path>,
    source: &Source,
    compilation: &lamlet::Compilation,
) -> io::Result<()> {
    if same_file(input, output)? {
        return Err(io::Error::other("Output must not replace the input source"));
    }
    if let Some(map) = map {
        if same_file(input, map)? || same_file(output, map)? {
            return Err(io::Error::other(
                "Source map, output, and input must be distinct files",
            ));
        }
    }
    let generated = prepare_output(output, &compilation.output)?;
    let prepared_map = if let Some(path) = map {
        let map = lamlet::codegen::SourceMap {
            format_version: 1,
            original_source: &source.path,
            output: &output.to_string_lossy(),
            lines: &compilation.mappings,
        };
        Some((
            path,
            prepare_output(path, &serde_json::to_vec_pretty(&map)?)?,
        ))
    } else {
        None
    };
    if let Some((path, file)) = prepared_map {
        file.persist(path).map_err(|e| e.error)?;
    }
    generated.persist(output).map_err(|e| e.error)?;
    Ok(())
}

fn run(cli: Cli) -> io::Result<u8> {
    let (input, output, language, format, strict, max_expanded_lines, source_map) =
        match cli.command {
            Command::Check {
                input,
                language,
                format,
                strict,
                max_expanded_lines,
            } => (
                input,
                None,
                language,
                format,
                strict,
                max_expanded_lines,
                None,
            ),
            Command::Build {
                input,
                output,
                language,
                format,
                strict,
                max_expanded_lines,
                source_map,
            } => (
                input,
                Some(output),
                language,
                format,
                strict,
                max_expanded_lines,
                source_map,
            ),
            Command::Schema { query, format } => return schema(query.as_deref(), format),
        };
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
    let mut compilation = compile_with_options(
        &source,
        &Options {
            language,
            strict,
            max_expanded_lines,
        },
    );
    let mut code = if compilation.has_errors() { 1 } else { 0 };
    if code == 0 {
        if let Some(path) = output {
            if let Err(error) =
                write_outputs(&input, &path, source_map.as_deref(), &source, &compilation)
            {
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
    emit_report(&source, &compilation.diagnostics, format)?;
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
