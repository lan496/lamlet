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
