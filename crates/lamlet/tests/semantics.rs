use lamlet::{Language, Options, compile_with_options, semantics::Outcome, source::Source};
fn check(text: &str) -> lamlet::Compilation {
    compile_with_options(
        &Source::new("physics.lamlet", text.as_bytes().to_vec()),
        &Options {
            language: Language::Lamlet,
            strict: true,
            ..Options::default()
        },
    )
}

#[test]
fn changing_atom_count_requires_dynamic_dof_for_selected_temperatures() {
    for exchange in [
        "deposit 10 1 1 123 region incoming",
        "pour 10 1 123 region incoming",
        "gcmc 10 10 10 1 123 300 -1 0.1",
        "evaporate 10 1 incoming 123",
    ] {
        for (setting, expected) in [
            ("", Outcome::Violated),
            ("compute_modify T dynamic/dof yes\n", Outcome::Satisfied),
            (
                "compute_modify T dynamic/dof no extra/dof -1\n",
                Outcome::Violated,
            ),
        ] {
            let text = format!(
                "compute T all temp\n{setting}thermo_modify temp T\nfix exchange all {exchange}\nrun 100\n"
            );
            let result = check(&text);
            assert!(!result.has_errors(), "{:?}", result.diagnostics);
            assert_eq!(result.output, text.as_bytes());
            let finding = result
                .physical_findings
                .iter()
                .find(|f| f.rule_id == "LML-PHYS-010")
                .unwrap();
            assert_eq!(finding.outcome, expected, "{text}");
            assert_eq!(
                result.diagnostics.iter().any(|d| d.code == "LML-PHYS-010"),
                expected == Outcome::Violated
            );
        }
    }
}

#[test]
fn dynamic_dof_tracks_final_selection_cleanup_and_unknown_groups() {
    for (prefix, expected) in [
        (
            "compute T mobile temp\nthermo_modify temp T\n",
            Some(Outcome::Unknown),
        ),
        (
            "compute T all temp\ncompute_modify T custom yes\nthermo_modify temp T\n",
            Some(Outcome::Unknown),
        ),
        (
            "compute T all temp\ncompute_modify T dynamic/dof yes dynamic/dof no\nthermo_modify temp T\n",
            Some(Outcome::Violated),
        ),
        (
            "compute T all temp\ncompute_modify T dynamic/dof no dynamic/dof yes\nthermo_modify temp T\n",
            Some(Outcome::Satisfied),
        ),
        ("compute T all temp\nthermo_style custom step\n", None),
    ] {
        let result = check(&format!(
            "{prefix}fix exchange gas evaporate 10 1 incoming 123\nrun 100\n"
        ));
        assert!(!result.has_errors());
        assert_eq!(
            result
                .physical_findings
                .iter()
                .find(|f| f.rule_id == "LML-PHYS-010")
                .map(|f| f.outcome),
            expected
        );
    }
    for suffix in [
        "unfix exchange\nrun 100\n",
        "run 0\n",
        "include opaque.in\nrun 100\n",
        "clear\nrun 100\n",
    ] {
        let result = check(&format!(
            "compute T all temp\nthermo_modify temp T\nfix exchange all evaporate 10 1 incoming 123\n{suffix}"
        ));
        assert!(
            !result
                .physical_findings
                .iter()
                .any(|f| f.rule_id == "LML-PHYS-010")
        );
    }
}

#[test]
fn dynamic_dof_follows_thermostat_selection_and_explicit_thermo_compute_fields() {
    for selection in [
        "thermo_style custom step c_T\n",
        "fix bath all nvt temp 300 300 0.1\nfix_modify bath temp T\nthermo_style custom step\n",
    ] {
        let text = format!(
            "compute T all temp/com\n{selection}fix exchange all evaporate 10 1 incoming 123\nrun 100\n"
        );
        let result = check(&text);
        assert!(!result.has_errors());
        assert_eq!(
            result
                .physical_findings
                .iter()
                .filter(|f| f.rule_id == "LML-PHYS-010")
                .map(|f| f.outcome)
                .collect::<Vec<_>>(),
            vec![Outcome::Violated]
        );
    }
}
fn outcome(text: &str, expected: Outcome) -> lamlet::Compilation {
    let c = check(text);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(
        c.physical_findings
            .iter()
            .filter(|f| f.rule_id == "LML-PHYS-002")
            .map(|f| f.outcome)
            .collect::<Vec<_>>(),
        vec![expected]
    );
    assert_eq!(c.output, text.as_bytes());
    c
}
#[test]
fn temperature_bias_difference_is_conditional_and_never_autofixed() {
    let c = outcome(
        "compute com all temp/com\nfix thermostat all nvt temp 300 300 0.1\nfix_modify thermostat temp com\nrun 1\n",
        Outcome::Violated,
    );
    let d = c
        .diagnostics
        .iter()
        .find(|d| d.code == "LML-PHYS-002")
        .unwrap();
    assert_eq!(d.severity, lamlet::diagnostic::Severity::Warning);
    let evidence = d
        .evidence
        .iter()
        .find(|e| e.subject == "fix thermostat")
        .unwrap();
    assert_eq!(evidence.definition, "compute com: temp/com, group all");
    assert_eq!(evidence.span.start_line, 1);
    assert_eq!(d.span.start_line, 4);
    assert!(!d.suggested_actions.is_empty());
}
#[test]
fn matching_selection_and_default_definitions_satisfy_the_narrow_rule() {
    outcome(
        "compute com all temp/com\nfix t all nvt temp 300 300 0.1\nfix_modify t temp com\nthermo_modify temp com\nrun 0\n",
        Outcome::Satisfied,
    );
    outcome(
        "fix t all nvt temp 300 300 0.1\nrun 0\n",
        Outcome::Satisfied,
    );
}
#[test]
fn unknown_definitions_dof_modifications_and_group_overlap_remain_unknown() {
    for script in [
        "compute T all temp/partial 1 1 0\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\nrun 0\n",
        "compute T all temp\ncompute_modify T extra/dof 6\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\nrun 0\n",
        "compute T mobile temp/com\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\nrun 0\n",
        "fix t all nvt temp 300 300 0.1\ninclude external.in\nrun 0\n",
    ] {
        let c = outcome(script, Outcome::Unknown);
        assert!(!c.diagnostics.iter().any(|d| d.code == "LML-PHYS-002"));
    }
}
#[test]
fn final_configuration_at_run_and_thermo_style_reset_are_observed() {
    outcome(
        "compute T all temp/com\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\nthermo_modify temp T\nthermo_style one\nrun 0\n",
        Outcome::Violated,
    );
    outcome(
        "compute T all temp/com\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\nthermo_style custom step c_T\nrun 0\n",
        Outcome::Unknown,
    );
    let c = check("fix t all nvt temp 300 300 0.1\nunfix t\nrun 0\n");
    assert!(
        !c.physical_findings
            .iter()
            .any(|f| f.rule_id == "LML-PHYS-002")
    );
}
#[test]
fn com_momentum_and_shake_do_not_inject_degree_of_freedom_corrections() {
    let script = "compute T all temp\nfix thermostat all nvt temp 300 300 0.1\nfix momentum all momentum 1 linear 1 1 1\nfix constraint all shake 0.0001 20 0 b 1\nrun 0\n";
    let c = outcome(script, Outcome::Satisfied);
    assert!(!c.diagnostics.iter().any(|d| d.code == "LML-PHYS-002"));
}
#[test]
fn rule_metadata_is_searchable_and_reports_conditional_coverage() {
    let rule = lamlet::semantics::RULES
        .iter()
        .find(|r| r.id == "LML-PHYS-002")
        .unwrap();
    assert_eq!(rule.confidence, "conditional");
    assert!(!rule.autofix);
    assert!(
        rule.references
            .contains(&"https://docs.lammps.org/compute_temp_com.html")
    );
    assert!(rule.applies_to.contains(&"fix:nvt"));
}
