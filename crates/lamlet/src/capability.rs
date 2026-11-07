//! An explicit target inventory, separate from command schemas and physical semantics.
use crate::{
    diagnostic::{Diagnostic, Severity},
    source::{LocatedSpan, Source},
    syntax::Command,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Validation {
    pub strict: bool,
    pub check_capabilities: bool,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectRequirements {
    pub version: Option<String>,
    pub packages: BTreeSet<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TargetConfig {
    pub executable: Option<String>,
    pub profile: String,
    pub requirements: ProjectRequirements,
}
impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            executable: None,
            profile: ".lamlet/target.json".into(),
            requirements: ProjectRequirements::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fingerprint {
    pub bytes: u64,
    pub modified_ns: Option<u128>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    pub lammps_version: String,
    pub version_text: String,
    pub executable: String,
    pub fingerprint: Option<Fingerprint>,
    pub platform: String,
    pub probe: String,
    pub plugin_path: Option<String>,
    pub packages: BTreeSet<String>,
    pub styles: BTreeMap<String, BTreeSet<String>>,
    /// Optional independently observed plugin inventory. -h collects built-ins only.
    #[serde(default)]
    pub plugins: BTreeMap<String, BTreeSet<String>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    BuiltIn,
    Plugin,
    Unavailable,
    Unknown,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported target profile schema_version".into());
        }
        date(&self.lammps_version)?;
        if self.executable.is_empty() || self.styles.is_empty() {
            return Err("Target profile needs an executable identity and style inventories".into());
        }
        if self
            .styles
            .iter()
            .chain(&self.plugins)
            .any(|(category, names)| {
                category.is_empty()
                    || names
                        .iter()
                        .any(|s| s.is_empty() || s.chars().any(char::is_whitespace))
            })
        {
            return Err(
                "Target inventories require nonempty categories and individual style names".into(),
            );
        }
        Ok(())
    }
    pub fn availability(&self, category: &str, style: &str, runtime_unknown: bool) -> Availability {
        if runtime_unknown {
            return Availability::Unknown;
        }
        if self.styles.get(category).is_some_and(|s| s.contains(style)) {
            return Availability::BuiltIn;
        }
        if self
            .plugins
            .get(category)
            .is_some_and(|s| s.contains(style))
        {
            return Availability::Plugin;
        }
        if self.plugin_path.as_ref().is_some_and(|p| !p.is_empty())
            || !self.styles.contains_key(category)
        {
            Availability::Unknown
        } else {
            Availability::Unavailable
        }
    }
}

fn date(value: &str) -> Result<String, String> {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err("Version dates require YYYY-MM-DD".into());
    }
    let number = |s: &str| {
        s.parse::<u32>()
            .map_err(|_| "Invalid version date".to_string())
    };
    let (year, month, day) = (number(parts[0])?, number(parts[1])?, number(parts[2])?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    if year == 0 || day == 0 || day > days {
        return Err("Invalid version date".into());
    }
    Ok(value.into())
}
fn conditions(text: &str) -> Result<Vec<(&str, String)>, String> {
    text.split(',')
        .map(|part| {
            let part = part.trim();
            let op = [">=", "<=", "==", ">", "<", "="]
                .into_iter()
                .find(|op| part.starts_with(op))
                .unwrap_or("=");
            let value = part.strip_prefix(op).unwrap_or(part).trim();
            Ok((op, date(value)?))
        })
        .collect()
}
pub fn validate_version_constraint(text: &str) -> Result<(), String> {
    conditions(text).map(|_| ())
}
pub fn version_matches(version: &str, requirement: &str) -> Result<bool, String> {
    let version = date(version)?;
    Ok(conditions(requirement)?
        .into_iter()
        .all(|(op, target)| match op {
            ">=" => version >= target,
            "<=" => version <= target,
            ">" => version > target,
            "<" => version < target,
            _ => version == target,
        }))
}

/// Parse the complete inventory from the official -h output, never a failed probe.
pub fn parse_help(help: &str, executable: String) -> Result<Profile, String> {
    let version_text = help
        .lines()
        .find(|s| s.starts_with("Large-scale Atomic/Molecular Massively Parallel Simulator - "))
        .ok_or("Unrecognized LAMMPS help header")?
        .to_string();
    let stamp = version_text
        .split(" - ")
        .nth(1)
        .ok_or("Missing LAMMPS version date")?;
    let parts: Vec<_> = stamp.split_whitespace().collect();
    if parts.len() != 3 {
        return Err("Unrecognized LAMMPS help version date".into());
    }
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|m| *m == parts[1])
    .ok_or("Unrecognized LAMMPS version month")?
        + 1;
    let year = parts[2]
        .parse::<u32>()
        .map_err(|_| "Invalid LAMMPS version year")?;
    let day = parts[0]
        .parse::<u32>()
        .map_err(|_| "Invalid LAMMPS version day")?;
    let version = date(&format!("{year:04}-{month:02}-{day:02}"))?;
    let mut packages = BTreeSet::new();
    let mut styles: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut in_packages = false;
    let mut saw_packages = false;
    let mut category = None;
    for line in help.lines().map(str::trim) {
        if line == "Installed packages:" {
            in_packages = true;
            saw_packages = true;
            continue;
        }
        if line.starts_with("List of individual style options") {
            in_packages = false;
            continue;
        }
        if let Some(header) = line.strip_prefix("* ") {
            in_packages = false;
            category = header
                .strip_suffix(':')
                .unwrap_or(header)
                .strip_suffix(" styles")
                .map(str::to_lowercase);
            if let Some(category) = &category {
                styles.entry(category.clone()).or_default();
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if in_packages {
            if !line.split_whitespace().all(|s| {
                s.bytes().all(|b| {
                    b.is_ascii_uppercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-')
                })
            }) {
                return Err("Unrecognized package inventory in LAMMPS help".into());
            }
            packages.extend(line.split_whitespace().map(str::to_owned));
        } else if let Some(category) = &category {
            if !line.split_whitespace().all(|s| {
                s.bytes().all(|b| {
                    b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_' | b'-' | b'+' | b'.')
                })
            }) {
                return Err("Unrecognized style inventory in LAMMPS help".into());
            }
            styles
                .get_mut(category)
                .unwrap()
                .extend(line.split_whitespace().map(str::to_owned));
        }
    }
    if !saw_packages
        || !["atom", "pair", "fix", "compute", "dump"]
            .iter()
            .all(|c| styles.contains_key(*c))
    {
        return Err("LAMMPS help inventory is incomplete".into());
    }
    let profile = Profile {
        schema_version: 1,
        lammps_version: version,
        version_text,
        executable,
        fingerprint: None,
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        probe: "lmp -h (compiled styles only)".into(),
        plugin_path: None,
        packages,
        styles,
        plugins: BTreeMap::new(),
    };
    profile.validate()?;
    Ok(profile)
}

#[derive(Debug, Serialize)]
pub struct StyleMetadata {
    pub category: &'static str,
    pub style: &'static str,
    pub required_package: Option<&'static str>,
    pub documentation: &'static str,
    pub schema_query: Option<&'static str>,
}
pub static METADATA: &[StyleMetadata] = &[
    StyleMetadata {
        category: "pair",
        style: "eam/alloy",
        required_package: Some("MANYBODY"),
        documentation: "https://docs.lammps.org/pair_eam.html",
        schema_query: Some("pair_coeff/eam/alloy"),
    },
    StyleMetadata {
        category: "pair",
        style: "eam",
        required_package: Some("MANYBODY"),
        documentation: "https://docs.lammps.org/pair_eam.html",
        schema_query: None,
    },
    StyleMetadata {
        category: "pair",
        style: "lj/cut",
        required_package: None,
        documentation: "https://docs.lammps.org/pair_lj.html",
        schema_query: None,
    },
    StyleMetadata {
        category: "fix",
        style: "nve",
        required_package: None,
        documentation: "https://docs.lammps.org/fix_nve.html",
        schema_query: Some("fix/nve"),
    },
    StyleMetadata {
        category: "fix",
        style: "nvt",
        required_package: None,
        documentation: "https://docs.lammps.org/fix_nh.html",
        schema_query: Some("fix/nvt"),
    },
    StyleMetadata {
        category: "fix",
        style: "npt",
        required_package: None,
        documentation: "https://docs.lammps.org/fix_nh.html",
        schema_query: Some("fix/npt"),
    },
];
pub fn metadata(category: &str, style: &str) -> Option<&'static StyleMetadata> {
    METADATA
        .iter()
        .find(|m| m.category == category && m.style == style)
}

#[derive(Clone, Debug, Serialize)]
pub struct Requirement {
    pub category: String,
    pub style: Option<String>,
    pub availability: Availability,
    pub span: LocatedSpan,
    pub reason: Option<String>,
}
#[derive(Default)]
pub(crate) struct Checker {
    pub requirements: Vec<Requirement>,
    profile: Option<Profile>,
    runtime_unknown: bool,
}
impl Checker {
    pub fn new(profile: Option<Profile>) -> Self {
        Self {
            profile,
            ..Self::default()
        }
    }
    fn record(
        &mut self,
        source: &Source,
        command: &Command,
        category: &str,
        style: Option<&str>,
        reason: Option<&str>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let availability = style
            .and_then(|s| {
                self.profile
                    .as_ref()
                    .map(|p| p.availability(category, s, self.runtime_unknown))
            })
            .unwrap_or(Availability::Unknown);
        if let (Some(style), Availability::Unavailable) = (style, availability) {
            let mut d = Diagnostic::new(
                source,
                command.line.span,
                Severity::Error,
                "LML-CAP-001",
                format!("{category} style '{style}' is unavailable in the selected LAMMPS target"),
            );
            if let Some(package) = metadata(category, style).and_then(|m| m.required_package) {
                d.notes.push(format!("Required package: {package}"));
                d.suggestion = Some(format!(
                    "Build LAMMPS with -D PKG_{package}=yes, or select another target"
                ));
            }
            diagnostics.push(d);
        }
        if self.profile.is_some() && availability == Availability::Unknown {
            diagnostics.push(Diagnostic::new(source, command.line.span, Severity::Unknown, "U_TARGET_CAPABILITY", reason.unwrap_or("Runtime plugins or incomplete target information prevent an availability proof")));
        }
        self.requirements.push(Requirement {
            category: category.into(),
            style: style.map(str::to_owned),
            availability,
            span: source.located(command.line.span),
            reason: reason.map(str::to_owned),
        });
    }
    pub fn command(
        &mut self,
        source: &Source,
        command: &Command,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let args = &command.arguments;
        let Some(first) = args.first() else {
            return;
        };
        let name = first.text.as_str();
        let (category, position) = match name {
            "pair_style" => ("pair", 1),
            "atom_style" => ("atom", 1),
            "bond_style" => ("bond", 1),
            "angle_style" => ("angle", 1),
            "dihedral_style" => ("dihedral", 1),
            "improper_style" => ("improper", 1),
            "kspace_style" => ("kspace", 1),
            "run_style" => ("integrate", 1),
            "min_style" => ("minimize", 1),
            "fix" => ("fix", 3),
            "compute" => ("compute", 3),
            "dump" => ("dump", 3),
            "region" => ("region", 2),
            _ => {
                if !command.unambiguous
                    || matches!(
                        name,
                        "plugin" | "include" | "jump" | "if" | "python" | "suffix" | "package"
                    )
                    || name == "run" && args.len() > 2
                {
                    self.runtime_unknown = true;
                    self.record(
                        source,
                        command,
                        "runtime",
                        None,
                        Some(
                            "Runtime commands may change availability or introduce unlisted styles",
                        ),
                        diagnostics,
                    );
                }
                return;
            }
        };
        if !command.unambiguous {
            self.runtime_unknown = true;
            self.record(
                source,
                command,
                category,
                None,
                Some("Runtime substitutions make the style position uncertain"),
                diagnostics,
            );
            return;
        }
        let Some(style) = args.get(position) else {
            return;
        };
        if style.text == "none"
            && matches!(
                category,
                "pair" | "bond" | "angle" | "dihedral" | "improper" | "kspace"
            )
        {
            return;
        }
        self.record(
            source,
            command,
            category,
            Some(&style.text),
            None,
            diagnostics,
        );
        if style.text.starts_with("hybrid") {
            if matches!(category, "bond" | "angle" | "dihedral" | "improper")
                && style.text == "hybrid"
            {
                for child in &args[position + 1..] {
                    self.record(
                        source,
                        command,
                        category,
                        Some(&child.text),
                        None,
                        diagnostics,
                    );
                }
            } else if category == "pair"
                && matches!(
                    style.text.as_str(),
                    "hybrid" | "hybrid/overlay" | "hybrid/scaled"
                )
            {
                let mut i = position + 1;
                while i < args.len() {
                    if style.text == "hybrid/scaled" {
                        i += 1;
                    }
                    let Some(child) = args.get(i) else {
                        break;
                    };
                    self.record(
                        source,
                        command,
                        category,
                        Some(&child.text),
                        None,
                        diagnostics,
                    );
                    i += 1;
                    match child.text.as_str() {
                        "eam" | "eam/alloy" => {}
                        "lj/cut" if args.get(i).is_some_and(|a| a.text.parse::<f64>().is_ok()) => {
                            i += 1;
                            if args.get(i).is_some_and(|a| a.text.parse::<f64>().is_ok()) {
                                i += 1;
                            }
                        }
                        _ => {
                            self.record(source, command, category, None, Some("Remaining hybrid substyles require unsupported argument schemas"), diagnostics);
                            break;
                        }
                    }
                }
            } else {
                self.record(
                    source,
                    command,
                    category,
                    None,
                    Some("Hybrid substyles require unsupported argument schemas"),
                    diagnostics,
                );
            }
        }
    }
}

pub fn check_project(
    source: &Source,
    profile: &Profile,
    required: &ProjectRequirements,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    if let Some(version) = &required.version {
        match version_matches(&profile.lammps_version, version) {
            Ok(true) => {}
            Ok(false) => diagnostics.push(Diagnostic::new(
                source,
                Default::default(),
                Severity::Error,
                "LML-CAP-003",
                format!(
                    "LAMMPS {} does not satisfy {version}",
                    profile.lammps_version
                ),
            )),
            Err(error) => diagnostics.push(Diagnostic::new(
                source,
                Default::default(),
                Severity::Error,
                "E_PROJECT_CONFIG",
                error,
            )),
        }
    }
    for package in &required.packages {
        if !profile.packages.contains(package) {
            diagnostics.push(Diagnostic::new(
                source,
                Default::default(),
                Severity::Error,
                "LML-CAP-004",
                format!("Required package '{package}' is absent from the selected target"),
            ));
        }
    }
    diagnostics
}
