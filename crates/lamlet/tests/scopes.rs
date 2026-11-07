use lamlet::{Compilation, Language, Options, compile_with_options, source::Source};
fn build(text: &str, strict: bool) -> Compilation {
    compile_with_options(
        &Source::new("scope.lamlet", text.as_bytes().to_vec()),
        &Options {
            language: Language::Lamlet,
            strict,
            ..Options::default()
        },
    )
}
fn output(text: &str) -> String {
    let c = build(text, false);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    String::from_utf8(c.output).unwrap()
}
fn error(text: &str, code: &str) {
    let c = build(text, false);
    assert!(
        c.diagnostics.iter().any(|d| d.code == code),
        "{text}: {:?}",
        c.diagnostics
    );
    assert!(c.output.is_empty());
    assert!(c.mappings.is_empty());
}
#[test]
fn flat_scope_releases_only_managed_resources_in_reverse_order() {
    assert_eq!(
        output(
            "@scope {\n@compute T all temp\n@fix thermostat all nvt temp 300 300 0.1\n@dump traj all custom 100 \"traj.lammpstrj\" id x y z\nrun 10000\n}\n"
        ),
        "compute T all temp\nfix thermostat all nvt temp 300 300 0.1\ndump traj all custom 100 \"traj.lammpstrj\" id x y z\nrun 10000\nundump traj\nunfix thermostat\nuncompute T\n"
    );
    assert_eq!(
        output(
            "@scope {\nfix legacy all nve\n@fix thermostat all langevin 300 300 0.1 12345\nrun 1\n}\nunfix legacy\n"
        ),
        "fix legacy all nve\nfix thermostat all langevin 300 300 0.1 12345\nrun 1\nunfix thermostat\nunfix legacy\n"
    );
}
#[test]
fn explicit_drop_supports_nonlast_resources_and_id_reuse() {
    assert_eq!(
        output(
            "@scope {\n@fix a all nve\n@compute T all temp\n@drop a\n@fix a all nve\nrun 1\n}\n"
        ),
        "fix a all nve\ncompute T all temp\nunfix a\nfix a all nve\nrun 1\nunfix a\nuncompute T\n"
    );
    error(
        "@scope {\n@fix a all nve\n@drop a\n@drop a\n}\n",
        "E_SCOPE_USE_AFTER_FREE",
    );
    error(
        "@scope {\n@fix T all nve\n@compute T all temp\n@drop T\n}\n",
        "E_SCOPE_RESOURCE",
    );
    error("@fix a all nve\n", "E_SCOPE_RESOURCE");
}
#[test]
fn nested_scopes_loops_and_bindings_obey_lexical_lifetimes() {
    assert_eq!(
        output(
            "@scope {\n@let n = 2\n@compute T all temp\n@for i in [1,2] {\n@scope {\n@fix a all nve\nrun @{n+i}\n}\n}\n}\n"
        ),
        "compute T all temp\nfix a all nve\nrun 3\nunfix a\nfix a all nve\nrun 4\nunfix a\nuncompute T\n"
    );
    error("@scope {\n@let n = 2\n}\nrun @{n}\n", "E_UNBOUND_NAME");
    error(
        "@scope {\n@for i in [1,2] {\n@fix a all nve\n}\n}\n",
        "E_SCOPE_OWNERSHIP",
    );
}
#[test]
fn ownership_and_use_after_release_are_checked() {
    for body in ["unfix a", "fix a all nve", "@fix a all nve"] {
        error(
            &format!("@scope {{\n@fix a all nve\n{body}\n}}\n"),
            "E_SCOPE_OWNERSHIP",
        );
    }
    error(
        "fix a all nve\n@scope {\n@fix a all nve\n}\n",
        "E_SCOPE_OWNERSHIP",
    );
    error(
        "@scope {\n@compute T all temp\n}\nvariable t equal c_T\n",
        "E_SCOPE_USE_AFTER_FREE",
    );
    error(
        "@scope {\n@fix a all nve\n}\nfix_modify a energy yes\n",
        "E_SCOPE_USE_AFTER_FREE",
    );
    assert_eq!(
        output("@scope {\n@fix a all nve\n}\nfix a all nve\nunfix a\n"),
        "fix a all nve\nunfix a\nfix a all nve\nunfix a\n"
    );
}
#[test]
fn dependencies_prevent_early_destruction_and_escaping_consumers() {
    error(
        "@scope {\n@compute T all temp\n@fix t all nvt temp 300 300 0.1\nfix_modify t temp T\n@drop T\n}\n",
        "E_SCOPE_DEPENDENCY",
    );
    error(
        "@scope {\n@compute T all temp\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\n}\n",
        "E_SCOPE_DEPENDENCY",
    );
    assert_eq!(
        output(
            "@scope {\n@compute T all temp\n@fix t all nvt temp 300 300 0.1\nfix_modify t temp T\n}\n"
        ),
        "compute T all temp\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\nunfix t\nuncompute T\n"
    );
}
#[test]
fn known_integrators_conflict_but_constraints_and_legacy_pairs_remain_allowed() {
    error(
        "@scope {\n@fix a all nve\n@fix b all nvt temp 300 300 0.1\n}\n",
        "E_EFFECT_CONFLICT",
    );
    error(
        "fix a all nve\n@scope {\n@fix b all nvt temp 300 300 0.1\n}\n",
        "E_EFFECT_CONFLICT",
    );
    assert!(output("@scope {\n@fix a all nvt temp 300 300 0.1\n@fix b all shake 0.0001 20 0 b 1\nrun 0\n}\n").ends_with("unfix b\nunfix a\n"));
    let legacy = "fix a all nve\nfix b all nvt temp 300 300 0.1\n";
    assert_eq!(output(legacy), legacy);
}
#[test]
fn unstructured_exits_and_embedded_ownership_changes_are_rejected() {
    for body in [
        "jump SELF end",
        "quit",
        "clear",
        "if '1' then 'jump SELF end'",
        "run 1 every 1 'jump SELF end'",
    ] {
        error(
            &format!("@scope {{\n@fix a all nve\n{body}\n}}\n"),
            "E_SCOPE_ESCAPE",
        );
    }
    error("@scope {\njump SELF end\n}\n", "E_SCOPE_ESCAPE");
    error(
        "@scope {\n@fix a all nve\nif '1' then 'unfix a'\n}\n",
        "E_SCOPE_OWNERSHIP",
    );
    let legacy = "jump SELF end\nlabel end\n";
    assert_eq!(output(legacy), legacy);
}
#[test]
fn opaque_effects_warn_and_strict_mode_rejects_without_claiming_proof() {
    for body in [
        "include common.in",
        "plugin hello",
        "run 1 every 1 'print hello'",
    ] {
        let text = format!("@scope {{\n@fix a all nve\n{body}\n}}\n");
        let c = build(&text, false);
        assert!(!c.has_errors(), "{:?}", c.diagnostics);
        assert!(c.diagnostics.iter().any(|d| d.code == "W_SCOPE_UNVERIFIED"));
        assert!(
            build(&text, true)
                .diagnostics
                .iter()
                .any(|d| d.code == "E_SCOPE_UNVERIFIED")
        );
    }
    assert!(
        !build(
            "@scope {\n@fix a all nvt temp 300 300 0.1\nrun 10\n}\n",
            true
        )
        .has_errors()
    );
}
#[test]
fn cleanup_maps_to_scope_end_and_keeps_expansion_context() {
    let c = build(
        "@for n in [0] {\n@scope {\n@fix a all nve\nrun @{n}\n}\n}\n",
        false,
    );
    assert!(!c.has_errors());
    let cleanup = c.mappings.iter().find(|m| m.generated_line == 3).unwrap();
    assert_eq!(cleanup.source.start_line, 5);
    assert_eq!(
        cleanup
            .expansion
            .iter()
            .find(|e| e.variable == "n")
            .unwrap()
            .iteration,
        0
    );
    let legacy = "@scope {\n@fix a all nve\n}\n";
    let c = compile_with_options(
        &Source::new("legacy.in", legacy.as_bytes().to_vec()),
        &Options::default(),
    );
    assert_eq!(c.output, legacy.as_bytes());
}

#[test]
fn persistent_borrowers_and_replaced_temperature_references_obey_lifetimes() {
    error(
        "@scope {\n@compute T all temp\nvariable x equal c_T\n}\n",
        "E_SCOPE_DEPENDENCY",
    );
    error(
        "@scope {\n@compute T all temp\nthermo_modify temp T\n}\n",
        "E_SCOPE_DEPENDENCY",
    );
    assert!(!build("@scope {\n@compute T all temp\nvariable x equal c_T\nvariable x delete\nthermo_modify temp T\nthermo_modify temp thermo_temp\n}\n", false).has_errors());
    assert!(!build("@scope {\n@compute T all temp\n@compute U all temp\n@fix f all nvt temp 300 300 0.1\nfix_modify f temp T\nfix_modify f temp U\n@drop T\n}\n", false).has_errors());
}
