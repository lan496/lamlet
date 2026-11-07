//! Typed filesystem references and project-relative lowering. Legacy text is never resolved.
use crate::{
    expr::ExprError,
    source::{LocatedSpan, Source, Span},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResourceKind {
    InputFile,
    OutputFile,
    Directory,
    InputScript,
    PotentialFile,
    InputPattern,
    DynamicPath,
}
impl ResourceKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::InputFile => "InputFile",
            Self::OutputFile => "OutputFile",
            Self::Directory => "Directory",
            Self::InputScript => "InputScript",
            Self::PotentialFile => "PotentialFile",
            Self::InputPattern => "InputPattern",
            Self::DynamicPath => "DynamicPath",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        [
            Self::InputFile,
            Self::OutputFile,
            Self::Directory,
            Self::InputScript,
            Self::PotentialFile,
            Self::InputPattern,
            Self::DynamicPath,
        ]
        .into_iter()
        .find(|k| k.name() == name)
    }
    pub fn is_input_file(self) -> bool {
        matches!(
            self,
            Self::InputFile | Self::InputScript | Self::PotentialFile
        )
    }
    pub fn accepts(self, other: Self) -> bool {
        self == other || self == Self::InputFile && other.is_input_file()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceValue {
    pub kind: ResourceKind,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub input_root: PathBuf,
    pub output_root: PathBuf,
    pub build_directory: PathBuf,
    pub name: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    project: ProjectConfig,
    paths: PathConfig,
    build: BuildConfig,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ProjectConfig {
    name: Option<String>,
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PathConfig {
    input_root: String,
    output_root: String,
}
impl Default for PathConfig {
    fn default() -> Self {
        Self {
            input_root: ".".into(),
            output_root: ".".into(),
        }
    }
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct BuildConfig {
    directory: String,
}
impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            directory: ".lamlet/build".into(),
        }
    }
}

fn failure(code: &'static str, message: impl Into<String>, span: Span) -> ExprError {
    ExprError::new(code, message, span)
}

fn lexical(root: &Path, text: &str) -> Result<PathBuf, String> {
    if text.is_empty()
        || text.contains(['\\', ':', '"', '\'', '\0', '\n', '\r'])
        || text.chars().any(char::is_control)
    {
        return Err("Paths require nonempty portable names without quotes, backslashes, colons, or control characters".into());
    }
    let mut path = root.to_path_buf();
    for part in Path::new(text).components() {
        match part {
            Component::Normal(p) => path.push(p),
            Component::CurDir => {}
            Component::ParentDir if path != root => {
                path.pop();
            }
            _ => return Err("Path escapes the project root or is absolute".into()),
        }
    }
    Ok(path)
}

/// Canonicalize even when a file or its parent directories do not yet exist.
pub fn resolved_destination(path: &Path) -> std::io::Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(_) => fs::canonicalize(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(e)?;
            Ok(resolved_destination(parent)?.join(
                path.file_name()
                    .ok_or_else(|| std::io::Error::other("Expected a file name"))?,
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn relative(from: &Path, to: &Path) -> PathBuf {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut path = PathBuf::new();
    for _ in &a[common..] {
        path.push("..");
    }
    for part in &b[common..] {
        path.push(part.as_os_str());
    }
    if path.as_os_str().is_empty() {
        path.push(".");
    }
    path
}
pub fn portable(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn aliases(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    #[cfg(unix)]
    if let (Ok(a), Ok(b)) = (fs::metadata(a), fs::metadata(b)) {
        use std::os::unix::fs::MetadataExt;
        return a.dev() == b.dev() && a.ino() == b.ino();
    }
    false
}

impl Project {
    pub fn discover(input: &Path) -> Result<Self, String> {
        let input = fs::canonicalize(input).map_err(|e| e.to_string())?;
        let parent = input.parent().ok_or("Input has no parent directory")?;
        let root = parent
            .ancestors()
            .find(|p| p.join("lamlet.toml").is_file())
            .unwrap_or(parent);
        Self::load(root)
    }
    pub fn load(root: &Path) -> Result<Self, String> {
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let config = match fs::read_to_string(root.join("lamlet.toml")) {
            Ok(text) => toml::from_str::<Config>(&text).map_err(|e| e.to_string())?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(e.to_string()),
        };
        let resolve = |text: &str| -> Result<PathBuf, String> {
            if text.contains(['$', '*', '%', '#']) {
                return Err("Project directory settings must be static paths".into());
            }
            let path = resolved_destination(&lexical(&root, text)?).map_err(|e| e.to_string())?;
            if !path.starts_with(&root) {
                return Err("Project directories must remain inside the project root".into());
            }
            Ok(path)
        };
        let input_root = resolve(&config.paths.input_root)?;
        if !input_root.is_dir() {
            return Err("paths.input_root must be an existing directory".into());
        }
        let output_root = resolve(&config.paths.output_root)?;
        let build_directory = resolve(&config.build.directory)?;
        for path in [&output_root, &build_directory] {
            if path.exists() && !path.is_dir() {
                return Err("Output and build roots must name directories".into());
            }
        }
        Ok(Self {
            root,
            input_root,
            output_root,
            build_directory,
            name: config.project.name,
        })
    }
    pub fn resolve(&self, value: &ResourceValue, span: Span) -> Result<PathBuf, ExprError> {
        let dynamic = value.kind == ResourceKind::DynamicPath;
        let patterned = matches!(
            value.kind,
            ResourceKind::InputPattern | ResourceKind::OutputFile
        );
        if (!dynamic && value.path.contains('$'))
            || (!patterned && !dynamic && value.path.contains(['*', '%']))
        {
            return Err(failure(
                "E_PATH_SYNTAX",
                "Use pattern() for restart patterns or dynamic() for runtime names",
                span,
            ));
        }
        let lexical =
            lexical(&self.root, &value.path).map_err(|e| failure("E_PATH_ESCAPE", e, span))?;
        let parent = lexical
            .parent()
            .ok_or_else(|| failure("E_PATH_KIND", "Expected a file path", span))?;
        if (patterned || dynamic)
            && portable(&relative(&self.root, parent)).contains(['*', '%', '$'])
        {
            return Err(failure(
                "E_PATH_SYNTAX",
                "Patterns and runtime substitutions are supported only in the basename",
                span,
            ));
        }
        let resolved = resolved_destination(&lexical).map_err(|e| {
            failure(
                "E_PATH_IO",
                format!("Cannot resolve '{}': {e}", value.path),
                span,
            )
        })?;
        let allowed = match value.kind {
            ResourceKind::OutputFile => &self.output_root,
            ResourceKind::Directory => &self.root,
            _ => &self.input_root,
        };
        // DynamicPath does not assert input/output direction. Its fixed prefix is
        // confined to the project; runtime substitution itself remains unverified.
        if !resolved.starts_with(&self.root) || (!dynamic && !resolved.starts_with(allowed)) {
            return Err(failure(
                "E_PATH_ESCAPE",
                format!("'{}' escapes its configured root", value.path),
                span,
            ));
        }
        match value.kind {
            kind if kind.is_input_file() => {
                if !resolved.exists() {
                    return Err(failure(
                        "E_PATH_NOT_FOUND",
                        format!("Input '{}' does not exist", value.path),
                        span,
                    ));
                }
                if !resolved.is_file() {
                    return Err(failure(
                        "E_PATH_KIND",
                        format!("Input '{}' is not a regular file", value.path),
                        span,
                    ));
                }
            }
            ResourceKind::Directory => {
                if !resolved.is_dir() {
                    return Err(failure(
                        "E_PATH_KIND",
                        format!("'{}' is not an existing directory", value.path),
                        span,
                    ));
                }
            }
            ResourceKind::OutputFile => {
                if resolved.exists() && !resolved.is_file() {
                    return Err(failure("E_PATH_KIND", "Output is not a regular file", span));
                }
            }
            ResourceKind::InputPattern if !parent.is_dir() => {
                return Err(failure(
                    "E_PATH_KIND",
                    "Input pattern requires an existing parent directory",
                    span,
                ));
            }
            _ => {}
        }
        Ok(resolved)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Dependency {
    pub path: String,
    pub kind: String,
    pub resource_type: ResourceKind,
    pub exists: Option<bool>,
    pub uses: Vec<LocatedSpan>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Unresolved {
    pub command: String,
    pub value: Option<String>,
    pub span: LocatedSpan,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Dependencies {
    pub schema_version: u32,
    pub working_directory: &'static str,
    pub complete: bool,
    pub coverage: &'static str,
    pub inputs: Vec<Dependency>,
    pub outputs: Vec<Dependency>,
    pub directories: Vec<Dependency>,
    pub dynamic: Vec<Dependency>,
    pub unresolved: Vec<Unresolved>,
    #[serde(skip)]
    pub typed: bool,
}
impl Default for Dependencies {
    fn default() -> Self {
        Self {
            schema_version: 1,
            working_directory: ".",
            complete: false,
            coverage: "explicit typed references; legacy and transitive dependencies are not verified",
            inputs: vec![],
            outputs: vec![],
            directories: vec![],
            dynamic: vec![],
            unresolved: vec![],
            typed: false,
        }
    }
}

pub(crate) struct Resolver {
    pub dependencies: Dependencies,
    project: Option<Project>,
    cwd: Option<PathBuf>,
    writers: Vec<PathBuf>,
    protected: Vec<PathBuf>,
}
impl Resolver {
    pub fn new(project: Option<Project>, source: &Source) -> Self {
        let cwd = project.as_ref().map(|p| p.root.clone());
        let mut protected: Vec<_> = fs::canonicalize(&source.path).ok().into_iter().collect();
        if let Some(config) = project
            .as_ref()
            .and_then(|p| fs::canonicalize(p.root.join("lamlet.toml")).ok())
        {
            protected.push(config);
        }
        Self {
            dependencies: Dependencies::default(),
            project,
            cwd,
            writers: vec![],
            protected,
        }
    }
    pub fn declare(
        &mut self,
        value: &ResourceValue,
        source: &Source,
        span: Span,
        role: &str,
        writer: bool,
    ) -> Result<PathBuf, ExprError> {
        let project = self.project.as_ref().ok_or_else(|| {
            failure(
                "E_PROJECT_REQUIRED",
                "Typed paths require an explicit project context",
                span,
            )
        })?;
        let path = project.resolve(value, span)?;
        self.dependencies.typed = true;
        if value.kind.is_input_file() {
            if self
                .dependencies
                .outputs
                .iter()
                .any(|p| aliases(&project.root.join(&p.path), &path))
            {
                return Err(failure(
                    "E_OUTPUT_COLLISION",
                    "Input aliases a typed output",
                    span,
                ));
            }
            self.protected.push(path.clone());
        }
        if value.kind == ResourceKind::OutputFile
            && (self.protected.iter().any(|p| aliases(p, &path)))
        {
            return Err(failure(
                "E_OUTPUT_COLLISION",
                "Output aliases an input or the source script",
                span,
            ));
        }
        if writer {
            if self.writers.iter().any(|p| aliases(p, &path)) {
                return Err(failure(
                    "E_OUTPUT_COLLISION",
                    "Multiple commands write the same typed output",
                    span,
                ));
            }
            self.writers.push(path.clone());
        }
        let normalized = portable(&relative(&project.root, &path));
        let list = match value.kind {
            ResourceKind::OutputFile => &mut self.dependencies.outputs,
            ResourceKind::Directory => &mut self.dependencies.directories,
            ResourceKind::DynamicPath => &mut self.dependencies.dynamic,
            _ => &mut self.dependencies.inputs,
        };
        if let Some(existing) = list.iter_mut().find(|d| d.path == normalized) {
            if role != "file" {
                existing.kind = role.into();
            }
            existing.uses.push(source.located(span));
        } else {
            list.push(Dependency {
                path: normalized,
                kind: role.into(),
                resource_type: value.kind,
                exists: if value.kind == ResourceKind::InputPattern
                    || value.kind == ResourceKind::DynamicPath
                {
                    None
                } else {
                    Some(path.exists())
                },
                uses: vec![source.located(span)],
            });
            list.sort_by(|a, b| a.path.cmp(&b.path));
        }
        Ok(path)
    }
    pub fn render(&self, path: &Path, kind: ResourceKind, span: Span) -> Result<String, ExprError> {
        let cwd = self.cwd.as_ref().ok_or_else(|| {
            failure(
                "E_PATH_CONTEXT",
                "Working directory is unknown after runtime control flow or shell cd",
                span,
            )
        })?;
        let emitted = portable(&relative(cwd, path));
        if kind == ResourceKind::DynamicPath {
            if emitted.contains(|c: char| c.is_whitespace() || c == '#') {
                return Err(failure(
                    "E_PATH_SYNTAX",
                    "Dynamic paths cannot contain whitespace or #",
                    span,
                ));
            }
            Ok(emitted)
        } else {
            Ok(format!("\"{emitted}\""))
        }
    }
    pub fn after_command(
        &mut self,
        source: &Source,
        command: &crate::syntax::Command,
        slot: Option<&crate::schema::PathOperandSchema>,
    ) {
        let args = &command.arguments;
        let Some(first) = args.first() else {
            return;
        };
        if self.project.is_none() {
            return;
        }
        if let Some(slot) = slot {
            if let Some(arg) = args
                .get(slot.position)
                .filter(|a| a.interpolation.is_none())
            {
                self.dependencies.unresolved.push(Unresolved {
                    command: first.text.clone(),
                    value: Some(arg.text.clone()),
                    span: source.located(arg.span),
                    reason:
                        "Legacy path retains runtime working-directory and environment semantics"
                            .into(),
                });
            }
        }
        if first.text == "shell" && args.get(1).is_some_and(|a| a.text == "cd") {
            self.cwd = if command.unambiguous && args.len() == 3 {
                self.cwd.as_ref().and_then(|cwd| {
                    fs::canonicalize(cwd.join(&args[2].text)).ok().filter(|p| {
                        p.is_dir() && p.starts_with(&self.project.as_ref().unwrap().root)
                    })
                })
            } else {
                None
            };
        } else if matches!(first.text.as_str(), "include" | "jump" | "if" | "python")
            || first.text == "run" && args.iter().any(|a| a.text == "every")
        {
            self.cwd = None;
        }
    }
}
