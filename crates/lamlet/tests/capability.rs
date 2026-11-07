use lamlet::{
    Language, Options,
    capability::{self, Availability, Profile},
    compile_with_options,
    resources::Project,
    source::Source,
};
use std::fs;
fn profile() -> Profile {
    capability::parse_help(
        include_str!("../../../tests/fixtures/lammps-help.txt"),
        "/target/lmp".into(),
    )
    .unwrap()
}
fn check(text: &str, profile: Option<Profile>) -> lamlet::Compilation {
    compile_with_options(
        &Source::new("target.lamlet", text.as_bytes().to_vec()),
        &Options {
            language: Language::Lamlet,
            capabilities: profile,
            strict: true,
            ..Options::default()
        },
    )
}
#[test]
fn official_help_format_is_sorted_deduplicated_and_missing_sections_fail() {
    let profile = profile();
    assert_eq!(profile.lammps_version, "2025-07-22");
    assert_eq!(
        profile
            .packages
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["MANYBODY", "MOLECULE"]
    );
    assert_eq!(
        profile.availability("fix", "nvt", false),
        Availability::BuiltIn
    );
    assert_eq!(
        profile.availability("pair", "missing", false),
        Availability::Unavailable
    );
    assert_eq!(
        profile.availability("future-category", "missing", false),
        Availability::Unknown
    );
    assert!(capability::parse_help("not LAMMPS help", "lmp".into()).is_err());
    let truncated = include_str!("../../../tests/fixtures/lammps-help.txt")
        .split("* Dump styles:")
        .next()
        .unwrap();
    assert!(capability::parse_help(truncated, "lmp".into()).is_err());
}
#[test]
fn style_availability_is_independent_from_packages_and_schema_support() {
    let mut p = profile();
    p.styles.get_mut("pair").unwrap().remove("eam/alloy");
    assert!(p.packages.contains("MANYBODY"));
    let c = check("pair_style eam/alloy\n", Some(p));
    let d = c
        .diagnostics
        .iter()
        .find(|d| d.code == "LML-CAP-001")
        .unwrap();
    assert!(d.notes.iter().any(|n| n.contains("MANYBODY")));
    assert!(
        d.suggestion
            .as_deref()
            .unwrap()
            .contains("PKG_MANYBODY=yes")
    );
    assert!(c.output.is_empty());
    let mut p = profile();
    p.styles
        .get_mut("fix")
        .unwrap()
        .insert("custom/opaque".into());
    let text = "fix f all custom/opaque anything\n";
    let c = check(text, Some(p));
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(c.output, text.as_bytes());
}
#[test]
fn no_target_never_rejects_legacy_style_names() {
    let text = "pair_style custom/potential\nfix f all plugin/style anything\n";
    let c = check(text, None);
    assert!(!c.has_errors());
    assert_eq!(c.output, text.as_bytes());
    assert!(
        c.requirements
            .iter()
            .all(|r| r.availability == Availability::Unknown)
    );
}
#[test]
fn plugin_sources_runtime_loads_and_opaque_control_flow_produce_unknown() {
    let mut p = profile();
    p.plugins
        .entry("pair".into())
        .or_default()
        .insert("plugin/style".into());
    assert_eq!(
        p.availability("pair", "plugin/style", false),
        Availability::Plugin
    );
    p.plugin_path = Some("/plugins".into());
    assert_eq!(
        p.availability("pair", "missing", false),
        Availability::Unknown
    );
    assert!(!check("pair_style missing\n", Some(p)).has_errors());
    for prefix in [
        "plugin load custom.so",
        "include setup.in",
        "if '1' then 'plugin load custom.so'",
        "jump SELF setup",
    ] {
        let c = check(
            &format!("{prefix}\npair_style custom/unknown\n"),
            Some(profile()),
        );
        assert!(!c.has_errors(), "{:?}", c.diagnostics);
        assert!(
            c.requirements
                .iter()
                .any(|r| r.style.as_deref() == Some("custom/unknown")
                    && r.availability == Availability::Unknown)
        );
    }
    let c = check("pair_style ${runtime}\n", Some(profile()));
    assert!(!c.has_errors());
    assert!(c.requirements.iter().any(|r| r.style.is_none()));
}
#[test]
fn known_hybrid_children_are_checked_and_unknown_grammars_are_not_guessed() {
    let c = check("pair_style hybrid eam/alloy lj/cut 2.5\n", Some(profile()));
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    for style in ["hybrid", "eam/alloy", "lj/cut"] {
        assert!(
            c.requirements
                .iter()
                .any(|r| r.style.as_deref() == Some(style)
                    && r.availability == Availability::BuiltIn)
        );
    }
    let c = check(
        "pair_style hybrid/scaled 0.5 eam/alloy 0.5 lj/cut 2.5\n",
        Some(profile()),
    );
    assert!(!c.has_errors());
    let mut p = profile();
    p.styles.get_mut("pair").unwrap().remove("eam/alloy");
    assert!(check("pair_style hybrid eam/alloy lj/cut 2.5\n", Some(p)).has_errors());
    let mut p = profile();
    p.styles
        .get_mut("pair")
        .unwrap()
        .insert("custom/opaque".into());
    let c = check(
        "pair_style hybrid custom/opaque argument another\n",
        Some(p),
    );
    assert!(!c.has_errors());
    assert!(c.requirements.iter().any(|r| r.style.is_none()));
    assert!(
        !c.requirements
            .iter()
            .any(|r| r.style.as_deref() == Some("argument"))
    );
    assert!(check("bond_style hybrid harmonic missing\n", Some(profile())).has_errors());
}
#[test]
fn version_requirements_use_calendar_dates_and_reject_unsupported_ranges() {
    assert!(capability::version_matches("2025-09-10", ">=2025-09-10, <2026-01-01").unwrap());
    assert!(!capability::version_matches("2025-07-22", ">=2025-09-10").unwrap());
    for invalid in [
        "^2025-09-10",
        "2025-02-29",
        ">=2025-13-01",
        "2025-09-10 || 2026-01-01",
        "*",
    ] {
        assert!(
            capability::validate_version_constraint(invalid).is_err(),
            "{invalid}"
        );
    }
}
#[test]
fn configuration_requirements_are_checked_before_simulation() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("lamlet.toml"), "[lammps]\nexecutable='/target/lmp'\n[lammps.requirements]\nversion='>=2025-09-10'\npackages=['RIGID']\n[validation]\nstrict=true\ncheck_capabilities=true\n").unwrap();
    let project = Project::load(dir.path()).unwrap();
    assert!(project.validation.strict);
    let c = compile_with_options(
        &Source::new("script.in", b"run 0\n".to_vec()),
        &Options {
            project: Some(project),
            capabilities: Some(profile()),
            ..Options::default()
        },
    );
    assert!(c.diagnostics.iter().any(|d| d.code == "LML-CAP-003"));
    assert!(c.diagnostics.iter().any(|d| d.code == "LML-CAP-004"));
    assert!(c.output.is_empty());
}
#[test]
fn managed_declarations_and_static_loop_requirements_use_expanded_commands() {
    let c = check(
        "@for i in [0,1] {\n@scope {\n@fix f all nve\nrun @{i}\n}\n}\n",
        Some(profile()),
    );
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(
        c.requirements
            .iter()
            .filter(|r| r.category == "fix" && r.style.as_deref() == Some("nve"))
            .count(),
        2
    );
    let mut p = profile();
    p.schema_version = 2;
    assert!(
        check("run 0\n", Some(p))
            .diagnostics
            .iter()
            .any(|d| d.code == "E_TARGET_PROFILE")
    );
}
