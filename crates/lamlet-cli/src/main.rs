use clap::{Parser, Subcommand, ValueEnum};
use lamlet::{Language, Options, compile_with_options, source::Source};
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

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

fn run(cli: Cli) -> io::Result<u8> {
    let (input, output, language, format, strict, max_expanded_lines) = match cli.command {
        Command::Check {
            input,
            language,
            format,
            strict,
            max_expanded_lines,
        } => (input, None, language, format, strict, max_expanded_lines),
        Command::Build {
            input,
            output,
            language,
            format,
            strict,
            max_expanded_lines,
        } => (
            input,
            Some(output),
            language,
            format,
            strict,
            max_expanded_lines,
        ),
    };
    let language = match language {
        Some(LanguageArg::Lammps) => Language::Lammps,
        Some(LanguageArg::Lamlet) => Language::Lamlet,
        None if input.extension().is_some_and(|ext| ext == "lamlet") => Language::Lamlet,
        None => Language::Lammps,
    };
    let source = Source::new(input.to_string_lossy(), fs::read(&input)?);
    let compilation = compile_with_options(
        &source,
        &Options {
            language,
            strict,
            max_expanded_lines,
        },
    );
    let report = compilation.report(&source);
    match format {
        Format::Json => println!("{}", serde_json::to_string(&report)?),
        Format::Human => {
            for d in &compilation.diagnostics {
                eprintln!(
                    "{}:{}:{}: {:?} [{}]: {}",
                    d.file, d.span.start_line, d.span.start_column, d.severity, d.code, d.message
                );
            }
            eprintln!(
                "{}: {} ({} errors, {} warnings, {} unknowns)",
                source.path,
                report.status,
                report.summary.errors,
                report.summary.warnings,
                report.summary.unknowns
            );
        }
    }
    if compilation.has_errors() {
        return Ok(1);
    }
    if let Some(path) = output {
        atomic_write(&path, &compilation.output)?;
    }
    Ok(0)
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("lamlet: {error}");
            ExitCode::from(2)
        }
    }
}
