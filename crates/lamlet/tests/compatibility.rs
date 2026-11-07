use lamlet::{
    Language, compile,
    source::{Source, Span},
    syntax::logical_lines,
};

const GOLDEN: &[u8] = b"# legacy\r\n\r\n  units metal # comment\r\nvariable dt equal 0.002\r\ntimestep ${dt}\r\nprint '$x $(1+2) # not comment'\r\nprint \"quoted &\r\n continued\"\r\nprint \"\"\"\r\n@let x = 1\r\n@{invalid}\r\n&\r\n\"\"\"\r\nplugin_command arg &\r\n    next_arg # comment\r\n# final";

#[test]
fn identity_golden_both_languages() {
    for language in [Language::Lammps, Language::Lamlet] {
        let source = Source::new("identity.in", GOLDEN.to_vec());
        let result = compile(&source, language);
        assert_eq!(result.output, GOLDEN);
        assert!(!result.has_errors());
    }
}

#[test]
fn legacy_preserves_non_utf8_and_reserved_syntax() {
    let bytes = b"# \xff\n@let invalid\ntimestep @{reserved}\n";
    assert_eq!(
        compile(&Source::new("old.in", bytes.to_vec()), Language::Lammps).output,
        bytes
    );
}

#[test]
fn logical_boundaries_follow_lammps_not_shell() {
    let bytes = b"print \"a &\n b\"\nprint \"\"\"\n&\n@let x = 2\n\"\"\"\nrun 0 & # stops continuation\nprint end\n";
    let lines = logical_lines(bytes);
    let text: Vec<_> = lines
        .iter()
        .map(|l| &bytes[l.span.start..l.span.end])
        .collect();
    assert_eq!(
        text,
        vec![
            b"print \"a &\n b\"\n".as_slice(),
            b"print \"\"\"\n&\n@let x = 2\n\"\"\"\n",
            b"run 0 & # stops continuation\n",
            b"print end\n"
        ]
    );
    assert_eq!(
        lines
            .iter()
            .find(|l| bytes[l.span.start..l.span.end].starts_with(b"print \"a"))
            .unwrap()
            .continuations
            .len(),
        1
    );
}

#[test]
fn spans_use_scalar_columns_and_half_open_bytes() {
    let source = Source::new("unicode.in", "# λ\r\nrun 0".as_bytes().to_vec());
    assert_eq!(source.location(4), (1, 4));
    let span = source.located(Span::new(6, 11));
    assert_eq!(
        (
            span.start_line,
            span.start_column,
            span.end_line,
            span.end_column
        ),
        (2, 1, 2, 6)
    );
}
