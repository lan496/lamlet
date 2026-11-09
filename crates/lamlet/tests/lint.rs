use lamlet::{
    Language, Options, compile_with_options, diagnostic::Severity, semantics::Outcome,
    source::Source,
};

fn check(text: &str) -> lamlet::Compilation {
    let result = compile_with_options(
        &Source::new("input.in", text.as_bytes().to_vec()),
        &Options {
            language: Language::Lammps,
            strict: true,
            ..Options::default()
        },
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert_eq!(result.output, text.as_bytes());
    result
}

fn warning(text: &str, code: &str, expected: bool) {
    let result = check(text);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code && d.severity == Severity::Warning),
        expected,
        "{:?}",
        result.diagnostics
    );
    if expected {
        let finding = result
            .physical_findings
            .iter()
            .find(|f| f.rule_id == code)
            .unwrap();
        assert_eq!(finding.outcome, Outcome::Violated);
        assert!(!finding.evidence.is_empty());
        assert!(!finding.suggested_actions.is_empty());
    }
}

#[test]
fn ordinary_integrators_are_warned_without_rejecting_or_rewriting_legacy() {
    warning(
        "fix a all nve\nfix b all nvt temp 300 300 0.1\nrun 10\n",
        "LML-PHYS-001",
        true,
    );
    for text in [
        "fix a all nve\nunfix a\nfix b all nvt temp 300 300 0.1\nrun 10\n",
        "fix a all nve\nfix b all langevin 300 300 0.1 123\nrun 10\n",
        "fix a all nve\nfix b all nvt temp 300 300 0.1\nrun 0\n",
        "group left type 1\ngroup right type 2\nfix a left nve\nfix b right nve\nrun 10\n",
        "fix a all nve\ninclude change.in\nfix b all nve\nrun 10\n",
        "fix a all nve\nfix b all nve\nfix callback all custom/plugin\nrun 10\n",
    ] {
        warning(text, "LML-PHYS-001", false);
    }
    let result =
        check("group left type 1\ngroup right type 2\nfix a left nve\nfix b right nve\nrun 10\n");
    assert_eq!(
        result
            .physical_findings
            .iter()
            .find(|f| f.rule_id == "LML-PHYS-001")
            .unwrap()
            .outcome,
        Outcome::Unknown
    );
}

#[test]
fn temperature_controllers_are_distinct_from_integrators() {
    warning(
        "fix a all nvt temp 300 300 0.1\nfix b all langevin 300 300 0.1 123\nrun 10\n",
        "LML-PHYS-003",
        true,
    );
    warning(
        "fix a all nve\nfix b all langevin 300 300 0.1 123\nrun 10\n",
        "LML-PHYS-003",
        false,
    );
    warning(
        "fix a all nvt/omp temp 300 300 0.1\nfix b all nve/omp\nrun 10\n",
        "LML-PHYS-001",
        true,
    );
}

#[test]
fn shake_follows_force_modifiers_but_has_no_blanket_nvt_order_requirement() {
    warning(
        "fix bonds all shake 0.0001 20 0 b 1\nfix bath all langevin 300 300 0.1 123\nfix step all nve\nrun 10\n",
        "LML-PHYS-007",
        true,
    );
    warning(
        "fix bonds all shake 0.0001 20 0 b 1\nfix freeze all setforce 0 0 0\nrun 10\n",
        "LML-PHYS-007",
        true,
    );
    for text in [
        "fix bath all langevin 300 300 0.1 123\nfix bonds all shake 0.0001 20 0 b 1\nfix step all nve\nrun 10\n",
        "fix bonds all shake 0.0001 20 0 b 1\nfix bath all nvt temp 300 300 0.1\nrun 10\n",
        "fix bath all nvt temp 300 300 0.1\nfix bonds all shake 0.0001 20 0 b 1\nrun 10\n",
        "fix bonds all shake 0.0001 20 0 b 1\nfix bath all langevin 300 300 0.1 123\nunfix bath\nrun 10\n",
        "fix bonds all shake 0.0001 20 0 b 1\nfix bath all langevin 300 300 0.1 123\nrun 0\n",
    ] {
        warning(text, "LML-PHYS-007", false);
    }
}

#[test]
fn rattle_follows_integrators_and_post_force_modifiers() {
    for style in [
        "nve",
        "nvt temp 300 300 0.1",
        "npt temp 300 300 0.1 iso 0 0 1",
        "nph iso 0 0 1",
        "langevin 300 300 0.1 123",
        "setforce 0 0 0",
    ] {
        warning(
            &format!("fix bonds all rattle 0.0001 20 0 b 1\nfix update all {style}\nrun 10\n"),
            "LML-PHYS-008",
            true,
        );
        warning(
            &format!("fix update all {style}\nfix bonds all rattle 0.0001 20 0 b 1\nrun 10\n"),
            "LML-PHYS-008",
            false,
        );
    }
}

#[test]
fn same_id_replacement_preserves_order_but_unfix_and_recreate_moves_it() {
    let start = "fix bonds all shake 0.0001 20 0 b 1\nfix bath all langevin 300 300 0.1 123\n";
    warning(
        &format!("{start}fix bonds all shake 0.00001 40 0 b 1\nrun 10\n"),
        "LML-PHYS-007",
        true,
    );
    warning(
        &format!("{start}unfix bonds\nfix bonds all shake 0.00001 40 0 b 1\nrun 10\n"),
        "LML-PHYS-007",
        false,
    );
    warning(
        "fix bath all langevin 300 300 0.1 123\nfix bonds all shake 0.0001 20 0 b 1\nfix bath all langevin 400 400 0.1 123\nrun 10\n",
        "LML-PHYS-007",
        false,
    );
    let result = check(&format!(
        "{start}fix bonds all shake 0.00001 40 0 b 1\nrun 10\n"
    ));
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "LML-PHYS-007")
        .unwrap();
    assert_eq!(diagnostic.span.start_line, 4);
    let original = diagnostic
        .evidence
        .iter()
        .find(|e| e.subject == "fix bonds effective order")
        .unwrap();
    let current = diagnostic
        .evidence
        .iter()
        .find(|e| e.subject == "fix bonds")
        .unwrap();
    assert_eq!(original.span.start_line, 1);
    assert_eq!(current.span.start_line, 3);
}

#[test]
fn uncertain_constraint_order_stays_unknown_without_rejecting_legacy() {
    for text in [
        "fix bonds left shake 0.0001 20 0 b 1\nfix bath right langevin 300 300 0.1 123\nrun 10\n",
        "fix bonds all rattle 0.0001 20 0 b 1\nfix bath all custom/plugin\nfix step all nve\nrun 10\n",
        "fix momentum all momentum 1 linear 1 1 1\nfix bonds all rattle 0.0001 20 0 b 1\nrun 10\n",
    ] {
        let result = check(text);
        let order_findings: Vec<_> = result
            .physical_findings
            .iter()
            .filter(|f| matches!(f.rule_id, "LML-PHYS-007" | "LML-PHYS-008"))
            .collect();
        assert!(!order_findings.is_empty());
        assert!(order_findings.iter().all(|f| f.outcome == Outcome::Unknown));
    }
    warning(
        "fix bonds all shake 0.0001 20 0 b 1\ninclude state.in\nfix bath all langevin 300 300 0.1 123\nrun 10\n",
        "LML-PHYS-007",
        false,
    );
}

#[test]
fn constraint_order_tracks_clear_aliases_and_managed_cleanup() {
    warning(
        "fix bonds all shake/omp 0.0001 20 0 b 1\nfix bath all langevin/omp 300 300 0.1 123\nrun 10\n",
        "LML-PHYS-007",
        true,
    );
    let result = check(
        "include state.in\nclear\nfix bath all nvt temp 300 300 0.1\nfix bonds all rattle 0.0001 20 0 b 1\nrun 10\n",
    );
    let finding = result
        .physical_findings
        .iter()
        .find(|f| f.rule_id == "LML-PHYS-008")
        .unwrap();
    assert_eq!(finding.outcome, Outcome::Satisfied);

    let text = "@scope {\n@fix step all nve\n@scope {\n@fix bonds all shake 0.0001 20 0 b 1\n@fix bath all langevin 300 300 0.1 123\n@drop bonds\n@fix bonds all shake 0.0001 20 0 b 1\nrun 10\n}\nrun 10\n}\n";
    let result = compile_with_options(
        &Source::new("input.lamlet", text.as_bytes().to_vec()),
        &Options {
            language: Language::Lamlet,
            strict: true,
            ..Options::default()
        },
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(!result.diagnostics.iter().any(|d| d.code == "LML-PHYS-007"));
    let findings: Vec<_> = result
        .physical_findings
        .iter()
        .filter(|f| f.rule_id == "LML-PHYS-007")
        .collect();
    assert_eq!(findings.len(), 1);
    assert!(findings.iter().all(|f| f.outcome == Outcome::Satisfied));
}

#[test]
fn barostat_axes_respect_periodicity_and_two_dimensional_shorthand() {
    warning(
        "boundary p p f\nfix a all npt temp 300 300 0.1 iso 0 0 1\nrun 10\n",
        "LML-PHYS-004",
        true,
    );
    for text in [
        "boundary p p f\nfix a all npt temp 300 300 0.1 x 0 0 1 y 0 0 1\nrun 10\n",
        "dimension 2\nfix a all npt temp 300 300 0.1 iso 0 0 1\nrun 10\n",
        "boundary f p p\nfix a all nph xy 0 0 1\nrun 10\n",
        "include state.in\nfix a all nph iso 0 0 1\nrun 10\n",
        "boundary p p f\nfix a all nph custom_keyword z\nrun 10\n",
    ] {
        warning(text, "LML-PHYS-004", false);
    }
    warning(
        "boundary p f p\nfix a all nph xy 0 0 1\nrun 10\n",
        "LML-PHYS-004",
        true,
    );
}

#[test]
fn two_dimensional_z_constraint_is_checked_at_box_creation() {
    warning(
        "dimension 2\nboundary p p f\nregion box block 0 1 0 1 -0.1 0.1\ncreate_box 1 box\n",
        "LML-PHYS-005",
        true,
    );
    warning(
        "dimension 2\nboundary p p p\nregion box block 0 1 0 1 -0.1 0.1\ncreate_box 1 box\n",
        "LML-PHYS-005",
        false,
    );
    warning(
        "dimension 2\ninclude setup.in\nread_data atoms.data\n",
        "LML-PHYS-005",
        false,
    );
}

#[test]
fn thermo_reset_is_advisory_and_points_to_the_lost_settings() {
    let text = "thermo_modify norm no\nthermo_style custom step temp\n";
    warning(text, "LML-LINT-001", true);
    let result = check(text);
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "LML-LINT-001")
        .unwrap();
    assert_eq!(diagnostic.span.start_line, 2);
    assert_eq!(
        diagnostic
            .evidence
            .iter()
            .find(|e| e.subject == "thermo_modify")
            .unwrap()
            .span
            .start_line,
        1
    );
    warning(
        "thermo_style custom step temp\nthermo_modify norm no\n",
        "LML-LINT-001",
        false,
    );
}

#[test]
fn literal_seed_and_region_counts_are_narrow_checks() {
    warning("velocity all create 300 0\n", "LML-LINT-002", true);
    warning(
        "fix random all langevin 300 300 0.1 -2\n",
        "LML-LINT-002",
        true,
    );
    for text in [
        "velocity all create 300 1234\n",
        "velocity all create 300 ${seed}\n",
        "fix random all custom 0\n",
    ] {
        warning(text, "LML-LINT-002", false);
    }
    warning("region combined union 1 left\n", "LML-LINT-003", true);
    warning(
        "region combined intersect 2 left right\n",
        "LML-LINT-003",
        false,
    );
    warning(
        "region combined union ${count} left right\n",
        "LML-LINT-003",
        false,
    );
}

#[test]
fn velocity_replacement_is_reported_without_assuming_the_authors_intent() {
    warning(
        "velocity projectile create 300 1234\nvelocity projectile set 0 0 20 units box\n",
        "LML-LINT-004",
        true,
    );
    for text in [
        "velocity projectile create 300 1234\nvelocity projectile set 0 0 20 sum yes units box\n",
        "velocity projectile set 0 0 20 units box\n",
        "velocity projectile create 300 1234\ngroup projectile clear\nvelocity projectile set 0 0 20 units box\n",
        "velocity projectile create 300 1234\ninclude state.in\nvelocity projectile set 0 0 20 units box\n",
        "velocity projectile create 300 1234\nvelocity projectile set NULL NULL NULL\n",
        "velocity all create 300 1234\nvelocity projectile create 0 4321\nvelocity projectile set 0 0 20\n",
        "velocity all create 300 1234\ncreate_atoms 1 single 0 0 0\nvelocity all set 0 0 20\n",
    ] {
        warning(text, "LML-LINT-004", false);
    }
}

#[test]
fn langevin_absence_claim_requires_a_complete_known_inventory() {
    warning(
        "fix a all langevin 300 300 0.1 123\nrun 10\n",
        "LML-PHYS-006",
        true,
    );
    for text in [
        "fix a all langevin 300 300 0.1 123\nfix b all nve\nrun 10\n",
        "include setup.in\nfix a all langevin 300 300 0.1 123\nrun 10\n",
        "fix a all langevin 300 300 0.1 123\nfix b all custom/integrator\nrun 10\n",
        "fix a all langevin 300 300 0.1 123\nrun 0\n",
    ] {
        warning(text, "LML-PHYS-006", false);
    }
}

#[test]
fn manual_index_is_queryable_and_every_rule_has_a_pinned_source_page() {
    let nvt = lamlet::documentation::query(Some("fix/nvt"));
    assert_eq!(nvt["revision"], "0165101ce505734e8fb249694b22e295a7ab4182");
    assert_eq!(nvt["page_count"], 1001);
    assert!(
        nvt["pages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["path"] == "fix_nh.rst")
    );
    for rule in lamlet::lint::RULES.iter().chain(lamlet::semantics::RULES) {
        assert!(!rule.autofix);
        assert!(!rule.limitations.is_empty());
        for reference in rule.references {
            let name = reference
                .strip_prefix("https://docs.lammps.org/")
                .unwrap()
                .trim_end_matches(".html");
            let found = lamlet::documentation::query(Some(name));
            assert!(
                found["pages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["url"] == *reference),
                "{reference}"
            );
        }
    }
}
