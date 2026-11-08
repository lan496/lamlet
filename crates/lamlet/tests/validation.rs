use lamlet::{Compilation, Language, Options, compile_with_options, source::Source};

fn check(text: &str, strict: bool) -> Compilation {
    compile_with_options(
        &Source::new("test.lamlet", text.as_bytes().to_vec()),
        &Options {
            language: Language::Lamlet,
            strict,
            ..Options::default()
        },
    )
}
fn error(text: &str, code: &str) -> Compilation {
    let result = check(text, false);
    assert!(result.has_errors(), "accepted {text}");
    assert!(
        result.diagnostics.iter().any(|d| d.code == code),
        "{:?}",
        result.diagnostics
    );
    result
}
fn accepted(text: &str, strict: bool) -> Compilation {
    let result = check(text, strict);
    assert!(!result.has_errors(), "{text}: {:?}", result.diagnostics);
    result
}

#[test]
fn dimension_errors_have_exact_original_ranges() {
    let text = "units metal\ntimestep @{300 K}\n";
    let result = error(text, "E_DIMENSION_MISMATCH");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E_DIMENSION_MISMATCH")
        .unwrap();
    assert_eq!(
        &text[diagnostic.span.start_byte..diagnostic.span.end_byte],
        "300 K"
    );
    assert_eq!(
        (diagnostic.span.start_line, diagnostic.span.start_column),
        (2, 12)
    );
    assert_eq!(diagnostic.expected.as_deref(), Some("Time"));
    assert_eq!(diagnostic.actual.as_deref(), Some("Temperature"));
    error(
        "units metal\nfix t all nvt temp @{2 fs} 300 0.1\n",
        "E_DIMENSION_MISMATCH",
    );
    error(
        "units metal\nfix t all npt iso @{2 K} 1 0.1\n",
        "E_DIMENSION_MISMATCH",
    );
}

#[test]
fn refinements_numeric_types_and_bare_units() {
    for text in [
        "timestep @{0}\n",
        "timestep @{-1}\n",
        "run -1\n",
        "fix t all nvt temp 300 300 -1\n",
    ] {
        error(text, "E_REFINEMENT");
    }
    for text in [
        "run @{true}\n",
        "run @{1.0}\n",
        "run 1.5\n",
        "timestep @{false}\n",
    ] {
        error(text, "E_TYPE_MISMATCH");
    }
    error("timestep 1e999\n", "E_NUMERIC_OVERFLOW");
    accepted("timestep 0.005\nrun 0\n", true);
    accepted("units metal\ntimestep @{2}\n", true);
    accepted(
        "units real\nfix t all npt temp @{300 K} @{400 K} @{100 fs} iso @{1 bar} @{1 atm} @{1 ps}\n",
        true,
    );
}

#[test]
fn unknown_legacy_commands_are_accepted_in_both_modes() {
    let text = "units si\ntimestep ${dt}\nplugin_command opaque args\nfix f all plugin/style ${param}\ninclude other.in\nif \"$x > 1\" then \"run 1\"\njump SELF label\nnext i\n";
    for strict in [false, true] {
        let result = accepted(text, strict);
        assert_eq!(result.output, text.as_bytes());
        assert!(
            result
                .report(&Source::new("test.lamlet", vec![]))
                .summary
                .unknowns
                > 0
        );
    }
}

#[test]
fn definite_and_uncertain_ordering_are_distinct() {
    for create in [
        "create_box 1 region\n",
        "read_data nonexistent.data\n",
        "read_restart nonexistent.restart\n",
    ] {
        error(&format!("{create}atom_style atomic\n"), "E_COMMAND_ORDER");
        error(&format!("{create}units metal\n"), "E_COMMAND_ORDER");
    }
    for barrier in [
        "plugin x\n",
        "include unseen.in\n",
        "if '1' then 'clear'\n",
        "jump SELF again\n",
        "timestep ${dt}\n",
    ] {
        let result = accepted(
            &format!("create_box 1 region\n{barrier}atom_style full\n"),
            true,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "U_COMMAND_ORDER")
        );
    }
    error("plugin x\ntimestep @{-1}\n", "E_REFINEMENT");
}

#[test]
fn id_tracking_and_groups_respect_unknown_state() {
    error("unfix absent\n", "E_UNKNOWN_ID");
    error("uncompute absent\n", "E_UNKNOWN_ID");
    error("fix f missing nve\n", "E_UNKNOWN_ID");
    accepted("group g empty\nfix f g nve\nunfix f\n", true);
    error(
        "group g empty\ngroup g delete\nfix f g nve\n",
        "E_UNKNOWN_ID",
    );
    accepted("fix f all nve\nfix f all nve\nunfix f\n", true);
    error("fix f all nve\nunfix f\nunfix f\n", "E_UNKNOWN_ID");
    accepted("compute c all any/style\nuncompute c\n", true);
    accepted("uncompute thermo_temp\n", true);
    accepted("fix f all nvt temp 300 300 1\nuncompute f_temp\n", true);
    for text in [
        "plugin x\nunfix absent\n",
        "read_data data\nfix f from_data nve\n",
        "fix f all plugin\nunfix other\n",
        "read_restart data\nuncompute absent\n",
    ] {
        assert!(
            accepted(text, true)
                .diagnostics
                .iter()
                .any(|d| d.code == "U_ID_STATE")
        );
    }
    accepted("fix @{1} all nve\nunfix 1\n", false);
}

#[test]
fn legacy_fragments_allow_external_ids_but_preserve_proven_absence() {
    for strict in [false, true] {
        for text in [
            "fix f external bond/create 1 2 3 3.0 1\n",
            "unfix external\n",
            "uncompute external\n",
        ] {
            let source = Source::new("fragment.lammps", text.as_bytes().to_vec());
            let result = compile_with_options(
                &source,
                &Options {
                    language: Language::Lammps,
                    strict,
                    ..Options::default()
                },
            );
            assert!(!result.has_errors(), "{:?}", result.diagnostics);
            assert_eq!(result.output, text.as_bytes());
            assert!(result.diagnostics.iter().any(|d| d.code == "U_ID_STATE"));
        }
        for text in [
            "clear\nfix f external nve\n",
            "group external empty\ngroup external delete\nfix f external nve\n",
            "fix f all nve\nunfix f\nunfix f\n",
        ] {
            let result = compile_with_options(
                &Source::new("in.test", text.as_bytes().to_vec()),
                &Options {
                    language: Language::Lammps,
                    strict,
                    ..Options::default()
                },
            );
            assert!(
                result.diagnostics.iter().any(|d| d.code == "E_UNKNOWN_ID"),
                "{text}: {:?}",
                result.diagnostics
            );
        }
    }
}

#[test]
fn strict_only_requires_proof_for_new_interpolations() {
    let script = "units metal\nplugin @{2 fs}\n";
    assert!(
        accepted(script, false)
            .diagnostics
            .iter()
            .any(|d| d.code == "W_UNVERIFIED_ARGUMENT")
    );
    assert!(
        check(script, true)
            .diagnostics
            .iter()
            .any(|d| d.code == "E_UNVERIFIED_ARGUMENT")
    );
    let escaped = "units metal\nplugin @unchecked{2 fs}\n";
    assert!(
        accepted(escaped, true)
            .diagnostics
            .iter()
            .any(|d| d.code == "W_UNCHECKED_INTERPOLATION")
    );
    let escaped_type = "units metal\ntimestep @unchecked{300 K}\n";
    assert!(
        accepted(escaped_type, true)
            .diagnostics
            .iter()
            .any(|d| d.code == "W_UNCHECKED_INTERPOLATION")
    );
    error(
        "plugin x\ntimestep @unchecked{2 fs}\n",
        "E_UNSUPPORTED_UNITS",
    );
    accepted("units metal\nfix t all other/style 1 2\n", true);
}

#[test]
fn unknown_fix_options_do_not_invent_keyword_boundaries() {
    // An opaque option can take the literal word 'temp' as its own value.
    accepted("fix t all nvt custom temp 1 2\n", true);
    accepted(
        "fix t all npt temp 300 300 0.1 custom value iso 1 1 0.1\n",
        true,
    );
    error("fix t all nvt temp 300 300\n", "E_ARGUMENT_COUNT");
    error("run\n", "E_ARGUMENT_COUNT");
    error("timestep 1 2\n", "E_ARGUMENT_COUNT");
}

#[test]
fn unrolled_commands_undergo_sequential_state_analysis() {
    accepted(
        "@for i in [1,2] {\nfix f all nve\nrun 0\nunfix f\n}\n",
        true,
    );
    error(
        "fix f all nve\n@for i in [1,2] {\nunfix f\n}\n",
        "E_UNKNOWN_ID",
    );
}

#[test]
fn restart_and_dynamic_units_require_known_conversion() {
    error(
        "units metal\nread_restart missing\ntimestep @{2 fs}\n",
        "E_UNSUPPORTED_UNITS",
    );
    error("units ${style}\ntimestep @{2 fs}\n", "E_UNSUPPORTED_UNITS");
    accepted("units lj\nrun @{10}\n", true);
}
