use lamlet::{Language, Options, compile, compile_with_options, source::Source};

#[test]
fn mixed_newlines_unicode_quoting_and_dollar_forms_round_trip() {
    let fragments = [
        "\n",
        "  # λ @{not_code}\r\n",
        "plugin ${name} $x $(1 + (2*3):%.10g)\n",
        "print 'a \"nested\" quote # $x @{literal}'\r\n",
        "print \"a 'nested' quote # $x @unchecked{literal}\"\n",
        "plugin one &\n  two &\r\n three\n",
        "# comment continued &\n @let ignored = invalid\n",
        "print \"\"\"\n# @let ignored = invalid\n@{also_ignored}\n&\n\"\"\"\n",
    ];
    // A deterministic corpus varies line order without enumerating a Cartesian product.
    for seed in 0..64 {
        let mut text = String::new();
        for n in 0..fragments.len() {
            text.push_str(fragments[(n + seed) % fragments.len()]);
        }
        if seed % 2 == 0 {
            text.push_str("# no final newline");
        }
        let source = Source::new("corpus.lamlet", text.into_bytes());
        for language in [Language::Lammps, Language::Lamlet] {
            let result = compile(&source, language);
            assert!(!result.has_errors(), "{:?}", result.diagnostics);
            assert_eq!(result.output, source.bytes);
        }
    }
}

#[test]
fn extension_parsing_is_separate_from_runtime_lammps_expressions() {
    let text = "@let n = 2\nplugin ${runtime} $x $(1 + 2) @{n}\n";
    let result = compile(
        &Source::new("x.lamlet", text.as_bytes().to_vec()),
        Language::Lamlet,
    );
    assert!(!result.has_errors());
    assert_eq!(result.output, b"plugin ${runtime} $x $(1 + 2) 2\n");
}

#[test]
fn syntax_rejects_injection_and_preserves_quoted_literal_delimiters() {
    for expression in [
        "1\nrun 100",
        "1; run 100",
        "\"1 2\"",
        "${runtime}",
        "$(1+2)",
        "1 # comment",
        "1}suffix",
        "1,2",
    ] {
        let text = format!("run @{{{expression}}}\n");
        let result = compile(
            &Source::new("x.lamlet", text.into_bytes()),
            Language::Lamlet,
        );
        assert!(result.has_errors(), "accepted {expression}");
        assert!(result.output.is_empty());
    }
    let text = "print 'unclosed @{literal}\n";
    let result = compile(
        &Source::new("x.lamlet", text.as_bytes().to_vec()),
        Language::Lamlet,
    );
    assert!(!result.has_errors());
    assert_eq!(result.output, text.as_bytes());
}

#[test]
fn pathological_expression_and_empty_loop_work_are_bounded() {
    for text in [
        format!("@let n = {}1{}\n", "(".repeat(150), ")".repeat(150)),
        format!("@let n = {}1\n", "1 + ".repeat(300)),
    ] {
        let result = compile(
            &Source::new("x.lamlet", text.into_bytes()),
            Language::Lamlet,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "E_RESOURCE_LIMIT")
        );
    }
    let mut text = String::new();
    for n in 0..12 {
        text.push_str(&format!("@for n{n} in [0, 1] {{\n"));
    }
    text.push_str(&"}\n".repeat(12));
    let result = compile_with_options(
        &Source::new("empty.lamlet", text.into_bytes()),
        &Options {
            language: Language::Lamlet,
            max_expanded_lines: 1,
            ..Options::default()
        },
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "E_EXPANSION_LIMIT")
    );
}

#[test]
fn crlf_extensions_and_collapsed_continuations_keep_original_maps() {
    let text = "@let dt = 2 fs\r\nunits metal\r\ntimestep @{dt + &\r\n 2 fs} # keep\r\n";
    let source = Source::new("x.lamlet", text.as_bytes().to_vec());
    let result = compile(&source, Language::Lamlet);
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert_eq!(result.output, b"units metal\r\ntimestep 0.004 # keep\r\n");
    assert_eq!(result.mappings.len(), 2);
    let mapping = result
        .mappings
        .iter()
        .find(|m| m.source.start_line == 3)
        .unwrap();
    assert_eq!(mapping.generated_line, 2);
    assert_eq!(mapping.source.end_line, 5);
}

#[test]
fn conversion_must_not_silently_underflow_a_positive_timestep() {
    let text = "units metal\ntimestep @{1e-322 fs}\n";
    let result = compile(
        &Source::new("x.lamlet", text.as_bytes().to_vec()),
        Language::Lamlet,
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "E_NUMERIC_OVERFLOW")
    );
}
