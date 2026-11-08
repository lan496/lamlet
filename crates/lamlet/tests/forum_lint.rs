use lamlet::{
    Language, Options, compile_with_options, diagnostic::Severity, semantics::Outcome,
    source::Source,
};

fn check(text: &str) -> lamlet::Compilation {
    let source = Source::new("input.in", text.as_bytes().to_vec());
    let compile = |strict| {
        let result = compile_with_options(
            &source,
            &Options {
                language: Language::Lammps,
                strict,
                ..Options::default()
            },
        );
        assert!(!result.has_errors(), "{text}: {:?}", result.diagnostics);
        assert_eq!(result.output, text.as_bytes());
        result
    };
    let regular = compile(false);
    let strict = compile(true);
    assert_eq!(
        serde_json::to_string(&regular.report(&source)).unwrap(),
        serde_json::to_string(&strict.report(&source)).unwrap()
    );
    regular
}

fn outcome(text: &str, id: &str, expected: Outcome) {
    let result = check(text);
    let findings: Vec<_> = result
        .physical_findings
        .iter()
        .filter(|f| f.rule_id == id)
        .collect();
    assert_eq!(findings.len(), 1, "{id}: {text}");
    assert!(
        findings.iter().all(|f| f.outcome == expected),
        "{id}: {text}"
    );
    let warning = result.diagnostics.iter().find(|d| d.code == id);
    assert_eq!(
        warning.is_some(),
        expected == Outcome::Violated,
        "{id}: {text}"
    );
    if let Some(warning) = warning {
        assert_eq!(warning.severity, Severity::Warning);
        assert!(!warning.evidence.is_empty());
        assert!(!warning.suggested_actions.is_empty());
    }
}

#[test]
fn orientation_integers_are_distinct_from_custom_primitive_vectors() {
    let id = "LML-MAN-LATTICE-ORIENT-INTEGERS";
    for text in [
        "lattice fcc 4.05 orient x 0.8 0.6 0 orient y -0.6 0.8 0\n",
        "lattice fcc 4.05 orient x 1.0 0 0\n",
        "lattice fcc 4.05 orient x 1e0 0 0\n",
        "lattice fcc 4.05 orient x 0.8 0.6 0 orient x 1 0 0\n",
    ] {
        outcome(text, id, Outcome::Violated);
    }
    for text in [
        "lattice custom 3.52 a1 1.0 0.0 0.0 a2 0.5 1.0 0.0 a3 0.0 0.0 0.5 basis 0.0 0.0 0.0 triclinic/general\n",
        "lattice fcc 4.05 orient x +4 +3 0 orient y -3 4 0\n",
    ] {
        outcome(text, id, Outcome::Satisfied);
    }
    for text in [
        "lattice fcc 4.05 orient x ${x} 0 0\n",
        "lattice future/style 4.05 orient x 0.8 0.6 0\n",
        "lattice fcc 4.05 orient x 1\n",
        "lattice fcc 4.05 unknown orient x 0.8 0.6 0\n",
    ] {
        outcome(text, id, Outcome::Unknown);
    }
}

#[test]
fn orientation_geometry_uses_defaults_final_options_and_checked_arithmetic() {
    let id = "LML-MAN-LATTICE-ORIENTATION";
    for text in [
        "lattice fcc 4.05 orient x 1 1 0\n",
        "lattice fcc 4.05 orient z 0 0 -1\n",
        "lattice fcc 4.05 orient x 0 0 0\n",
        "dimension 2\nlattice sq 1 orient x 0 0 1 orient y 0 1 0 orient z -1 0 0\n",
    ] {
        outcome(text, id, Outcome::Violated);
    }
    for text in [
        "lattice fcc 4.05 orient x 4 3 0 orient y -3 4 0 orient z 0 0 7\n",
        "lattice fcc 4.05 orient x 1 1 0 orient x 1 0 0\n",
        "lattice hcp 1\n",
        "include external.in\nlattice fcc 1\n",
        "lattice custom 1 a1 1 0 0 a2 0.5 1 0 a3 0 0 1 basis 0 0 0\n",
    ] {
        outcome(text, id, Outcome::Satisfied);
    }
    for text in [
        "lattice fcc 4.05 orient x ${x} 0 0\n",
        "lattice fcc 4.05 orient x 0.8 0.6 0\n",
        "lattice fcc 1 orient x 9223372036854775807 0 0 orient y 0 9223372036854775807 0 orient z 0 0 9223372036854775807\n",
        "include external.in\nlattice fcc 1 orient x 0 0 1 orient y 0 1 0 orient z -1 0 0\n",
    ] {
        outcome(text, id, Outcome::Unknown);
    }
}

#[test]
fn drag_is_an_advisory_about_the_final_known_value_and_known_option_grammar() {
    let id = "LML-MAN-NH-DRAG";
    for (style, options) in [
        ("nvt", "temp 300 300 0.1"),
        ("nvt/gpu", "temp 300 300 0.1"),
        ("npt/omp", "temp 300 300 0.1 iso 0 0 1"),
        ("nph/kk", "iso 0 0 1"),
    ] {
        outcome(
            &format!("fix ensemble all {style} {options} drag 1\n"),
            id,
            Outcome::Violated,
        );
    }
    for text in [
        "fix ensemble all npt temp 300 300 0.1 iso 0 0 1\n",
        "fix ensemble all npt temp 300 300 0.1 iso 0 0 1 drag 0\n",
        "fix ensemble all npt temp 300 300 0.1 iso 0 0 1 dilate drag\n",
        "fix ensemble all nvt temp 300 300 0.1 drag 1 drag 0\n",
    ] {
        outcome(text, id, Outcome::Satisfied);
    }
    for text in [
        "fix ensemble all nvt temp 300 300 0.1 drag ${drag}\n",
        "fix ensemble all nvt temp 300 300 0.1 custom drag 1\n",
        "fix ensemble all nvt temp 300 300 0.1 drag NaN\n",
        "fix ensemble all nvt temp 300 300 0.1 drag\n",
    ] {
        outcome(text, id, Outcome::Unknown);
    }
    assert!(
        !check("fix ensemble all nvt/custom temp 300 300 0.1 drag 1\n")
            .physical_findings
            .iter()
            .any(|f| f.rule_id == id)
    );
}

#[test]
fn neighbor_exclusions_are_checked_at_execution_after_final_configuration() {
    let id = "LML-MAN-KSPACE-EXCLUSIONS";
    for options in [
        "exclude type 1 2",
        "exclude group frozen frozen",
        "exclude molecule/intra rigid",
        "exclude molecule/inter rigid",
        "exclude none exclude type C H",
        "exclude type 1 2 check yes",
        "exclude type 1 2 collection/type 2 1 2",
    ] {
        outcome(
            &format!("kspace_style pppm 1e-4\nneigh_modify {options}\nrun 0\n"),
            id,
            Outcome::Violated,
        );
    }
    outcome(
        "neigh_modify exclude type 1 2\nkspace_style ewald/omp 1e-4\nminimize 1e-8 1e-10 100 1000\n",
        id,
        Outcome::Violated,
    );
    for text in [
        "kspace_style pppm 1e-4\nneigh_modify exclude type 1 2\nneigh_modify exclude none\nrun 0\n",
        "kspace_style pppm 1e-4\nneigh_modify exclude type 1 2 exclude none\nrun 0\n",
        "kspace_style pppm 1e-4\nneigh_modify include exclude\nrun 0\n",
        "kspace_style pppm 1e-4\nneigh_modify collection/type 2 exclude none\nrun 0\n",
        "kspace_style pppm 1e-4\nneigh_modify exclude type 1 2\nkspace_style none\nrun 0\n",
        "kspace_style pppm 1e-4\nneigh_modify exclude type 1 2\nclear\nrun 0\n",
        "include external.in\nkspace_style none\nrun 0\n",
        "include external.in\nneigh_modify exclude none\nrun 0\n",
        "neigh_modify exclude type 1 2\nrun 0\n",
    ] {
        outcome(text, id, Outcome::Satisfied);
    }
}

#[test]
fn opaque_effects_and_unreviewed_solvers_do_not_establish_a_conflict() {
    let id = "LML-MAN-KSPACE-EXCLUSIONS";
    for solver in ["pppm/custom", "custom/solver", "zero", "scafacos"] {
        outcome(
            &format!("neigh_modify exclude type 1 2\nkspace_style {solver} 1e-4\nrun 0\n"),
            id,
            Outcome::Unknown,
        );
    }
    for opaque in [
        "include external.in",
        "jump SELF loop",
        "plugin load custom.so",
        "read_restart restart.bin",
        "fix callback all custom/style",
        "neigh_modify unknown exclude none",
        "neigh_modify exclude type ${a} 2",
        "neigh_modify collection/type 18446744073709551615",
    ] {
        outcome(
            &format!("kspace_style pppm 1e-4\nneigh_modify exclude type 1 2\n{opaque}\nrun 0\n"),
            id,
            Outcome::Unknown,
        );
    }
    let text =
        "kspace_style pppm 1e-4\nneigh_modify exclude type 1 2\nneigh_modify check yes\nrun 0\n";
    let result = check(text);
    let warning = result.diagnostics.iter().find(|d| d.code == id).unwrap();
    for definition in ["kspace_style pppm 1e-4", "neigh_modify exclude type 1 2"] {
        assert!(warning.evidence.iter().any(|e| e.definition == definition));
    }
    assert!(
        !check("kspace_style pppm 1e-4\nneigh_modify exclude type 1 2\nrun 10 every 1 command\n")
            .physical_findings
            .iter()
            .any(|f| f.rule_id == id)
    );
}

#[test]
fn forum_provenance_is_available_through_the_public_catalog() {
    let rule = lamlet::manual_lint::catalog()
        .rules
        .iter()
        .find(|r| r.id == "LML-MAN-LATTICE-ORIENT-INTEGERS")
        .unwrap();
    assert!(rule.discussions.iter().any(|url| url.ends_with("/14833")));
    assert_eq!(rule.source.page, "lattice.rst");
    assert!(!rule.autofix);
}
