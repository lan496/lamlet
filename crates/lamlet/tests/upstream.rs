use lamlet::{Language, Options, compile_with_options, source::Source};

#[test]
fn upstream_legacy_timesteps_remain_valid_but_typed_timesteps_are_positive() {
    // LAMMPS test_simple_commands.cpp, TimeStep: both cases are explicitly valid.
    for text in ["timestep 0.0\n", "timestep -0.1\n"] {
        for language in [Language::Lammps, Language::Lamlet] {
            for strict in [false, true] {
                let result = compile_with_options(
                    &Source::new("upstream.in", text.as_bytes().to_vec()),
                    &Options {
                        language,
                        strict,
                        ..Options::default()
                    },
                );
                assert!(!result.has_errors(), "{:?}", result.diagnostics);
                assert_eq!(result.output, text.as_bytes());
            }
        }
    }
    for text in ["timestep @{0}\n", "timestep @{-0.1}\n"] {
        let result = compile_with_options(
            &Source::new("typed.lamlet", text.as_bytes().to_vec()),
            &Options {
                language: Language::Lamlet,
                ..Options::default()
            },
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E_REFINEMENT"));
    }
}

#[test]
fn upstream_negative_cases_distinguish_covered_errors_from_opaque_commands() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/corpus/negative.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let text = case["script"].as_str().unwrap();
        for strict in [false, true] {
            let result = compile_with_options(
                &Source::new(case["id"].as_str().unwrap(), text.as_bytes().to_vec()),
                &Options {
                    strict,
                    ..Options::default()
                },
            );
            if let Some(code) = case["error"].as_str() {
                assert!(result.has_errors(), "{}", case["id"]);
                assert!(
                    result.diagnostics.iter().any(|d| d.code == code),
                    "{:?}",
                    result.diagnostics
                );
                assert!(result.output.is_empty());
            } else {
                assert!(
                    !result.has_errors(),
                    "{}: {:?}",
                    case["id"],
                    result.diagnostics
                );
                assert_eq!(result.output, text.as_bytes());
                assert!(
                    result
                        .diagnostics
                        .iter()
                        .any(|d| d.severity == lamlet::diagnostic::Severity::Unknown)
                );
            }
        }
    }
}

#[test]
fn quoted_runtime_numbers_remain_unknown() {
    for text in ["timestep \"${dt}\"\n", "run \"${steps}\"\n"] {
        let result = compile_with_options(
            &Source::new("runtime.in", text.as_bytes().to_vec()),
            &Options::default(),
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.output, text.as_bytes());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "U_RUNTIME_VALUE")
        );
    }
}
