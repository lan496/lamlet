use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lamlet"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}
fn json_output(output: &Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{:?}", output.stderr);
    assert!(!output.stdout.contains(&0x1b));
    // from_slice rejects trailing non-whitespace, including a second JSON object.
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value.is_object());
    value
}

#[test]
fn deterministic_json_error_golden() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("bad.lamlet"),
        "units metal\ntimestep @{300 K}\n",
    )
    .unwrap();
    let args = ["check", "bad.lamlet", "--format", "json"];
    let first = cli(&args, dir.path());
    let second = cli(&args, dir.path());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(
        json_output(&first, 1),
        json!({
            "schema_version": 1, "status": "error", "input": "bad.lamlet",
            "diagnostics": [{
                "severity": "error", "code": "E_DIMENSION_MISMATCH",
                "message": "Expected Time, but this expression has Temperature",
                "file": "bad.lamlet", "span": { "start_byte": 23, "end_byte": 28, "start_line": 2, "start_column": 12, "end_line": 2, "end_column": 17 },
                "expected": "Time", "actual": "Temperature", "notes": [],
                "suggestion": "Use a positive time quantity such as 2 fs"
            }], "summary": { "errors": 1, "warnings": 0, "unknowns": 0 }
        })
    );
}

#[test]
fn language_overrides_and_legacy_identity() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"# \xff\r\n@let n = 1\r\nrun @{n}\r\n";
    fs::write(dir.path().join("old.in"), bytes).unwrap();
    json_output(
        &cli(
            &["build", "old.in", "-o", "copy.in", "--format", "json"],
            dir.path(),
        ),
        0,
    );
    assert_eq!(fs::read(dir.path().join("copy.in")).unwrap(), bytes);
    fs::write(dir.path().join("ext.in"), "@let n = 2\nrun @{n}\n").unwrap();
    json_output(
        &cli(
            &[
                "build",
                "ext.in",
                "-o",
                "compiled.in",
                "--language",
                "lamlet",
                "--format",
                "json",
            ],
            dir.path(),
        ),
        0,
    );
    assert_eq!(
        fs::read(dir.path().join("compiled.in")).unwrap(),
        b"run 2\n"
    );
    fs::write(dir.path().join("old.lamlet"), bytes).unwrap();
    json_output(
        &cli(
            &[
                "build",
                "old.lamlet",
                "-o",
                "forced.in",
                "--language",
                "lammps",
                "--format",
                "json",
            ],
            dir.path(),
        ),
        0,
    );
    assert_eq!(fs::read(dir.path().join("forced.in")).unwrap(), bytes);
}

#[test]
fn failed_build_preserves_source_and_existing_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let source = b"units metal\ntimestep @{300 K}\n";
    fs::write(dir.path().join("bad.lamlet"), source).unwrap();
    fs::write(dir.path().join("out.in"), b"original output").unwrap();
    fs::write(dir.path().join("out.map.json"), b"original map").unwrap();
    json_output(
        &cli(
            &[
                "build",
                "bad.lamlet",
                "-o",
                "out.in",
                "--source-map",
                "out.map.json",
                "--format",
                "json",
            ],
            dir.path(),
        ),
        1,
    );
    assert_eq!(fs::read(dir.path().join("bad.lamlet")).unwrap(), source);
    assert_eq!(
        fs::read(dir.path().join("out.in")).unwrap(),
        b"original output"
    );
    assert_eq!(
        fs::read(dir.path().join("out.map.json")).unwrap(),
        b"original map"
    );
    json_output(
        &cli(
            &["build", "bad.lamlet", "-o", "new.in", "--format", "json"],
            dir.path(),
        ),
        1,
    );
    assert!(!dir.path().join("new.in").exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn source_maps_cover_every_generated_line_with_loop_context() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("loop.lamlet"),
        "# heading\n@for n in [0, 1] {\n@for i in [0] {\nrun @{n + i}\n}\n}\n",
    )
    .unwrap();
    json_output(
        &cli(
            &[
                "build",
                "loop.lamlet",
                "-o",
                "out.in",
                "--source-map",
                "out.map.json",
                "--format",
                "json",
            ],
            dir.path(),
        ),
        0,
    );
    assert_eq!(
        fs::read(dir.path().join("out.in")).unwrap(),
        b"# heading\nrun 0\nrun 1\n"
    );
    let map: Value =
        serde_json::from_slice(&fs::read(dir.path().join("out.map.json")).unwrap()).unwrap();
    assert_eq!(map["format_version"], 1);
    assert_eq!(map["original_source"], "loop.lamlet");
    assert_eq!(map["output"], "out.in");
    assert_eq!(map["lines"].as_array().unwrap().len(), 3);
    let run = map["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|line| {
            line["expansion"].as_array().is_some_and(|context| {
                context
                    .iter()
                    .any(|c| c["variable"] == "n" && c["iteration"] == 1)
            })
        })
        .unwrap();
    assert_eq!(run["generated_line"], 3);
    assert_eq!(run["source"]["start_line"], 4);
    assert_eq!(
        run["expansion"],
        json!([{ "variable": "n", "iteration": 1 }, { "variable": "i", "iteration": 0 }])
    );
}

#[test]
fn check_is_read_only_and_matches_build_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = "units metal\nplugin @unchecked{2 fs}\n";
    fs::write(dir.path().join("x.lamlet"), bytes).unwrap();
    let check = json_output(
        &cli(
            &["check", "x.lamlet", "--strict", "--format", "json"],
            dir.path(),
        ),
        0,
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    let build = json_output(
        &cli(
            &[
                "build", "x.lamlet", "-o", "x.in", "--strict", "--format", "json",
            ],
            dir.path(),
        ),
        0,
    );
    assert_eq!(check, build);
    assert_eq!(check["status"], "warning");
    assert!(
        check["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "W_UNCHECKED_INTERPOLATION")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("x.lamlet")).unwrap(),
        bytes
    );
}

#[test]
fn strict_and_expansion_limit_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("x.lamlet"), "units metal\nplugin @{2 fs}\n").unwrap();
    json_output(
        &cli(
            &["check", "x.lamlet", "--strict", "--format", "json"],
            dir.path(),
        ),
        1,
    );
    json_output(
        &cli(&["check", "x.lamlet", "--format", "json"], dir.path()),
        0,
    );
    fs::write(
        dir.path().join("x.lamlet"),
        "@for n in [0, 1] {\nrun @{n}\n}\n",
    )
    .unwrap();
    let value = json_output(
        &cli(
            &[
                "build",
                "x.lamlet",
                "-o",
                "out.in",
                "--max-expanded-lines",
                "1",
                "--format",
                "json",
            ],
            dir.path(),
        ),
        1,
    );
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_EXPANSION_LIMIT")
    );
    assert!(!dir.path().join("out.in").exists());
}

#[test]
fn io_and_cli_errors_produce_one_json_object_and_exit_two() {
    let dir = tempfile::tempdir().unwrap();
    let missing = json_output(
        &cli(&["check", "missing.in", "--format", "json"], dir.path()),
        2,
    );
    assert_eq!(
        missing["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["severity"] == "error")
            .unwrap()["code"],
        "E_IO"
    );
    let bad_cli = json_output(
        &cli(&["check", "--format=json", "--bad-option"], dir.path()),
        2,
    );
    assert_eq!(
        bad_cli["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["severity"] == "error")
            .unwrap()["code"],
        "E_CLI"
    );
    fs::write(dir.path().join("x.in"), "run 0\n").unwrap();
    json_output(
        &cli(
            &["build", "x.in", "-o", "missing/out.in", "--format", "json"],
            dir.path(),
        ),
        2,
    );
    json_output(
        &cli(
            &[
                "build",
                "x.in",
                "-o",
                "out.in",
                "--source-map",
                "missing/out.map.json",
                "--format",
                "json",
            ],
            dir.path(),
        ),
        2,
    );
    assert!(!dir.path().join("out.in").exists());
}

#[test]
fn output_paths_cannot_overwrite_source_or_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let source = "@let n = 0\nrun @{n}\n";
    fs::write(dir.path().join("x.lamlet"), source).unwrap();
    for args in [
        vec!["build", "x.lamlet", "-o", "./x.lamlet", "--format", "json"],
        vec![
            "build",
            "x.lamlet",
            "-o",
            "out.in",
            "--source-map",
            "x.lamlet",
            "--format",
            "json",
        ],
        vec![
            "build",
            "x.lamlet",
            "-o",
            "out.in",
            "--source-map",
            "./out.in",
            "--format",
            "json",
        ],
    ] {
        json_output(&cli(&args, dir.path()), 2);
    }
    assert_eq!(
        fs::read_to_string(dir.path().join("x.lamlet")).unwrap(),
        source
    );
    assert!(!dir.path().join("out.in").exists());
}

#[cfg(unix)]
#[test]
fn source_aliases_are_protected() {
    let dir = tempfile::tempdir().unwrap();
    let source = "@let n = 0\nrun @{n}\n";
    fs::write(dir.path().join("x.lamlet"), source).unwrap();
    std::os::unix::fs::symlink("x.lamlet", dir.path().join("link.in")).unwrap();
    fs::hard_link(dir.path().join("x.lamlet"), dir.path().join("hard.in")).unwrap();
    for alias in ["link.in", "hard.in"] {
        json_output(
            &cli(
                &["build", "x.lamlet", "-o", alias, "--format", "json"],
                dir.path(),
            ),
            2,
        );
    }
    assert_eq!(
        fs::read_to_string(dir.path().join("x.lamlet")).unwrap(),
        source
    );
}

#[test]
fn schema_reports_partial_coverage_types_and_stable_codes() {
    let dir = tempfile::tempdir().unwrap();
    let args = ["schema", "--format", "json"];
    let first = cli(&args, dir.path());
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let schema = json_output(&first, 0);
    assert_eq!(schema["schema_version"], 1);
    assert_eq!(schema["typed_unit_styles"], json!(["metal", "real"]));
    let fix = schema["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "fix/nvt")
        .unwrap();
    assert_eq!(fix["coverage"], "partial");
    let temp = fix["keywords"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["keyword"] == "temp")
        .unwrap();
    assert_eq!(
        temp["operands"],
        json!(["Temperature", "Temperature", "PositiveTime"])
    );
    assert!(
        schema["diagnostic_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_DIMENSION_MISMATCH")
    );
    let query = json_output(
        &cli(&["schema", "fix/nvt", "--format", "json"], dir.path()),
        0,
    );
    assert_eq!(query["commands"], json!([fix]));
    let unknown = json_output(
        &cli(&["schema", "plugin", "--format", "json"], dir.path()),
        0,
    );
    assert_eq!(unknown["coverage"], "opaque");
    assert_eq!(unknown["commands"], json!([]));
}

#[test]
fn shipped_examples_build_without_lammps_or_data_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["legacy.in", "typed.lamlet", "thermostat.lamlet"] {
        fs::copy(root.join("examples").join(name), dir.path().join(name)).unwrap();
        json_output(
            &cli(
                &["build", name, "-o", "out.in", "--format", "json"],
                dir.path(),
            ),
            0,
        );
    }
}

#[test]
#[ignore = "requires LAMMPS; set LAMLET_LAMMPS_BIN to select an executable"]
fn optional_lammps_zero_step_integration() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut failures = vec![];
    for name in ["legacy.in", "typed.lamlet"] {
        fs::copy(root.join("examples").join(name), dir.path().join(name)).unwrap();
        json_output(
            &cli(
                &["build", name, "-o", "out.in", "--format", "json"],
                dir.path(),
            ),
            0,
        );
        let output =
            Command::new(std::env::var_os("LAMLET_LAMMPS_BIN").unwrap_or_else(|| "lmp".into()))
                .args(["-in", "out.in", "-log", "none"])
                .current_dir(dir.path())
                .output()
                .unwrap();
        if !output.status.success() {
            failures.push(format!(
                "{name}: {}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
