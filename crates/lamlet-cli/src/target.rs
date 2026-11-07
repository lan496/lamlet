use super::{Format, InputOptions, LanguageArg, emit_report, prepare_output, write_json};
use clap::{Args, Subcommand};
use lamlet::{
    capability::{self, Availability, Fingerprint, Profile},
    diagnostic::{Diagnostic, Report, Severity},
    resources::{Project, portable},
    source::{Source, Span},
};
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Seek},
    path::{Path, PathBuf},
    process::{Command as Process, Stdio},
    time::{Duration, Instant, UNIX_EPOCH},
};

#[derive(Subcommand)]
pub(super) enum Command {
    Inspect {
        #[command(flatten)]
        common: TargetOptions,
        #[arg(long)]
        lammps_bin: Option<OsString>,
    },
    List {
        #[command(flatten)]
        common: TargetOptions,
        #[arg(long)]
        category: String,
    },
    Check {
        #[command(flatten)]
        common: TargetOptions,
        #[arg(long)]
        category: String,
        #[arg(long)]
        style: String,
    },
    Explain {
        #[command(flatten)]
        common: TargetOptions,
        #[arg(long)]
        category: String,
        #[arg(long)]
        style: String,
    },
    Requirements(InputOptions),
}
#[derive(Args)]
pub(super) struct TargetOptions {
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(long, value_enum, default_value = "human")]
    format: Format,
}
pub(super) fn profile_path(project: &Project) -> PathBuf {
    project.root.join(&project.lammps.profile)
}
pub(super) fn engine(project: &Project, explicit: Option<&OsString>) -> OsString {
    if let Some(explicit) = explicit {
        return explicit.clone();
    }
    if let Some(configured) = &project.lammps.executable {
        let path = Path::new(configured);
        return if path.is_relative() && path.components().count() > 1 {
            project.root.join(path).into_os_string()
        } else {
            configured.into()
        };
    }
    std::env::var_os("LAMLET_LAMMPS_BIN").unwrap_or_else(|| "lmp".into())
}
pub(super) fn executable(name: &OsStr) -> Result<PathBuf, String> {
    let name = Path::new(name);
    if name.is_absolute() || name.components().count() > 1 {
        return fs::canonicalize(name).map_err(|e| e.to_string());
    }
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return fs::canonicalize(candidate).map_err(|e| e.to_string());
        }
        #[cfg(windows)]
        {
            let candidate = candidate.with_extension("exe");
            if candidate.is_file() {
                return fs::canonicalize(candidate).map_err(|e| e.to_string());
            }
        }
    }
    Err(format!(
        "Executable '{}' was not found in PATH",
        name.display()
    ))
}
fn fingerprint(path: &Path) -> Result<Fingerprint, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    Ok(Fingerprint {
        bytes: meta.len(),
        modified_ns: meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|t| t.as_nanos()),
    })
}
fn plugin_path() -> Option<String> {
    std::env::var("LAMMPS_PLUGIN_PATH")
        .ok()
        .filter(|p| !p.is_empty())
}
pub(super) fn load(project: &Project) -> Result<Profile, String> {
    let bytes = fs::read(profile_path(project)).map_err(|e| {
        format!(
            "Cannot read target profile: {e}; run lamlet target inspect in the target environment"
        )
    })?;
    let profile: Profile =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid target profile: {e}"))?;
    profile.validate()?;
    let path = Path::new(&profile.executable);
    if path.is_file()
        && profile
            .fingerprint
            .as_ref()
            .is_some_and(|expected| fingerprint(path).as_ref() != Ok(expected))
    {
        return Err(
            "Target binary changed since inspection; run lamlet target inspect again".into(),
        );
    }
    if let Some(configured) = &project.lammps.executable {
        let selected = engine(project, None);
        if let Ok(selected) = executable(&selected) {
            if plugin_path() != profile.plugin_path {
                return Err("LAMMPS_PLUGIN_PATH changed since inspection; inspect this target environment again".into());
            }
            if selected != Path::new(&profile.executable) {
                return Err(format!(
                    "Configured executable '{configured}' differs from the saved target; inspect that target again"
                ));
            }
        }
    }
    Ok(profile)
}
pub(super) fn verify_runtime(profile: &Profile, selected: &OsStr) -> Result<(), String> {
    if executable(selected)? != Path::new(&profile.executable) {
        return Err(
            "Run executable differs from the checked target; inspect the selected target first"
                .into(),
        );
    }
    if plugin_path() != profile.plugin_path {
        return Err(
            "LAMMPS_PLUGIN_PATH differs from the checked target; inspect this environment first"
                .into(),
        );
    }
    Ok(())
}
fn probe(project: &Project, selected: &OsStr) -> Result<Profile, String> {
    let executable = executable(selected)?;
    let mut stdout = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut stderr = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut child = Process::new(&executable)
        .arg("-h")
        .current_dir(&project.root)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().map_err(|e| e.to_string())?)
        .stderr(stderr.try_clone().map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Capability probe did not finish within 10 seconds: {result:?}"
                ));
            }
        }
    };
    if !status.success() {
        stderr.rewind().map_err(|e| e.to_string())?;
        let mut detail = String::new();
        stderr
            .take(4096)
            .read_to_string(&mut detail)
            .map_err(|e| e.to_string())?;
        return Err(format!("LAMMPS -h failed ({status}): {detail}"));
    }
    if stdout.metadata().map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024 {
        return Err("LAMMPS help exceeds the 16 MiB probe limit".into());
    }
    stdout.rewind().map_err(|e| e.to_string())?;
    let mut help = String::new();
    stdout
        .read_to_string(&mut help)
        .map_err(|e| e.to_string())?;
    let mut profile = capability::parse_help(&help, portable(&executable))?;
    profile.fingerprint = Some(fingerprint(&executable)?);
    profile.plugin_path = plugin_path();
    Ok(profile)
}
fn save(project: &Project, profile: &Profile) -> Result<(), String> {
    let path = profile_path(project);
    if path.exists() {
        let existing: Profile =
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|_| "Refusing to replace a file that is not a target profile")?;
        existing.validate()?;
    }
    let resolved = lamlet::resources::resolved_destination(&path).map_err(|e| e.to_string())?;
    if !resolved.starts_with(&project.root)
        || lamlet::resources::aliases(&resolved, Path::new(&profile.executable))
        || lamlet::resources::aliases(&resolved, &project.root.join("lamlet.toml"))
    {
        return Err(
            "Profile must stay inside the project and not replace its configuration or executable"
                .into(),
        );
    }
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    prepare_output(
        &path,
        &serde_json::to_vec_pretty(profile).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?
    .persist(path)
    .map_err(|e| e.to_string())?;
    Ok(())
}
fn project(path: &Path) -> Result<Project, String> {
    let path = fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !path.is_dir() {
        return Err("--project requires a directory".into());
    }
    let root = path
        .ancestors()
        .find(|p| p.join("lamlet.toml").is_file())
        .unwrap_or(&path);
    Project::load(root)
}
fn fail(source: &Source, code: &str, error: String, format: Format) -> io::Result<u8> {
    emit_report(
        source,
        &[Diagnostic::new(
            source,
            Span::default(),
            Severity::Error,
            code,
            error,
        )],
        format,
    )?;
    Ok(2)
}
fn requirements(common: InputOptions) -> io::Result<u8> {
    let source = match fs::read(&common.input) {
        Ok(bytes) => Source::new(common.input.to_string_lossy(), bytes),
        Err(e) => {
            return fail(
                &Source::new(common.input.to_string_lossy(), vec![]),
                "E_IO",
                e.to_string(),
                common.format,
            );
        }
    };
    let project = match Project::discover(&common.input) {
        Ok(p) => p,
        Err(e) => return fail(&source, "E_PROJECT_CONFIG", e, common.format),
    };
    let capabilities = if profile_path(&project).exists() || project.validation.check_capabilities {
        match load(&project) {
            Ok(p) => Some(p),
            Err(e) => return fail(&source, "E_TARGET_PROFILE", e, common.format),
        }
    } else {
        None
    };
    let language = match common.language {
        Some(LanguageArg::Lamlet) => lamlet::Language::Lamlet,
        Some(LanguageArg::Lammps) => lamlet::Language::Lammps,
        None if common.input.extension().is_some_and(|e| e == "lamlet") => lamlet::Language::Lamlet,
        _ => lamlet::Language::Lammps,
    };
    let c = lamlet::compile_with_options(
        &source,
        &lamlet::Options {
            language,
            strict: common.strict || project.validation.strict,
            max_expanded_lines: common.max_expanded_lines,
            project: Some(project.clone()),
            capabilities,
            stage: match common
                .stage
                .as_deref()
                .map(|name| {
                    lamlet::stage::Invocation::parse(
                        name,
                        common.instance.as_deref(),
                        &common.arguments,
                    )
                })
                .transpose()
            {
                Ok(invocation) => invocation,
                Err(error) => {
                    return fail(&source, "E_STAGE_ARGUMENT", error.message, common.format);
                }
            },
        },
    );
    let mut value = serde_json::to_value(c.report(&source))?;
    value["requirements"] = serde_json::to_value(&c.requirements)?;
    value["project_requirements"] = serde_json::to_value(&project.lammps.requirements)?;
    value["coverage"] =
        "literal style operands and a limited hybrid grammar; dynamic effects remain unknown"
            .into();
    if matches!(common.format, Format::Human) {
        emit_report(&source, &c.diagnostics, common.format)?;
    }
    write_json(&value)?;
    Ok(u8::from(c.has_errors()))
}
pub(super) fn run(command: Command) -> io::Result<u8> {
    if let Command::Requirements(common) = command {
        return requirements(common);
    }
    let (common, selected, category, style, explain) = match command {
        Command::Inspect { common, lammps_bin } => (common, lammps_bin, None, None, false),
        Command::List { common, category } => (common, None, Some(category), None, false),
        Command::Check {
            common,
            category,
            style,
        } => (common, None, Some(category), Some(style), false),
        Command::Explain {
            common,
            category,
            style,
        } => (common, None, Some(category), Some(style), true),
        Command::Requirements(_) => unreachable!(),
    };
    let source = Source::new("lamlet.toml", vec![]);
    let project = match project(&common.project) {
        Ok(p) => p,
        Err(e) => return fail(&source, "E_PROJECT_CONFIG", e, common.format),
    };
    let profile = if category.is_none() {
        match probe(&project, &engine(&project, selected.as_ref())).and_then(|p| {
            save(&project, &p)?;
            Ok(p)
        }) {
            Ok(p) => p,
            Err(e) => return fail(&source, "E_TARGET_PROBE", e, common.format),
        }
    } else {
        match load(&project) {
            Ok(p) => p,
            Err(e) => return fail(&source, "E_TARGET_PROFILE", e, common.format),
        }
    };
    let mut diagnostics =
        capability::check_project(&source, &profile, &project.lammps.requirements);
    let mut data = serde_json::Map::new();
    if let (Some(category), Some(style)) = (&category, &style) {
        let availability = profile.availability(category, style, false);
        data.insert("category".into(), category.clone().into());
        data.insert("style".into(), style.clone().into());
        data.insert("availability".into(), serde_json::to_value(availability)?);
        let metadata = capability::metadata(category, style);
        if availability == Availability::Unavailable {
            let mut d = Diagnostic::new(
                &source,
                Span::default(),
                Severity::Error,
                "LML-CAP-001",
                format!("{category} style '{style}' is unavailable in the selected target"),
            );
            if let Some(package) = metadata.and_then(|m| m.required_package) {
                d.suggestion = Some(format!("-D PKG_{package}=yes"));
            }
            diagnostics.push(d);
        }
        if availability == Availability::Unknown {
            diagnostics.push(Diagnostic::new(
                &source,
                Span::default(),
                Severity::Unknown,
                "U_TARGET_CAPABILITY",
                "Target availability is unknown for this category or runtime plugin environment",
            ));
        }
        if explain {
            data.insert("metadata".into(), serde_json::to_value(metadata)?);
            data.insert(
                "cmake_option".into(),
                metadata
                    .and_then(|m| m.required_package)
                    .map(|p| format!("-D PKG_{p}=yes"))
                    .into(),
            );
            data.insert(
                "schema".into(),
                serde_json::to_value(
                    metadata
                        .and_then(|m| m.schema_query)
                        .and_then(lamlet::schema::find),
                )?,
            );
        }
    } else if let Some(category) = category {
        let mut names = profile.styles.get(&category).cloned().unwrap_or_default();
        names.extend(
            profile
                .plugins
                .get(&category)
                .into_iter()
                .flatten()
                .cloned(),
        );
        let styles: Vec<_> = names.iter().map(|s| serde_json::json!({ "style": s, "availability": profile.availability(&category, s, false) })).collect();
        data.insert("category".into(), category.clone().into());
        data.insert("styles".into(), styles.into());
        data.insert(
            "complete".into(),
            (profile.styles.contains_key(&category) && profile.plugin_path.is_none()).into(),
        );
    } else {
        data.insert("profile".into(), serde_json::to_value(&profile)?);
    }
    let report = Report::new(&source.path, &diagnostics);
    let code = u8::from(report.summary.errors > 0);
    let mut value = serde_json::to_value(report)?;
    let object = value.as_object_mut().unwrap();
    object.extend(data);
    object.insert("profile_path".into(), project.lammps.profile.into());
    if matches!(common.format, Format::Human) {
        emit_report(&source, &diagnostics, common.format)?;
    }
    write_json(&value)?;
    Ok(code)
}
