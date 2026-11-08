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

const STAGE_SOURCE: &str = "@stage relax(structure: InputFile, result: OutputFile) {\nunits metal\natom_style atomic\nread_data @{structure} nocoeff\npair_style lj/cut 2.5\npair_coeff * * 1 1\nminimize 1e-8 1e-10 10 100\nwrite_data @{result}\n}\n@stage invalid() {\nrun 100\n}\n";
#[test]
fn manual_rules_are_queryable_and_warning_builds_preserve_legacy_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let args = ["schema", "LML-MAN-NEIGHBOR-DELAY", "--format=json"];
    let first = cli(&args, dir.path());
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let schema = json_output(&first, 0);
    assert_eq!(schema["manual_rules"].as_array().unwrap().len(), 1);
    let rule = schema["manual_rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "LML-MAN-NEIGHBOR-DELAY")
        .unwrap();
    assert_eq!(rule["predicate"]["kind"], "neighbor_delay");
    assert_eq!(schema["coverage"], "partial");
    assert_eq!(schema["manual_review"]["complete"], false);
    assert!(
        rule["source"]["url"]
            .as_str()
            .unwrap()
            .contains("/blob/0165101")
    );
    let styles = json_output(
        &cli(&["schema", "pair_style/eam", "--format=json"], dir.path()),
        0,
    );
    assert!(
        styles["manual_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "LML-MAN-PAIR-EAM-MODIFY-SHIFT")
    );
    let human = cli(&["schema", "LML-MAN-NEIGHBOR-DELAY"], dir.path());
    assert!(human.status.success());
    assert!(String::from_utf8_lossy(&human.stdout).contains("LML-MAN-NEIGHBOR-DELAY [warning]"));

    let input = b"# retain formatting\r\nneigh_modify delay 3 every 2\r\n";
    fs::write(dir.path().join("legacy.in"), input).unwrap();
    let args = ["check", "legacy.in", "--strict", "--format=json"];
    let first = cli(&args, dir.path());
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let checked = json_output(&first, 0);
    assert!(checked["diagnostics"].as_array().unwrap().iter().any(|d| {
        d["code"] == "LML-MAN-NEIGHBOR-DELAY"
            && d["severity"] == "warning"
            && d["span"]["start_line"] == 2
    }));
    json_output(
        &cli(
            &[
                "build",
                "legacy.in",
                "--strict",
                "-o",
                "copy.in",
                "--format=json",
            ],
            dir.path(),
        ),
        0,
    );
    assert_eq!(fs::read(dir.path().join("copy.in")).unwrap(), input);
}

fn stage_project(path: &Path) {
    fs::write(path.join("workflow.lamlet"), STAGE_SOURCE).unwrap();
    fs::write(path.join("atoms.data"), "explicit input fixture").unwrap();
    fs::write(path.join("ambient.marker"), "must not be read implicitly").unwrap();
}
fn stage_args(action: &str) -> Vec<&str> {
    vec![
        action,
        "workflow.lamlet",
        "--stage",
        "relax",
        "--arg",
        "structure=path(\"atoms.data\")",
        "--arg",
        "result=output(\"relaxed.data\")",
        "--format=json",
    ]
}

#[test]
fn stage_inspection_checks_and_builds_are_deterministic_and_selective() {
    let dir = tempfile::tempdir().unwrap();
    stage_project(dir.path());
    let first = cli(
        &["inspect", "stages", "workflow.lamlet", "--format=json"],
        dir.path(),
    );
    let second = cli(
        &["inspect", "stages", "workflow.lamlet", "--format=json"],
        dir.path(),
    );
    assert_eq!(first.stdout, second.stdout);
    let definitions = json_output(&first, 0);
    assert!(definitions["stages"].as_array().unwrap().iter().any(|s| {
        s["name"] == "relax"
            && s["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["name"] == "result" && p["direction"] == "output")
    }));
    let checked = json_output(&cli(&stage_args("check"), dir.path()), 0);
    assert_eq!(checked["stage"]["independent_execution"], true);
    assert_eq!(checked["stage"]["resumable"], false);
    assert!(!dir.path().join(".lamlet").exists());
    let built = json_output(&cli(&stage_args("build"), dir.path()), 0);
    assert_eq!(
        built["build"]["working_directory"],
        ".lamlet/build/stages/relax"
    );
    let script = dir.path().join(".lamlet/build/stages/relax/simulation.in");
    assert!(
        fs::read_to_string(&script)
            .unwrap()
            .contains("read_data \"../../../../atoms.data\" nocoeff\n")
    );
    let manifest: Value = serde_json::from_slice(
        &fs::read(script.with_file_name("simulation.in.manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["stage"], built["stage"]);
    assert_eq!(manifest["dependencies"]["unresolved"], json!([]));
    json_output(
        &cli(&["check", "workflow.lamlet", "--format=json"], dir.path()),
        1,
    );
    let invalid = json_output(
        &cli(
            &[
                "check",
                "workflow.lamlet",
                "--stage",
                "invalid",
                "--format=json",
            ],
            dir.path(),
        ),
        1,
    );
    assert!(
        invalid["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_STATE_CONTRACT")
    );
    for argument in [
        "result=output(\"simulation.in\")",
        "result=output(\"simulation.in.manifest.json\")",
        "result=output(\"../escape.data\")",
    ] {
        let mut args = stage_args("build");
        let output_binding = args
            .iter_mut()
            .find(|arg| arg.starts_with("result="))
            .unwrap();
        *output_binding = argument;
        let result = cli(&args, dir.path());
        assert!(!result.status.success());
        serde_json::from_slice::<Value>(&result.stdout).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn stages_run_in_separate_processes_and_working_directories_from_declared_inputs() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    stage_project(dir.path());
    fs::create_dir(dir.path().join("caller")).unwrap();
    let engine = dir.path().join("stage-engine");
    fs::write(&engine, "#!/bin/sh\ntest ! -e ambient.marker || exit 11\ntest -f ../../../../atoms.data || exit 12\ntest -f simulation.in || exit 13\nprintf '%s\\n' \"$$\"\npwd\nprintf 'artifact\\n' > relaxed.data\n").unwrap();
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o755)).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for instance in ["first", "second", "first"] {
        let args = [
            "run",
            "../workflow.lamlet",
            "--stage",
            "relax",
            "--instance",
            instance,
            "--arg",
            "structure=path(\"atoms.data\")",
            "--arg",
            "result=output(\"relaxed.data\")",
            "--lammps-bin",
            "../stage-engine",
            "--format=json",
        ];
        let value = json_output(&cli(&args, &dir.path().join("caller")), 0);
        assert_eq!(
            value["execution"]["working_directory"],
            format!(".lamlet/build/stages/{instance}")
        );
        let directory = dir.path().join(format!(".lamlet/build/stages/{instance}"));
        let stdout = fs::read_to_string(directory.join("simulation.in.stdout")).unwrap();
        ids.insert(stdout.lines().next().unwrap().to_owned());
        assert!(stdout.contains(directory.canonicalize().unwrap().to_str().unwrap()));
        assert_eq!(
            fs::read(directory.join("relaxed.data")).unwrap(),
            b"artifact\n"
        );
    }
    assert_eq!(ids.len(), 3);
    // Relative PATH entries select the caller's engine before changing cwd.
    let directory = dir.path().join(".lamlet/build/stages/path_lookup");
    fs::create_dir_all(&directory).unwrap();
    let shadow = directory.join("stage-engine");
    fs::write(&shadow, "#!/bin/sh\nexit 19\n").unwrap();
    fs::set_permissions(&shadow, fs::Permissions::from_mode(0o755)).unwrap();
    let mut args = stage_args("run");
    args.extend(["--instance", "path_lookup", "--lammps-bin", "stage-engine"]);
    let output = Command::new(env!("CARGO_BIN_EXE_lamlet"))
        .args(args)
        .current_dir(dir.path())
        .env("PATH", ".")
        .output()
        .unwrap();
    json_output(&output, 0);
    assert_eq!(
        fs::read(directory.join("relaxed.data")).unwrap(),
        b"artifact\n"
    );
    assert_eq!(
        fs::read(dir.path().join("atoms.data")).unwrap(),
        b"explicit input fixture"
    );
    assert!(!dir.path().join("relaxed.data").exists());
    let missing_target = json_output(&cli(&stage_args("run"), dir.path()), 2);
    assert!(
        missing_target["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_STAGE_TARGET")
    );
    // A successful process status alone cannot establish an output artifact.
    fs::write(&engine, "#!/bin/sh\nexit 0\n").unwrap();
    let mut args = stage_args("run");
    args.extend([
        "--instance",
        "missing_output",
        "--lammps-bin",
        "./stage-engine",
    ]);
    let missing = json_output(&cli(&args, dir.path()), 3);
    assert_eq!(missing["execution"]["exit_code"], 0);
    assert_eq!(missing["execution"]["success"], false);
    assert!(
        missing["execution"]["observed_outputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |o| o["path"] == ".lamlet/build/stages/missing_output/relaxed.data"
                    && o["exists"] == false
            )
    );
    assert!(
        missing["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_STAGE_OUTPUT")
    );
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
            "physical_findings": [{
                "rule_id": "LML-MAN-UNITS-BEFORE-BOX", "outcome": "satisfied",
                "span": { "start_byte": 0, "end_byte": 12, "start_line": 1, "start_column": 1, "end_line": 2, "end_column": 1 },
                "evidence": [{ "subject": "LAMMPS command", "definition": "units metal", "span": { "start_byte": 0, "end_byte": 12, "start_line": 1, "start_column": 1, "end_line": 2, "end_column": 1 } }],
                "suggested_actions": []
            }],
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
    assert_eq!(query["documentation"]["page_count"], 1001);
    assert!(query["documentation"]["pages"].as_array().unwrap().iter().any(|page| {
        page["path"] == "fix_nh.rst"
            && page["source_url"] == "https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst"
    }));
    assert!(
        query["lint_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| { rule["id"] == "LML-PHYS-001" && rule["autofix"] == false })
    );
    let unknown = json_output(
        &cli(&["schema", "plugin", "--format", "json"], dir.path()),
        0,
    );
    assert_eq!(unknown["coverage"], "opaque");
    assert_eq!(unknown["commands"], json!([]));
    assert!(
        unknown["documentation"]["pages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|page| page["path"] == "plugin.rst")
    );
    let absent = json_output(
        &cli(
            &["schema", "not-a-lammps-command", "--format=json"],
            dir.path(),
        ),
        0,
    );
    assert_eq!(absent["documentation"]["pages"], json!([]));
}

#[test]
fn legacy_lint_json_is_deterministic_and_warnings_do_not_reject_builds() {
    let dir = tempfile::tempdir().unwrap();
    let source = "fix first all nve\nfix second all nvt temp 300 300 0.1\nrun 10\n";
    fs::write(dir.path().join("input.in"), source).unwrap();
    let args = ["check", "input.in", "--strict", "--format=json"];
    let first = cli(&args, dir.path());
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let checked = json_output(&first, 0);
    let diagnostic = checked["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "LML-PHYS-001")
        .unwrap();
    assert_eq!(diagnostic["severity"], "warning");
    assert_eq!(diagnostic["span"]["start_line"], 3);
    assert_eq!(diagnostic["evidence"].as_array().unwrap().len(), 2);
    let built = json_output(
        &cli(
            &[
                "build",
                "input.in",
                "-o",
                "copy.in",
                "--strict",
                "--format=json",
            ],
            dir.path(),
        ),
        0,
    );
    assert_eq!(built["diagnostics"], checked["diagnostics"]);
    assert_eq!(
        fs::read_to_string(dir.path().join("copy.in")).unwrap(),
        source
    );
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

fn resource_project(root: &Path) {
    fs::create_dir(root.join("structures")).unwrap();
    fs::write(root.join("structures/Si.data"), "input data").unwrap();
    fs::write(root.join("lamlet.toml"), "[project]\nname='silicon'\n[paths]\ninput_root='.'\noutput_root='results'\n[build]\ndirectory='.lamlet/build'\n").unwrap();
    fs::write(root.join("simulation.lamlet"), "@let data = path(\"structures/Si.data\")\nread_data @{data}\n@scope {\n@fix integrate all nve\n@dump traj all custom 100 @{output(\"results/traj.lammpstrj\")} id x y z\nrun 0\n}\n").unwrap();
}

#[test]
fn project_check_and_dependency_inspection_are_read_only_and_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    resource_project(dir.path());
    json_output(
        &cli(&["check", "simulation.lamlet", "--format=json"], dir.path()),
        0,
    );
    let args = [
        "inspect",
        "dependencies",
        "simulation.lamlet",
        "--format=json",
    ];
    let first = cli(&args, dir.path());
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let value = json_output(&first, 0);
    let deps = &value["dependencies"];
    assert_eq!(deps["complete"], false);
    assert_eq!(deps["working_directory"], ".");
    let data = deps["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["path"] == "structures/Si.data")
        .unwrap();
    assert_eq!(data["kind"], "data");
    assert_eq!(data["exists"], true);
    let trajectory = deps["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["path"] == "results/traj.lammpstrj")
        .unwrap();
    assert_eq!(trajectory["kind"], "trajectory");
    assert_eq!(trajectory["exists"], false);
    assert!(!dir.path().join("results").exists());
    assert!(!dir.path().join(".lamlet").exists());
}

#[test]
fn default_build_records_portable_execution_contract_and_preserves_source() {
    let dir = tempfile::tempdir().unwrap();
    resource_project(dir.path());
    let source = fs::read(dir.path().join("simulation.lamlet")).unwrap();
    let args = ["build", "simulation.lamlet", "--format=json"];
    let first = cli(&args, dir.path());
    let value = json_output(&first, 0);
    assert_eq!(value["build"]["script"], ".lamlet/build/simulation.in");
    assert_eq!(value["build"]["project_root"], "../..");
    assert_eq!(value["build"]["working_directory"], ".");
    let script = fs::read_to_string(dir.path().join(".lamlet/build/simulation.in")).unwrap();
    assert_eq!(
        script,
        "read_data \"structures/Si.data\"\nfix integrate all nve\ndump traj all custom 100 \"results/traj.lammpstrj\" id x y z\nrun 0\nundump traj\nunfix integrate\n"
    );
    let manifest_path = dir.path().join(".lamlet/build/simulation.in.manifest.json");
    let manifest = fs::read(&manifest_path).unwrap();
    let parsed: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(parsed["build"], value["build"]);
    assert!(
        !String::from_utf8(manifest.clone())
            .unwrap()
            .contains(dir.path().to_str().unwrap())
    );
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    assert_eq!(fs::read(manifest_path).unwrap(), manifest);
    assert_eq!(
        fs::read(dir.path().join("simulation.lamlet")).unwrap(),
        source
    );
    assert!(!dir.path().join("results").exists());
}

#[test]
fn project_is_discovered_from_nested_sources_and_cli_working_directories() {
    let dir = tempfile::tempdir().unwrap();
    resource_project(dir.path());
    fs::create_dir(dir.path().join("scripts")).unwrap();
    fs::rename(
        dir.path().join("simulation.lamlet"),
        dir.path().join("scripts/test.sim.lamlet"),
    )
    .unwrap();
    let value = json_output(
        &cli(
            &["build", "test.sim.lamlet", "--format=json"],
            &dir.path().join("scripts"),
        ),
        0,
    );
    assert_eq!(value["build"]["script"], ".lamlet/build/test.sim.in");
    assert!(
        fs::read_to_string(dir.path().join(".lamlet/build/test.sim.in"))
            .unwrap()
            .starts_with("read_data \"structures/Si.data\"\n")
    );
}

#[test]
fn malformed_projects_do_not_change_legacy_explicit_builds() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("lamlet.toml"), "invalid = [").unwrap();
    fs::write(dir.path().join("x.lamlet"), "run 0\n").unwrap();
    fs::write(dir.path().join("old.in"), "read_data missing.data\n").unwrap();
    let value = json_output(&cli(&["check", "x.lamlet", "--format=json"], dir.path()), 2);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_PROJECT_CONFIG")
    );
    json_output(
        &cli(
            &["build", "old.in", "-o", "copy.in", "--format=json"],
            dir.path(),
        ),
        0,
    );
    assert_eq!(
        fs::read(dir.path().join("copy.in")).unwrap(),
        b"read_data missing.data\n"
    );
}

#[test]
fn build_artifacts_cannot_replace_typed_inputs_outputs_or_project_config() {
    let dir = tempfile::tempdir().unwrap();
    resource_project(dir.path());
    let data = fs::read(dir.path().join("structures/Si.data")).unwrap();
    let config = fs::read(dir.path().join("lamlet.toml")).unwrap();
    fs::create_dir(dir.path().join("results")).unwrap();
    for target in [
        "structures/Si.data",
        "results/traj.lammpstrj",
        "lamlet.toml",
    ] {
        json_output(
            &cli(
                &["build", "simulation.lamlet", "-o", target, "--format=json"],
                dir.path(),
            ),
            2,
        );
    }
    json_output(
        &cli(
            &[
                "build",
                "simulation.lamlet",
                "-o",
                "out.in",
                "--source-map",
                "structures/Si.data",
                "--format=json",
            ],
            dir.path(),
        ),
        2,
    );
    assert_eq!(
        fs::read(dir.path().join("structures/Si.data")).unwrap(),
        data
    );
    assert_eq!(fs::read(dir.path().join("lamlet.toml")).unwrap(), config);
    assert!(!dir.path().join("out.in").exists());
    assert!(!dir.path().join("results/traj.lammpstrj").exists());
}

#[cfg(unix)]
#[test]
fn run_owns_working_directory_and_captures_engine_output_in_one_json_report() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    resource_project(dir.path());
    fs::create_dir(dir.path().join("caller")).unwrap();
    let engine = dir.path().join("fake-engine");
    fs::write(&engine, "#!/bin/sh\npwd\nprintf '%s\\n' \"$@\"\nprintf 'engine stderr\\n' >&2\ntest -f structures/Si.data || exit 9\ntest -d results || exit 10\nexit 0\n").unwrap();
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o755)).unwrap();
    let args = [
        "run",
        "../simulation.lamlet",
        "--lammps-bin",
        "../fake-engine",
        "--format=json",
    ];
    let value = json_output(&cli(&args, &dir.path().join("caller")), 0);
    assert_eq!(value["execution"]["exit_code"], 0);
    let stdout = fs::read_to_string(dir.path().join(".lamlet/build/simulation.in.stdout")).unwrap();
    assert_eq!(
        stdout,
        format!(
            "{}\n-in\n.lamlet/build/simulation.in\n-log\n.lamlet/build/simulation.in.log\n-screen\nnone\n",
            fs::canonicalize(dir.path()).unwrap().display()
        )
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(".lamlet/build/simulation.in.stderr")).unwrap(),
        "engine stderr\n"
    );
    fs::write(&engine, "#!/bin/sh\nexit 7\n").unwrap();
    let value = json_output(&cli(&args, &dir.path().join("caller")), 3);
    assert_eq!(value["execution"]["exit_code"], 7);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_EXECUTION")
    );
}

#[test]
fn invalid_input_never_starts_engine_and_missing_engine_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    resource_project(dir.path());
    fs::write(
        dir.path().join("bad.lamlet"),
        "read_data @{path(\"missing\")}\n",
    )
    .unwrap();
    json_output(
        &cli(
            &[
                "run",
                "bad.lamlet",
                "--lammps-bin",
                "not-an-engine",
                "--format=json",
            ],
            dir.path(),
        ),
        1,
    );
    assert!(!dir.path().join(".lamlet").exists());
    let value = json_output(
        &cli(
            &[
                "run",
                "simulation.lamlet",
                "--lammps-bin",
                dir.path().join("absent-engine").to_str().unwrap(),
                "--format=json",
            ],
            dir.path(),
        ),
        3,
    );
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_EXECUTION")
    );
}

#[test]
fn project_example_checks_and_builds_without_starting_lammps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/project");
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "lamlet.toml",
        "structure.data",
        "simulation.lamlet",
        "stages.lamlet",
    ] {
        fs::copy(root.join(name), dir.path().join(name)).unwrap();
    }
    json_output(
        &cli(
            &["check", "simulation.lamlet", "--strict", "--format=json"],
            dir.path(),
        ),
        0,
    );
    json_output(
        &cli(&["build", "simulation.lamlet", "--format=json"], dir.path()),
        0,
    );
    assert!(
        fs::read_to_string(dir.path().join(".lamlet/build/simulation.in"))
            .unwrap()
            .lines()
            .any(|line| line == "timestep 0.002")
    );
    assert!(!dir.path().join("results").exists());
    let args = [
        "check",
        "stages.lamlet",
        "--stage",
        "relax",
        "--arg",
        "structure=path(\"structure.data\")",
        "--arg",
        "relaxed=output(\"relaxed.data\")",
        "--strict",
        "--format=json",
    ];
    let stage = json_output(&cli(&args, dir.path()), 0);
    assert_eq!(stage["stage"]["independent_execution"], true);
}

#[test]
fn physical_findings_include_outcome_evidence_actions_and_rule_metadata() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("physics.lamlet"),
        "compute T all temp/com\nfix t all nvt temp 300 300 0.1\nfix_modify t temp T\nrun 0\n",
    )
    .unwrap();
    let args = ["check", "physics.lamlet", "--strict", "--format=json"];
    let first = cli(&args, dir.path());
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let value = json_output(&first, 0);
    assert!(
        value["physical_findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule_id"] == "LML-PHYS-002" && f["outcome"] == "violated")
    );
    let diagnostic = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "LML-PHYS-002")
        .unwrap();
    assert_eq!(diagnostic["severity"], "warning");
    assert_eq!(diagnostic["evidence"].as_array().unwrap().len(), 2);
    assert!(
        !diagnostic["suggested_actions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let schema = json_output(&cli(&["schema", "--format=json"], dir.path()), 0);
    assert!(
        schema["physical_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "LML-PHYS-002" && r["autofix"] == false)
    );
}

fn write_target_profile(root: &Path) {
    let profile = lamlet::capability::parse_help(
        include_str!("../../../tests/fixtures/lammps-help.txt"),
        "/remote/target/lmp".into(),
    )
    .unwrap();
    fs::create_dir_all(root.join(".lamlet")).unwrap();
    fs::write(
        root.join(".lamlet/target.json"),
        serde_json::to_vec_pretty(&profile).unwrap(),
    )
    .unwrap();
}
#[test]
fn saved_capability_queries_explain_metadata_and_unknown_categories() {
    let dir = tempfile::tempdir().unwrap();
    write_target_profile(dir.path());
    let value = json_output(
        &cli(
            &["target", "list", "--category", "pair", "--format=json"],
            dir.path(),
        ),
        0,
    );
    assert!(
        value["styles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["style"] == "eam/alloy" && s["availability"] == "built_in")
    );
    let args = [
        "target",
        "explain",
        "--category",
        "pair",
        "--style",
        "eam/alloy",
        "--format=json",
    ];
    let first = cli(&args, dir.path());
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let value = json_output(&first, 0);
    assert_eq!(value["metadata"]["required_package"], "MANYBODY");
    assert_eq!(value["cmake_option"], "-D PKG_MANYBODY=yes");
    assert_eq!(value["schema"]["name"], "pair_coeff/eam/alloy");
    let value = json_output(
        &cli(
            &[
                "target",
                "check",
                "--category",
                "pair",
                "--style",
                "absent",
                "--format=json",
            ],
            dir.path(),
        ),
        1,
    );
    assert_eq!(value["availability"], "unavailable");
    let value = json_output(
        &cli(
            &[
                "target",
                "check",
                "--category",
                "future",
                "--style",
                "absent",
                "--format=json",
            ],
            dir.path(),
        ),
        0,
    );
    assert_eq!(value["availability"], "unknown");
}
#[test]
fn configured_capability_validation_applies_to_legacy_and_typed_sources() {
    let dir = tempfile::tempdir().unwrap();
    write_target_profile(dir.path());
    fs::write(dir.path().join("lamlet.toml"), "[lammps]\nexecutable='/remote/target/lmp'\n[validation]\ncheck_capabilities=true\nstrict=true\n").unwrap();
    for name in ["x.in", "x.lamlet"] {
        fs::write(dir.path().join(name), "pair_style absent/style\n").unwrap();
        let args = ["check", name, "--format=json"];
        let value = json_output(&cli(&args, dir.path()), 1);
        assert!(
            value["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"] == "LML-CAP-001")
        );
        assert!(
            value["target_requirements"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["style"] == "absent/style" && r["availability"] == "unavailable")
        );
        fs::write(dir.path().join(name), "pair_style eam/alloy\n").unwrap();
        json_output(&cli(&args, dir.path()), 0);
    }
    let saved_profile = fs::read(dir.path().join(".lamlet/target.json")).unwrap();
    for destination in [".lamlet/target.json", "lamlet.toml"] {
        json_output(
            &cli(
                &["build", "x.in", "-o", destination, "--format=json"],
                dir.path(),
            ),
            2,
        );
    }
    assert_eq!(
        fs::read(dir.path().join(".lamlet/target.json")).unwrap(),
        saved_profile
    );
    fs::write(dir.path().join("lamlet.toml"), "[lammps.requirements]\nversion='>=2026-01-01'\npackages=['RIGID']\n[validation]\ncheck_capabilities=true\n").unwrap();
    let value = json_output(&cli(&["check", "x.lamlet", "--format=json"], dir.path()), 1);
    for code in ["LML-CAP-003", "LML-CAP-004"] {
        assert!(
            value["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"] == code)
        );
    }
}
#[test]
fn target_requirements_work_offline_and_expose_dynamic_hybrid_limits() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("x.lamlet"),
        "pair_style hybrid eam/alloy lj/cut 2.5\n@scope {\n@fix f all nve\nrun 0\n}\n",
    )
    .unwrap();
    let args = ["target", "requirements", "x.lamlet", "--format=json"];
    let value = json_output(&cli(&args, dir.path()), 0);
    assert!(!dir.path().join(".lamlet").exists());
    assert!(
        value["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["category"] == "pair"
                && r["style"] == "eam/alloy"
                && r["availability"] == "unknown")
    );
    write_target_profile(dir.path());
    let value = json_output(&cli(&args, dir.path()), 0);
    assert!(
        value["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["style"] == "eam/alloy" && r["availability"] == "built_in")
    );
}
#[test]
fn missing_or_invalid_profile_is_an_explicit_environment_error() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("lamlet.toml"),
        "[validation]\ncheck_capabilities=true\n",
    )
    .unwrap();
    fs::write(dir.path().join("x.lamlet"), "run 0\n").unwrap();
    let value = json_output(&cli(&["check", "x.lamlet", "--format=json"], dir.path()), 2);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_TARGET_PROFILE")
    );
    assert!(!dir.path().join(".lamlet").exists());
    fs::create_dir(dir.path().join(".lamlet")).unwrap();
    fs::write(dir.path().join(".lamlet/target.json"), "not json").unwrap();
    json_output(
        &cli(
            &[
                "target",
                "check",
                "--category",
                "fix",
                "--style",
                "nvt",
                "--format=json",
            ],
            dir.path(),
        ),
        2,
    );
}
#[cfg(unix)]
#[test]
fn capability_probe_persists_only_complete_success_and_binds_binary_identity() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let engine = dir.path().join("fake-engine");
    let help = include_str!("../../../tests/fixtures/lammps-help.txt");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = '-h' ]; then\ncat <<'LAMLET_HELP'\n{help}LAMLET_HELP\nexit 0\nfi\nexit 0\n"
    );
    fs::write(&engine, script).unwrap();
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        dir.path().join("lamlet.toml"),
        "[lammps]\nexecutable='./fake-engine'\n[validation]\ncheck_capabilities=true\n",
    )
    .unwrap();
    let args = ["target", "inspect", "--format=json"];
    let first = cli(&args, dir.path());
    let value = json_output(&first, 0);
    assert_eq!(value["profile"]["lammps_version"], "2025-07-22");
    assert_eq!(first.stdout, cli(&args, dir.path()).stdout);
    let saved = fs::read(dir.path().join(".lamlet/target.json")).unwrap();
    fs::write(dir.path().join("x.lamlet"), "fix f all nve\nrun 0\n").unwrap();
    json_output(&cli(&["check", "x.lamlet", "--format=json"], dir.path()), 0);
    json_output(&cli(&["run", "x.lamlet", "--format=json"], dir.path()), 0);
    json_output(
        &cli(
            &[
                "run",
                "x.lamlet",
                "--lammps-bin",
                "/bin/false",
                "--format=json",
            ],
            dir.path(),
        ),
        2,
    );
    fs::write(
        &engine,
        format!("#!/bin/sh\ncat <<'LAMLET_HELP'\n{help}LAMLET_HELP\nexit 7\n"),
    )
    .unwrap();
    let value = json_output(&cli(&args, dir.path()), 2);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_TARGET_PROBE")
    );
    assert_eq!(
        fs::read(dir.path().join(".lamlet/target.json")).unwrap(),
        saved
    );
    let value = json_output(&cli(&["check", "x.lamlet", "--format=json"], dir.path()), 2);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_TARGET_PROFILE")
    );
}
