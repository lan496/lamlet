use lamlet::{
    Compilation, Language, Options, compile, compile_with_options,
    expr::{self, Scope, Value},
    source::Source,
    units::{self, Dimension},
};

fn build(text: &str) -> Compilation {
    compile(
        &Source::new("test.lamlet", text.as_bytes().to_vec()),
        Language::Lamlet,
    )
}
fn output(text: &str) -> String {
    let result = build(text);
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    String::from_utf8(result.output).unwrap()
}
fn has_error(text: &str, code: &str) {
    let result = build(text);
    assert!(
        result.diagnostics.iter().any(|d| d.code == code),
        "{text}: {:?}",
        result.diagnostics
    );
    assert!(result.output.is_empty());
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(1e-300),
        "{actual} != {expected}"
    );
}

#[test]
fn basic_units_golden() {
    assert_eq!(
        output("@let dt = 2 fs\nunits metal\ntimestep @{dt}\n"),
        "units metal\ntimestep 0.002\n"
    );
    assert_eq!(
        output("units real\ntimestep @{0.1 ps}\n"),
        "units real\ntimestep 100\n"
    );
    assert_eq!(
        output("units metal\nfix t all nvt temp @{300 K} @{300 K} @{100 fs}\n"),
        "units metal\nfix t all nvt temp 300 300 0.1\n"
    );
}

#[test]
fn loops_scopes_and_order_golden() {
    let script = "@let n = 1\n@for t in [300 K, 400 K] { # outer\n  @let n = 2\n  @for i in [0, 1] {\n    run @{n + i}\n  }\n}\n@for empty in [] {\n  run @{missing}\n}\nrun @{n}\n";
    assert_eq!(
        output(script),
        "    run 2\n    run 3\n    run 2\n    run 3\nrun 1\n"
    );
    has_error(
        "@for i in [1] {\n@let local = 2\n}\nrun @{local}\n",
        "E_UNBOUND_NAME",
    );
    has_error("@let a = 1\n@let a = 2\n", "E_DUPLICATE_BINDING");
    has_error("@let a = b\n@let b = 1\n", "E_UNBOUND_NAME");
    has_error("@for i in [1, true] {\n}\n", "E_TYPE_MISMATCH");
    has_error("@for i in [1 fs, 1 K] {\n}\n", "E_TYPE_MISMATCH");
    assert_eq!(
        output("@for n in [1, 2.5] {\nplugin @{n * 2}\n}\n"),
        "plugin 2\nplugin 5\n"
    );
}

#[test]
fn precedence_type_and_dimension_algebra() {
    assert_eq!(
        output("plugin @{-9223372036854775808}\n"),
        "plugin -9223372036854775808\n"
    );
    has_error("@let bad = -(-9223372036854775808)\n", "E_NUMERIC_OVERFLOW");
    assert_eq!(
        output("plugin @{1 + 2 * 3} @{-(3 + 4) / 2} @{true}\n"),
        "plugin 7 -3.5 true\n"
    );
    assert_eq!(
        output("plugin @{(4 fs * 2 fs) / (2 fs * 2 fs)}\n"),
        "plugin 2\n"
    );
    has_error("@let bad = 2 fs + 300 K\n", "E_DIMENSION_MISMATCH");
    has_error("@let bad = 2 + 1 fs\n", "E_DIMENSION_MISMATCH");
    has_error("@let bad = true * 2\n", "E_TYPE_MISMATCH");
    has_error("@let bad = 1 / (2 - 2)\n", "E_DIVISION_BY_ZERO");
    has_error("@let bad = 9223372036854775807 + 1\n", "E_NUMERIC_OVERFLOW");
    has_error("@let bad = 1e309\n", "E_NUMERIC_OVERFLOW");
    has_error("@let bad = 1e308 * 1e308\n", "E_NUMERIC_OVERFLOW");
}

#[test]
fn centralized_constants_and_derived_units() {
    for (symbol, metal, real) in [
        ("fs", 0.002, 2.0),
        ("ps", 2.0, 2000.0),
        ("ns", 2000.0, 2e6),
        ("s", 2e12, 2e15),
        ("angstrom", 2.0, 2.0),
        ("nm", 20.0, 20.0),
        ("m", 2e10, 2e10),
        ("K", 2.0, 2.0),
        ("Pa", 0.00002, 0.000019738465334320256),
        ("bar", 2.0, 1.9738465334320256),
        ("atm", 2.0265, 2.0),
        ("eV", 2.0, 46.12109566123806),
        ("kcal/mol", 0.08672820848360188, 2.0),
    ] {
        for (style, expected) in [("metal", metal), ("real", real)] {
            let text = output(&format!("units {style}\nplugin @{{2 {symbol}}}\n"));
            let n = text
                .lines()
                .find_map(|s| s.strip_prefix("plugin "))
                .unwrap()
                .parse::<f64>()
                .unwrap();
            near(n, expected);
        }
    }
    let expr = expr::parse("1 eV / (1 angstrom * 1 angstrom * 1 angstrom)", 0).unwrap();
    assert!(matches!(
        expr::evaluate(&expr, &[Scope::new()]).unwrap(),
        Value::Quantity(_, Dimension::PRESSURE)
    ));
    near(units::EV_PER_KCAL_MOL, 0.04336410424180094);
    let (dp, bar) = units::unit("bar").unwrap();
    near(
        bar / units::target_scale("real", dp).unwrap(),
        0.9869232667160128,
    );
}

#[test]
fn syntax_errors_never_pass_through() {
    for text in [
        "@wat x\n",
        "@let x =\n",
        "@let bad-name = 1\n",
        "@for n in [1] {\n",
        "run @{1\n",
        "run prefix@{1}\n",
        "run @{1}suffix\n",
        "@{1} arg\n",
        "run @{[1,2]}\n",
        "run @{sin(2)}\n",
        "run @{2fs}\n",
        "@let x = 20 C\n",
    ] {
        assert!(build(text).has_errors(), "accepted {text}");
    }
}

#[test]
fn comments_quotes_and_logical_commands_shield_extensions() {
    let script = "# @let x = 2 @{bad}\nprint '@{bad}'\nprint \"@unchecked{bad}\"\nprint \"\"\"\n@let x = 2\n@{bad}\n}\n\"\"\"\nprint hello &\n  @let x = 2\n";
    assert_eq!(output(script), script);
    assert_eq!(
        output("@let x = 2 # comment\nrun @{x} # @{missing}\n"),
        "run 2 # @{missing}\n"
    );
}

#[test]
fn directive_like_continuation_is_legacy_text() {
    // Reserved directive names at a continuation's start are not interpolations.
    let source = Source::new("x.in", b"print x &\n @let x = 3\n".to_vec());
    assert!(!compile(&source, Language::Lammps).has_errors());
    assert_eq!(
        output("print x &\n @let x = 3\n"),
        "print x &\n @let x = 3\n"
    );
    assert_eq!(output("print 'λ\n"), "print 'λ\n");
}

#[test]
fn unit_state_and_unchecked_safety() {
    for style in ["lj", "si", "cgs", "electron", "micro", "nano"] {
        has_error(
            &format!("units {style}\ntimestep @{{2 fs}}\n"),
            "E_UNSUPPORTED_UNITS",
        );
    }
    has_error("timestep @{2 fs}\n", "E_UNSUPPORTED_UNITS");
    has_error("plugin @unchecked{2 fs}\n", "E_UNSUPPORTED_UNITS");
    let result = build("units metal\nplugin @unchecked{2 fs}\n");
    assert!(!result.has_errors());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "W_UNCHECKED_INTERPOLATION")
    );
}

#[test]
fn expansion_limit_counts_only_expansion_lines() {
    let source = Source::new("test.lamlet", b"@for i in [1,2] {\nrun 0\n}\n".to_vec());
    let opts = Options {
        language: Language::Lamlet,
        max_expanded_lines: 1,
        ..Options::default()
    };
    let result = compile_with_options(&source, &opts);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "E_EXPANSION_LIMIT")
    );
    let source = Source::new("big.lamlet", "run 0\n".repeat(100).into_bytes());
    assert_eq!(compile_with_options(&source, &opts).output, source.bytes);
}

#[test]
fn interpolation_spans_refer_to_original_continuations() {
    assert_eq!(
        output("units metal\ntimestep @{1 fs + &\n 1 fs}\n"),
        "units metal\ntimestep 0.002\n"
    );
    let result = build("@let a = &\n missing\n");
    let d = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E_UNBOUND_NAME")
        .unwrap();
    assert_eq!((d.span.start_line, d.span.start_column), (2, 2));
}
