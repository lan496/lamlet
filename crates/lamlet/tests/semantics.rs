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

const EXCHANGE_SETUP: &str = "fix bath all nvt temp 300 300 0.1\nfix exchange all gcmc 10 10 10 1 123 300 -1 0.1\nthermo_style custom step\n";

#[test]
fn dynamic_dof_preserves_facts_independent_of_runtime_parameters() {
    for command in [
        "fix bath all nvt temp ${T} ${T} ${damping}",
        "fix bath all npt ${settings}",
        "fix exchange all gcmc 10 10 10 1 123 ${T} ${mu} ${displacement}",
        "variable damping equal 100*${dt}",
        "region incoming block 0 10 0 10 0 10",
        "region incoming block 0 ${L} 0 ${L} 0 ${L}",
        "restart 5000 state.1 state.2",
        "restart ${frequency} ${file}",
        "fix_modify bath dynamic/dof yes",
    ] {
        let text = format!("{EXCHANGE_SETUP}{command}\nrun 100\n");
        let result = check(&text);
        assert!(!result.has_errors(), "{command}: {:?}", result.diagnostics);
        assert_eq!(result.output, text.as_bytes());
        let findings = result
            .physical_findings
            .iter()
            .filter(|finding| {
                finding.rule_id == "LML-PHYS-010"
                    && finding
                        .evidence
                        .iter()
                        .any(|e| e.subject == "fix bath temperature selection")
            })
            .collect::<Vec<_>>();
        assert_eq!(findings.len(), 1, "{command}");
        assert_eq!(
            findings.iter().map(|f| f.outcome).collect::<Vec<_>>(),
            vec![Outcome::Violated],
            "{command}"
        );
        assert!(
            result.diagnostics.iter().any(|d| d.code == "LML-PHYS-010"),
            "{command}"
        );
    }
}

#[test]
fn dynamic_dof_preserves_uncertainty_across_opaque_effects() {
    for command in [
        "include opaque.in",
        "${command} arbitrary arguments",
        "fix ${id} all nvt temp 300 300 0.1",
        "fix bath ${group} nvt temp 300 300 0.1",
        "fix exchange all ${style} 10 1 incoming 123",
        "compute_modify bath_temp ${options}",
        "compute_modify bath_temp dynamic/dof ${policy}",
        "fix_modify bath temp ${selection}",
        "fix_modify bath ${options}",
    ] {
        let text = format!("{EXCHANGE_SETUP}{command}\nrun 100\n");
        let result = check(&text);
        assert!(!result.has_errors(), "{command}");
        assert_eq!(result.output, text.as_bytes());
        let outcomes = result
            .physical_findings
            .iter()
            .filter(|finding| {
                finding.rule_id == "LML-PHYS-010"
                    && finding
                        .evidence
                        .iter()
                        .any(|e| e.subject == "fix bath temperature selection")
            })
            .map(|f| f.outcome)
            .collect::<Vec<_>>();
        assert_eq!(outcomes, vec![Outcome::Unknown], "{command}");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "LML-PHYS-010"),
            "{command}"
        );
    }
}

#[test]
fn dynamic_dof_cannot_restore_resource_roles_with_only_compute_updates() {
    let result = check(&format!(
        "{EXCHANGE_SETUP}include opaque.in\ncompute T all temp/com\nfix_modify bath temp T\nthermo_modify temp T\nrun 100\n"
    ));
    assert!(!result.has_errors());
    let outcomes = result
        .physical_findings
        .iter()
        .filter(|f| matches!(f.rule_id, "LML-PHYS-010" | "LML-PHYS-002"))
        .map(|f| f.outcome)
        .collect::<Vec<_>>();
    assert!(!outcomes.is_empty());
    assert!(outcomes.iter().all(|o| *o == Outcome::Unknown));
    for suffix in ["unfix exchange", "clear", "fix exchange all nve"] {
        let result = check(&format!(
            "{EXCHANGE_SETUP}include opaque.in\n{suffix}\nrun 100\n"
        ));
        assert!(!result.has_errors());
        assert!(
            !result
                .physical_findings
                .iter()
                .any(|f| f.rule_id == "LML-PHYS-010"),
            "{suffix}"
        );
    }
}

#[test]
fn dynamic_dof_requires_new_definitions_to_restore_roles_after_opaque_effects() {
    let result = check(&format!(
        "{EXCHANGE_SETUP}include opaque.in\n{EXCHANGE_SETUP}compute_modify bath_temp dynamic/dof yes\nrun 100\n"
    ));
    assert!(!result.has_errors());
    let outcomes = result
        .physical_findings
        .iter()
        .filter(|f| f.rule_id == "LML-PHYS-010")
        .map(|f| f.outcome)
        .collect::<Vec<_>>();
    assert_eq!(outcomes, vec![Outcome::Satisfied]);

    let result =
        check("fix exchange all gcmc 10 10 10 1 123 300 -1 0.1\ninclude opaque.in\nrun 100\n");
    assert!(!result.has_errors());
    let outcomes = result
        .physical_findings
        .iter()
        .filter(|f| f.rule_id == "LML-PHYS-010")
        .map(|f| f.outcome)
        .collect::<Vec<_>>();
    assert_eq!(outcomes, vec![Outcome::Unknown]);
    assert!(!result.diagnostics.iter().any(|d| d.code == "LML-PHYS-010"));
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
    for suffix in ["unfix exchange\nrun 100\n", "run 0\n", "clear\nrun 100\n"] {
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
