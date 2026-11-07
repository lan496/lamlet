use lamlet::{Compilation, Language, Options, compile, compile_with_options, source::Source};

fn build(text: &str) -> Compilation {
    compile(
        &Source::new("modules.lamlet", text.as_bytes().to_vec()),
        Language::Lamlet,
    )
}
fn error(text: &str, code: &str) {
    let c = build(text);
    assert!(
        c.diagnostics.iter().any(|d| d.code == code),
        "{text}: {:?}",
        c.diagnostics
    );
    assert!(c.output.is_empty());
    assert!(c.mappings.is_empty());
}

#[test]
fn typed_calls_are_lexical_and_preserve_definition_source_maps() {
    let text = "@module time {\n@fn set(dt: Time) requires [] ensures [] {\nunits metal\ntimestep @{dt}\n}\n}\n@call time.set(2 fs)\n@call time.set(0.5 ps)\n";
    let c = build(text);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(
        c.output,
        b"units metal\ntimestep 0.002\nunits metal\ntimestep 0.5\n"
    );
    let mappings: Vec<_> = c
        .mappings
        .iter()
        .filter(|m| m.source.start_line == 4)
        .collect();
    assert_eq!(mappings.len(), 2);
    assert!(
        mappings
            .iter()
            .all(|m| m.expansion.iter().any(|e| e.variable == "@call time.set"))
    );
    error(&text.replace("2 fs", "300 K"), "E_TYPE_MISMATCH");
    error(&text.replace("2 fs", "2"), "E_TYPE_MISMATCH");
    error(&text.replace("2 fs", ""), "E_ARGUMENT_COUNT");
    error(
        "@let hidden = 2 fs\n@module m {\n@fn f() requires [] ensures [] {\ntimestep @{hidden}\n}\n}\n@call m.f()\n",
        "E_UNBOUND_NAME",
    );
}

const SYSTEM: &str = "units metal\nregion box block 0 1 0 1 0 1\ncreate_box 1 box\ncreate_atoms 1 box\npair_style lj/cut 2.5\npair_coeff * * 1 1\n";

#[test]
fn contracts_compose_with_cleanup_and_invalidation() {
    let module = "@module md {\n@fn sample(n: Int) requires [AtomsDefined, ModelDefined] ensures [AtomsDefined, ModelDefined] {\n@fix integrate all nve\nrun @{n}\n}\n}\n";
    let text = format!("{module}{SYSTEM}@call md.sample(4)\n@call md.sample(2)\n");
    let c = build(&text);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(
        String::from_utf8(c.output).unwrap(),
        format!(
            "{SYSTEM}fix integrate all nve\nrun 4\nunfix integrate\nfix integrate all nve\nrun 2\nunfix integrate\n"
        )
    );
    error(&format!("{module}@call md.sample(1)\n"), "E_STATE_CONTRACT");
    error(
        &format!("{module}{SYSTEM}custom_command\n@call md.sample(1)\n"),
        "E_STATE_CONTRACT",
    );
    error(
        &format!("{module}{SYSTEM}clear\n@call md.sample(1)\n"),
        "E_STATE_CONTRACT",
    );
    error(
        &format!("{module}{SYSTEM}pair_style lj/cut 3\n@call md.sample(1)\n"),
        "E_STATE_CONTRACT",
    );
    // Local integrators disappear before the postcondition is checked.
    error(
        &format!(
            "{}{SYSTEM}@call md.sample(1)\n",
            module.replace(
                "ensures [AtomsDefined, ModelDefined]",
                "ensures [IntegratorConfigured]"
            )
        ),
        "E_STATE_CONTRACT",
    );
}

#[test]
fn forward_calls_are_bounded_and_definitions_are_unique() {
    let text = "@call a.f()\n@module a {\n@fn f() requires [] ensures [] {\n@call b.g()\n}\n}\n@module b {\n@fn g() requires [] ensures [] {\nunits real\n}\n}\n";
    let c = build(text);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(c.output, b"units real\n");
    error(
        &text.replace("units real", "@call a.f()"),
        "E_MODULE_RECURSION",
    );
    error(&text.replace("b.g()", "b.missing()"), "E_MODULE_CALL");
    error(&format!("{text}@module a {{\n}}\n"), "E_MODULE_DEFINITION");
    error(
        "@fn f() requires [] ensures [] {\n}\n",
        "E_MODULE_DEFINITION",
    );
    error("@scope {\n@module a {\n}\n}\n", "E_MODULE_DEFINITION");
    error("@module a {\nunits real\n}\n", "E_MODULE_DEFINITION");
}

#[test]
fn unsupported_effects_are_rejected_only_at_new_boundaries() {
    for command in [
        "include external.in",
        "jump SELF done",
        "shell touch hidden",
        "plugin load style.so",
        "if 1 then 'clear'",
        "run 1 every 1 'clear'",
        "fix f all custom/style",
        "pair_style hybrid lj/cut 3",
        "print '${runtime}'",
    ] {
        let text = format!(
            "@module m {{\n@fn f() requires [] ensures [] {{\n{command}\n}}\n}}\n@call m.f()\n"
        );
        error(&text, "E_UNVERIFIED_EFFECT");
        let legacy = compile(
            &Source::new("legacy.in", format!("{command}\n").into_bytes()),
            Language::Lammps,
        );
        assert!(!legacy.has_errors(), "{command}: {:?}", legacy.diagnostics);
    }
    let source = Source::new("limit.lamlet", b"@module m {\n@fn f() requires [] ensures [] {\nunits metal\nunits real\n}\n}\n@call m.f()\n".to_vec());
    let c = compile_with_options(
        &source,
        &Options {
            language: Language::Lamlet,
            max_expanded_lines: 1,
            ..Options::default()
        },
    );
    assert!(c.diagnostics.iter().any(|d| d.code == "E_EXPANSION_LIMIT"));
}
