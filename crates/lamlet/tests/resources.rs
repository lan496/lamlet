use lamlet::{
    Compilation, Language, Options, compile_with_options, resources::Project, source::Source,
};
use std::fs;

struct Fixture {
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("structures")).unwrap();
        fs::create_dir(dir.path().join("results")).unwrap();
        fs::write(dir.path().join("structures/Si.data"), "data").unwrap();
        fs::write(dir.path().join("structures/a #}.data"), "data").unwrap();
        Self { dir }
    }
    fn check(&self, text: &str, strict: bool) -> Compilation {
        let file = self.dir.path().join("simulation.lamlet");
        fs::write(&file, text).unwrap();
        compile_with_options(
            &Source::new(file.to_string_lossy(), text.as_bytes().to_vec()),
            &Options {
                language: Language::Lamlet,
                strict,
                project: Some(Project::load(self.dir.path()).unwrap()),
                ..Options::default()
            },
        )
    }
    fn output(&self, text: &str) -> String {
        let c = self.check(text, true);
        assert!(!c.has_errors(), "{:?}", c.diagnostics);
        String::from_utf8(c.output).unwrap()
    }
    fn error(&self, text: &str, code: &str) {
        let c = self.check(text, true);
        assert!(
            c.diagnostics.iter().any(|d| d.code == code),
            "{text}: {:?}",
            c.diagnostics
        );
        assert!(c.output.is_empty());
        assert!(c.mappings.is_empty());
    }
}

#[test]
fn typed_paths_annotations_and_style_dependent_slots() {
    let f = Fixture::new();
    let text = "@let structure: InputFile = path(\"structures/Si.data\")\n@let potential: PotentialFile = path(\"structures/Si.data\")\n@let trajectory = output(\"results/traj.lammpstrj\")\nread_data @{structure}\npair_style eam/alloy\npair_coeff * * @{potential} Si\ndump traj all custom 100 @{trajectory} id type x y z\n";
    let c = f.check(text, true);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(
        String::from_utf8(c.output).unwrap(),
        "read_data \"structures/Si.data\"\npair_style eam/alloy\npair_coeff * * \"structures/Si.data\" Si\ndump traj all custom 100 \"results/traj.lammpstrj\" id type x y z\n"
    );
    let data = c
        .resources
        .inputs
        .iter()
        .find(|d| d.path == "structures/Si.data")
        .unwrap();
    assert_eq!(data.kind, "potential");
    assert_eq!(data.exists, Some(true));
    let out = c
        .resources
        .outputs
        .iter()
        .find(|d| d.path == "results/traj.lammpstrj")
        .unwrap();
    assert_eq!(out.kind, "trajectory");
    assert_eq!(out.exists, Some(false));
    assert!(!f.dir.path().join("results/traj.lammpstrj").exists());
    assert!(!c.resources.complete);
    assert_eq!(f.check(text, true).resources.inputs.len(), 1);
}

#[test]
fn path_strings_protect_lammps_delimiters_and_reject_wrong_types() {
    let f = Fixture::new();
    assert_eq!(
        f.output("@let p = path(\"structures/a #}.data\") # comment\nread_data @{p}\n"),
        "read_data \"structures/a #}.data\"\n"
    );
    f.error("read_data @{dir(\"results\")}\n", "E_TYPE_MISMATCH");
    f.error("read_data @{output(\"results/x\")}\n", "E_TYPE_MISMATCH");
    f.error("read_data @{1}\n", "E_TYPE_MISMATCH");
    f.error(
        "@let x: Directory = path(\"structures/Si.data\")\n",
        "E_TYPE_MISMATCH",
    );
    f.error(
        "@let x = path(\"structures/Si.data\") + 1\n",
        "E_TYPE_MISMATCH",
    );
    f.error("@let x = \"structures/Si.data\"\n", "E_EXPRESSION");
    f.error("include @{path(\"simulation.lamlet\")}\n", "E_INPUT_SCRIPT");
}

#[test]
fn explicit_resources_check_existence_kind_roots_and_collisions() {
    let f = Fixture::new();
    f.error("@let p = path(\"missing\")\n", "E_PATH_NOT_FOUND");
    f.error("@let p = path(\"results\")\n", "E_PATH_KIND");
    f.error("@let p = dir(\"structures/Si.data\")\n", "E_PATH_KIND");
    f.error("@let p = path(\"../elsewhere\")\n", "E_PATH_ESCAPE");
    f.error(
        "@let p = output(\"simulation.lamlet\")\n",
        "E_OUTPUT_COLLISION",
    );
    for declarations in [
        "@let i = path(\"structures/Si.data\")\n@let o = output(\"structures/Si.data\")\n",
        "@let o = output(\"structures/Si.data\")\n@let i = path(\"structures/Si.data\")\n",
    ] {
        f.error(declarations, "E_OUTPUT_COLLISION");
    }
    f.error(
        "@let p = output(\"results/x\")\nwrite_data @{p}\nwrite_restart @{p}\n",
        "E_OUTPUT_COLLISION",
    );
    fs::write(
        f.dir.path().join("lamlet.toml"),
        "[paths]\noutput_root = 'results'\n",
    )
    .unwrap();
    assert_eq!(
        f.output("write_data @{output(\"results/x\")}\n"),
        "write_data \"results/x\"\n"
    );
    f.error("write_data @{output(\"x\")}\n", "E_PATH_ESCAPE");
}

#[test]
fn directory_changes_rebase_paths_and_runtime_barriers_stop_claims() {
    let f = Fixture::new();
    assert_eq!(
        f.output("shell cd results\nread_data @{path(\"structures/Si.data\")}\n"),
        "shell cd results\nread_data \"../structures/Si.data\"\n"
    );
    assert_eq!(
        f.output("shell cd @{dir(\"results\")}\nread_data @{path(\"structures/Si.data\")}\n"),
        "shell cd \"results\"\nread_data \"../structures/Si.data\"\n"
    );
    for barrier in [
        "include common.in",
        "shell cd ${dir}",
        "jump SELF end",
        "if '1' then 'shell cd results'",
    ] {
        f.error(
            &format!("{barrier}\nread_data @{{path(\"structures/Si.data\")}}\n"),
            "E_PATH_CONTEXT",
        );
    }
}

#[test]
fn patterns_dynamic_names_and_legacy_paths_have_distinct_guarantees() {
    let f = Fixture::new();
    assert_eq!(
        f.output("read_restart @{pattern(\"structures/restart.*\")}\n"),
        "read_restart \"structures/restart.*\"\n"
    );
    f.error(
        "read_data @{pattern(\"structures/restart.*\")}\n",
        "E_TYPE_MISMATCH",
    );
    f.error(
        "read_restart @{path(\"structures/restart.*\")}\n",
        "E_PATH_SYNTAX",
    );
    let c = f.check("read_data @{dynamic(\"structures/${name}.data\")}\n", false);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(c.output, b"read_data structures/${name}.data\n");
    assert_eq!(
        c.resources
            .dynamic
            .iter()
            .find(|d| d.path.contains("${name}"))
            .unwrap()
            .exists,
        None
    );
    f.error(
        "read_data @{dynamic(\"structures/${name}.data\")}\n",
        "E_UNVERIFIED_ARGUMENT",
    );
    assert_eq!(
        f.output("read_data @unchecked{dynamic(\"structures/${name}.data\")}\n"),
        "read_data structures/${name}.data\n"
    );
    let text = "read_data missing.data\nread_restart ../missing.*\n";
    let c = f.check(text, true);
    assert!(!c.has_errors());
    assert_eq!(c.output, text.as_bytes());
    assert!(c.resources.inputs.is_empty());
    assert_eq!(c.resources.unresolved.len(), 2);
}

#[test]
fn runtime_basenames_keep_patterns_and_require_fixed_confined_parents() {
    let f = Fixture::new();
    assert_eq!(
        f.output("read_restart @{pattern(\"structures/restart.*.%\")}\n"),
        "read_restart \"structures/restart.*.%\"\n"
    );
    assert_eq!(
        f.output(
            "dump traj all custom 100 @{output(\"results/new/traj.*.%.lammpstrj\")} id x y z\n"
        ),
        "dump traj all custom 100 \"results/new/traj.*.%.lammpstrj\" id x y z\n"
    );
    assert!(!f.dir.path().join("results/new").exists());
    f.error(
        "read_restart @{pattern(\"missing/restart.*\")}\n",
        "E_PATH_KIND",
    );
    for text in [
        "read_restart @{pattern(\"structures/*/restart.*\")}\n",
        "dump traj all custom 100 @{output(\"results/*/traj.*\")} id x y z\n",
        "read_data @unchecked{dynamic(\"structures/${dir}/input.data\")}\n",
    ] {
        f.error(text, "E_PATH_SYNTAX");
    }
}

#[cfg(unix)]
#[test]
fn symlink_escapes_and_hardlink_output_aliases_are_rejected() {
    let f = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "external").unwrap();
    std::os::unix::fs::symlink(outside.path(), f.dir.path().join("outside")).unwrap();
    f.error("read_data @{path(\"outside/secret\")}\n", "E_PATH_ESCAPE");
    f.error("write_data @{output(\"outside/new\")}\n", "E_PATH_ESCAPE");
    f.error(
        "read_restart @{pattern(\"outside/restart.*\")}\n",
        "E_PATH_ESCAPE",
    );
    f.error(
        "dump traj all custom 100 @{output(\"outside/traj.*\")} id x y z\n",
        "E_PATH_ESCAPE",
    );
    std::os::unix::fs::symlink(
        outside.path().join("secret"),
        f.dir.path().join("results/leak"),
    )
    .unwrap();
    f.error(
        "write_data @{output(\"results/unused*/../leak\")}\n",
        "E_PATH_ESCAPE",
    );
    fs::hard_link(
        f.dir.path().join("structures/Si.data"),
        f.dir.path().join("results/alias"),
    )
    .unwrap();
    f.error(
        "@let i = path(\"structures/Si.data\")\nwrite_data @{output(\"results/alias\")}\n",
        "E_OUTPUT_COLLISION",
    );
}

#[test]
fn project_discovery_and_configuration_are_explicit() {
    let f = Fixture::new();
    fs::write(f.dir.path().join("lamlet.toml"), "[project]\nname='silicon'\n[paths]\ninput_root='structures'\noutput_root='results'\n[build]\ndirectory='.lamlet/generated'\n").unwrap();
    fs::write(f.dir.path().join("structures/simulation.lamlet"), "").unwrap();
    let p = Project::discover(&f.dir.path().join("structures/simulation.lamlet")).unwrap();
    assert_eq!(p.name.as_deref(), Some("silicon"));
    assert_eq!(p.build_directory, p.root.join(".lamlet/generated"));
    for config in [
        "[paths]\ninput_root='../escape'\n",
        "[paths]\ninput_root='missing'\n",
        "[build]\nunknown=true\n",
    ] {
        fs::write(f.dir.path().join("lamlet.toml"), config).unwrap();
        assert!(Project::load(f.dir.path()).is_err());
    }
}
