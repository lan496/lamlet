use lamlet::{
    Options, compile_with_options, diagnostic::Severity, semantics::Outcome, source::Source,
};

fn check(text: &str) -> lamlet::Compilation {
    let result = compile_with_options(
        &Source::new("in.test", text.as_bytes().to_vec()),
        &Options::default(),
    );
    // Negative fixtures can also violate pre-existing schema errors. New manual
    // constraints always remain warnings, including under strict validation.
    assert!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("LML-MAN-"))
            .all(|d| d.severity == Severity::Warning)
    );
    if !result.has_errors() {
        assert_eq!(result.output, text.as_bytes());
    }
    result
}

fn outcome(text: &str, id: &str, expected: Outcome) {
    let result = check(text);
    let finding = result
        .physical_findings
        .iter()
        .find(|f| f.rule_id == id)
        .unwrap_or_else(|| panic!("missing {id} for {text}: {:?}", result.physical_findings));
    assert_eq!(finding.outcome, expected, "{id}: {text}");
    let diagnostic = result.diagnostics.iter().find(|d| d.code == id);
    assert_eq!(diagnostic.is_some(), expected == Outcome::Violated);
    if let Some(diagnostic) = diagnostic {
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert!(
            diagnostic
                .notes
                .iter()
                .any(|n| n.contains("/blob/0165101ce505734e8fb249694b22e295a7ab4182/"))
        );
    }
}

#[test]
fn reviewed_examples_execute_through_the_public_compiler() {
    for rule in &lamlet::manual_lint::catalog().rules {
        for example in &rule.examples {
            let expected = match example.outcome.as_str() {
                "violated" => Outcome::Violated,
                "satisfied" => Outcome::Satisfied,
                "unknown" => Outcome::Unknown,
                value => panic!("invalid expected outcome: {value}"),
            };
            outcome(&example.input, &rule.id, expected);
        }
    }
}

#[test]
fn numeric_limits_include_zero_and_large_or_nonfinite_counterexamples() {
    for (text, expected) in [
        ("run 0\n", Outcome::Satisfied),
        ("run -1\n", Outcome::Violated),
        ("run 2147483647\n", Outcome::Satisfied),
        ("run 2147483648\n", Outcome::Violated),
        ("run 1.5\n", Outcome::Violated),
        ("run \"${steps}\"\n", Outcome::Unknown),
    ] {
        outcome(text, "LML-MAN-RUN-LIMIT", expected);
    }
    for (text, expected) in [
        ("mass 1 NaN\n", Outcome::Violated),
        ("mass 1 inf\n", Outcome::Violated),
        ("mass 1 -0.5\n", Outcome::Violated),
        ("mass 1 1e-10\n", Outcome::Satisfied),
        ("mass 1 \"${mass}\"\n", Outcome::Unknown),
    ] {
        outcome(text, "LML-MAN-MASS", expected);
    }
    assert!(
        !check("run 2147483648 upto\n")
            .diagnostics
            .iter()
            .any(|d| d.code == "LML-MAN-RUN-LIMIT")
    );
}

#[test]
fn unknown_effects_and_clear_invalidate_and_reset_box_facts() {
    outcome(
        "read_data atoms.data\nunits metal\n",
        "LML-MAN-UNITS-BEFORE-BOX",
        Outcome::Violated,
    );
    outcome(
        "read_data atoms.data\nclear\nunits metal\n",
        "LML-MAN-UNITS-BEFORE-BOX",
        Outcome::Satisfied,
    );
    for opaque in [
        "include external.in",
        "if \"1\" then \"clear\"",
        "custom/plugin anything",
        "run 10 every 1 \"clear\"",
    ] {
        outcome(
            &format!("read_data atoms.data\n{opaque}\nunits metal\n"),
            "LML-MAN-UNITS-BEFORE-BOX",
            Outcome::Unknown,
        );
    }
}

#[test]
fn sampling_arithmetic_handles_overflow_and_runtime_values() {
    for (arguments, expected) in [
        ("0 1 10", Outcome::Violated),
        ("1 0 10", Outcome::Violated),
        ("2 6 10", Outcome::Violated),
        ("2 5 10", Outcome::Satisfied),
        (
            "9223372036854775807 2 9223372036854775807",
            Outcome::Violated,
        ),
        ("\"${stride}\" 1 10", Outcome::Unknown),
    ] {
        outcome(
            &format!("fix sample all ave/time {arguments} c_value\n"),
            "LML-MAN-AVE-TIME",
            expected,
        );
    }
}

#[test]
fn style_restrictions_require_an_exact_known_context() {
    outcome(
        "units metal\npair_style amoeba\n",
        "LML-MAN-AMOEBA-UNITS",
        Outcome::Violated,
    );
    outcome(
        "units real\npair_style amoeba\n",
        "LML-MAN-AMOEBA-UNITS",
        Outcome::Satisfied,
    );
    outcome(
        "include setup.in\npair_style amoeba\n",
        "LML-MAN-AMOEBA-UNITS",
        Outcome::Unknown,
    );
    outcome(
        "pair_style eam\npair_modify shift yes\n",
        "LML-MAN-PAIR-EAM-MODIFY-SHIFT",
        Outcome::Violated,
    );
    outcome(
        "pair_style eam\npair_modify shift no\n",
        "LML-MAN-PAIR-EAM-MODIFY-SHIFT",
        Outcome::Satisfied,
    );
    outcome(
        "pair_style eam\npair_modify shift yes shift no\n",
        "LML-MAN-PAIR-EAM-MODIFY-SHIFT",
        Outcome::Satisfied,
    );
    for text in [
        "pair_style hybrid eam lj/cut 2.5\npair_modify shift yes\n",
        "pair_style eam\ninclude setup.in\npair_modify shift yes\n",
        "pair_style custom/plugin\npair_modify shift yes\n",
    ] {
        assert!(
            !check(text)
                .diagnostics
                .iter()
                .any(|d| d.code == "LML-MAN-PAIR-EAM-MODIFY-SHIFT")
        );
    }
    outcome(
        "dimension 2\nkspace_style pppm 1e-5\n",
        "LML-MAN-KSPACE-EWALD-3D",
        Outcome::Violated,
    );
    outcome(
        "dimension 3\nboundary p p f\nkspace_style pppm 1e-5\nkspace_modify slab 3.0\n",
        "LML-MAN-KSPACE-EWALD-3D",
        Outcome::Satisfied,
    );
    assert!(
        !check("dimension 2\nkspace_style scafacos direct 1e-5\n")
            .diagnostics
            .iter()
            .any(|d| d.code.starts_with("LML-MAN-KSPACE-"))
    );
}

#[test]
fn settings_across_commands_keep_their_evidence_and_reset_on_clear() {
    let result = check("neigh_modify delay 6\nneigh_modify every 4\n");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "LML-MAN-NEIGHBOR-DELAY")
        .unwrap();
    assert_eq!(diagnostic.span.start_line, 2);
    assert!(
        diagnostic
            .evidence
            .iter()
            .any(|e| e.subject == "neighbor_delay" && e.span.start_line == 1)
    );
    assert!(
        !check("neigh_modify delay 6\nclear\nneigh_modify every 4\n")
            .diagnostics
            .iter()
            .any(|d| d.code == "LML-MAN-NEIGHBOR-DELAY")
    );

    let result = check(
        "pair_style lj/cut 2.5\npair_modify tail yes\npair_style lj/cut 3.0\npair_modify shift yes\n",
    );
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "LML-MAN-PAIR-SHIFT-TAIL")
        .unwrap();
    assert_eq!(diagnostic.span.start_line, 4);
    assert!(
        diagnostic
            .evidence
            .iter()
            .any(|e| e.subject == "pair_tail" && e.span.start_line == 2)
    );
    assert!(!check("pair_style lj/cut 2.5\npair_modify tail yes\npair_style zero 3.0\npair_modify shift yes\n").diagnostics.iter().any(|d| d.code == "LML-MAN-PAIR-SHIFT-TAIL"));
}

#[test]
fn dynamic_callbacks_and_unknown_grammars_never_establish_state_proofs() {
    outcome(
        "dimension 2\ncompute value all viscosity/cos \"$(py_callback())\"\n",
        "LML-MAN-COMPUTE-VISCOSITY-COS-3D",
        Outcome::Unknown,
    );
    for opaque in [
        "include setup.in",
        "custom/plugin call",
        "variable callback python run_something",
        "print \"$(py_callback())\"",
        "fix callback all custom/plugin",
        "fix callback all custom/plugin\nrun 10",
    ] {
        let text = format!("read_data atoms.data\n{opaque}\nunits metal\n");
        outcome(&text, "LML-MAN-UNITS-BEFORE-BOX", Outcome::Unknown);
    }
    for options in [
        "exclude type 1 2 delay 3 every 2",
        "unknown_keyword page one 1",
        "collection/type 2 1 2",
    ] {
        outcome(
            &format!("neigh_modify {options}\n"),
            "LML-MAN-NEIGHBOR-DELAY",
            Outcome::Unknown,
        );
    }
    outcome(
        "read_data atoms.data group add\n",
        "LML-MAN-READ-DATA-ADD",
        Outcome::Satisfied,
    );
    outcome(
        "read_data atoms.data fix handler add shift\n",
        "LML-MAN-READ-DATA-OPTIONS",
        Outcome::Satisfied,
    );
    outcome(
        "dimension 2\nprocessors 1 1 *\n",
        "LML-MAN-PROCESSORS-2D",
        Outcome::Unknown,
    );
}

#[test]
fn resource_deletions_and_group_static_conversion_remove_known_restrictions() {
    outcome(
        "dump d all atom 10 out.dump\nundump d\nreset_timestep 0\n",
        "LML-MAN-RESET-DUMP",
        Outcome::Satisfied,
    );
    outcome(
        "fix timer all dt/reset 1 0.01 0.1 0.1\nunfix timer\nreset_timestep 0\n",
        "LML-MAN-RESET-TIME-FIX",
        Outcome::Satisfied,
    );
    outcome(
        "group mobile dynamic all every 10\ngroup mobile static\ncompute value mobile msd\n",
        "LML-MAN-COMPUTE-MSD-GROUP",
        Outcome::Satisfied,
    );
    outcome(
        "include setup.in\ngroup mobile clear\ncompute value mobile msd\n",
        "LML-MAN-COMPUTE-MSD-GROUP",
        Outcome::Unknown,
    );
    // LAMMPS temporarily adjusts neighbor rebuilding during minimization.
    let result = check("neigh_modify every 10 delay 20\nminimize 0.0 1e-8 100 1000\n");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code.starts_with("LML-MAN-"))
    );
}

#[test]
fn every_warning_preserves_accepted_legacy_bytes_under_strict_mode() {
    let text = "neigh_modify delay 3 every 2\npair_modify shift yes tail yes\nrun 2147483648\n";
    for strict in [false, true] {
        let source = Source::new("input.in", text.as_bytes().to_vec());
        let result = compile_with_options(
            &source,
            &Options {
                strict,
                ..Options::default()
            },
        );
        assert!(!result.has_errors());
        assert_eq!(result.output, text.as_bytes());
        for code in [
            "LML-MAN-NEIGHBOR-DELAY",
            "LML-MAN-PAIR-SHIFT-TAIL",
            "LML-MAN-RUN-LIMIT",
        ] {
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|d| d.code == code && d.severity == Severity::Warning)
            );
        }
    }
}
