use lamlet::{
    Compilation, Language, Options, compile_with_options, resources::Project, source::Source,
    stage::Invocation,
};

const STAGE: &str = "@stage relax(structure: InputFile, result: OutputFile, steps: Int) {\nunits metal\natom_style atomic\nread_data @{structure} nocoeff\npair_style lj/cut 2.5\npair_coeff * * 1 1\n@fix integrate all nve\nrun @{steps}\nwrite_data @{result}\n}\n";

fn compile(text: &str, arguments: &[&str]) -> Compilation {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("atoms.data"), "test fixture").unwrap();
    let source = Source::new(
        dir.path().join("workflow.lamlet").to_string_lossy(),
        text.as_bytes().to_vec(),
    );
    let invocation = Invocation::parse(
        "relax",
        None,
        &arguments.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
    )
    .unwrap();
    let result = compile_with_options(
        &source,
        &Options {
            language: Language::Lamlet,
            strict: true,
            project: Some(Project::load(dir.path()).unwrap()),
            stage: Some(invocation),
            ..Options::default()
        },
    );
    assert!(
        !dir.path().join(".lamlet").exists(),
        "Compilation must be read only"
    );
    result
}
const ARGS: &[&str] = &[
    "structure=path(\"atoms.data\")",
    "result=output(\"relaxed.data\")",
    "steps=10",
];
fn error(text: &str, args: &[&str], code: &str) {
    let c = compile(text, args);
    assert!(
        c.diagnostics.iter().any(|d| d.code == code),
        "{text}: {:?}",
        c.diagnostics
    );
    assert!(c.output.is_empty());
    assert!(c.stage.is_none_or(|stage| !stage.independent_execution));
}

#[test]
fn stage_lowers_in_fresh_state_with_explicit_paths_and_owned_cleanup() {
    let c = compile(STAGE, ARGS);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(c.output, b"units metal\natom_style atomic\nread_data \"../../../../atoms.data\" nocoeff\npair_style lj/cut 2.5\npair_coeff * * 1 1\nfix integrate all nve\nrun 10\nwrite_data \"relaxed.data\"\nunfix integrate\n");
    let plan = c.stage.unwrap();
    assert!(plan.independent_execution);
    assert!(!plan.resumable);
    assert_eq!(plan.working_directory, ".lamlet/build/stages/relax");
    assert_eq!(
        c.resources
            .outputs
            .iter()
            .find(|d| d.kind == "data")
            .unwrap()
            .path,
        ".lamlet/build/stages/relax/relaxed.data"
    );
    assert!(c.resources.unresolved.is_empty());
    assert!(c.resources.complete);
    error(
        &STAGE.replace("read_data @{structure} nocoeff\n", ""),
        ARGS,
        "E_STATE_CONTRACT",
    );
    error(
        &STAGE.replace("@fix integrate all nve\n", ""),
        ARGS,
        "E_STATE_CONTRACT",
    );
    error(
        &STAGE.replace("pair_coeff * * 1 1\n", ""),
        ARGS,
        "E_STATE_CONTRACT",
    );
    error(
        &STAGE.replace("run @{steps}", "unfix integrate\nrun @{steps}"),
        ARGS,
        "E_SCOPE_OWNERSHIP",
    );
    error(
        &STAGE.replace(
            "@fix integrate all nve",
            "fix first all nve\nfix second all nve",
        ),
        ARGS,
        "E_EFFECT_CONFLICT",
    );
    error(&STAGE.replace(" nocoeff", ""), ARGS, "E_UNVERIFIED_EFFECT");
    let zero = compile(
        &STAGE
            .replace("@fix integrate all nve\n", "")
            .replace("run @{steps}", "run 0"),
        ARGS,
    );
    assert!(!zero.has_errors(), "{:?}", zero.diagnostics);
    let typed = compile(
        &STAGE.replace(
            "@fix integrate all nve",
            "@compute T all temp\n@fix integrate all nvt temp @{300 K} @{300 K} @{100 fs}",
        ),
        ARGS,
    );
    assert!(!typed.has_errors(), "{:?}", typed.diagnostics);
    assert!(
        String::from_utf8(typed.output)
            .unwrap()
            .contains("fix integrate all nvt temp 300 300 0.1\n")
    );
}

#[test]
fn parameters_are_exact_typed_and_cannot_escape_or_hide_dependencies() {
    error(STAGE, &ARGS[..2], "E_STAGE_ARGUMENT");
    error(STAGE, &[ARGS[0], ARGS[1], "steps=300 K"], "E_TYPE_MISMATCH");
    error(
        STAGE,
        &["structure=path(\"missing\")", ARGS[1], ARGS[2]],
        "E_PATH_NOT_FOUND",
    );
    error(
        STAGE,
        &[ARGS[0], "result=output(\"../outside\")", ARGS[2]],
        "E_PATH_ESCAPE",
    );
    error(
        STAGE,
        &[ARGS[0], "result=output(\"restart.*\")", ARGS[2]],
        "E_STAGE_RESOURCE",
    );
    error(
        &STAGE.replace("read_data @{structure}", "read_data atoms.data"),
        ARGS,
        "E_STAGE_RESOURCE",
    );
    error(
        &STAGE.replace(
            "read_data @{structure}",
            "read_data @{path(\"other.data\")}",
        ),
        ARGS,
        "E_STAGE_RESOURCE",
    );
    error(
        &STAGE.replace("write_data @{result}\n", ""),
        ARGS,
        "E_STAGE_OUTPUT",
    );
    error(
        &STAGE.replace("run @{steps}", "run @unchecked{steps}"),
        ARGS,
        "E_STAGE_RESOURCE",
    );
    error(&format!("units metal\n{STAGE}"), ARGS, "E_STAGE_DEFINITION");
    error(&format!("{STAGE}{STAGE}"), ARGS, "E_STAGE_DEFINITION");
    error(
        &STAGE.replace("structure: InputFile", "structure: DynamicPath"),
        ARGS,
        "E_STAGE_DEFINITION",
    );
    error(
        &STAGE.replace("run @{steps}", "include hidden.in"),
        ARGS,
        "E_STAGE_RESOURCE",
    );
}

#[test]
fn module_expansion_cannot_bypass_stage_dependency_or_state_contracts() {
    let module = "@module model {\n@fn setup() requires [BoxDefined] ensures [ModelDefined] {\npair_style lj/cut 2.5\npair_coeff * * 1 1\n}\n}\n";
    let text = format!(
        "{module}{}",
        STAGE.replace(
            "pair_style lj/cut 2.5\npair_coeff * * 1 1",
            "@call model.setup()"
        )
    );
    let c = compile(&text, ARGS);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    error(
        &text.replace(
            "pair_style lj/cut 2.5",
            "read_data hidden.data nocoeff\npair_style lj/cut 2.5",
        ),
        ARGS,
        "E_STAGE_RESOURCE",
    );
    error(
        &text.replace(
            "pair_style lj/cut 2.5",
            "@let hidden = path(\"hidden.data\")\npair_style lj/cut 2.5",
        ),
        ARGS,
        "E_STAGE_RESOURCE",
    );
}

#[test]
fn definition_inspection_does_not_bind_artifacts_or_claim_execution() {
    let source = Source::new("workflow.lamlet", STAGE.as_bytes().to_vec());
    let definitions = lamlet::stage::inspect(&source).unwrap();
    let stage = definitions.iter().find(|d| d.name == "relax").unwrap();
    assert_eq!(
        stage
            .parameters
            .iter()
            .find(|p| p.name == "structure")
            .unwrap()
            .direction,
        "input"
    );
    assert_eq!(
        stage
            .parameters
            .iter()
            .find(|p| p.name == "result")
            .unwrap()
            .direction,
        "output"
    );
    let c = compile_with_options(
        &source,
        &Options {
            language: Language::Lamlet,
            ..Options::default()
        },
    );
    assert!(c.diagnostics.iter().any(|d| d.code == "E_STAGE_SELECTION"));
    for (args, code) in [
        (vec!["n=1", "n=2"], "E_STAGE_ARGUMENT"),
        (vec!["n=missing"], "E_UNBOUND_NAME"),
        (vec!["n=1/0"], "E_DIVISION_BY_ZERO"),
    ] {
        let arguments = args.into_iter().map(String::from).collect::<Vec<_>>();
        assert_eq!(
            Invocation::parse("relax", None, &arguments)
                .unwrap_err()
                .code,
            code
        );
    }
    assert!(Invocation::parse("relax", Some("../escape"), &[]).is_err());
}

#[test]
fn stage_potential_parameters_refine_input_files_and_preserve_role_metadata() {
    let dir = tempfile::tempdir().unwrap();
    for file in ["atoms.data", "Si.eam"] {
        std::fs::write(dir.path().join(file), "static resource fixture").unwrap();
    }
    let text = "@stage single(structure: InputFile, potential: PotentialFile, result: OutputFile) {\nunits metal\nread_data @{structure} nocoeff\npair_style eam\npair_coeff * * @{potential}\nrun 0\nwrite_data @{result}\n}\n";
    let arguments = [
        "structure=path(\"atoms.data\")",
        "potential=path(\"Si.eam\")",
        "result=output(\"state.data\")",
    ]
    .map(String::from);
    let source = Source::new(
        dir.path().join("single.lamlet").to_string_lossy(),
        text.as_bytes().to_vec(),
    );
    let options = Options {
        language: Language::Lamlet,
        strict: true,
        project: Some(Project::load(dir.path()).unwrap()),
        stage: Some(Invocation::parse("single", None, &arguments).unwrap()),
        ..Options::default()
    };
    let c = compile_with_options(&source, &options);
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    let potential = c
        .resources
        .inputs
        .iter()
        .find(|input| input.path == "Si.eam")
        .unwrap();
    assert_eq!(potential.kind, "potential");
    assert_eq!(
        potential.resource_type,
        lamlet::resources::ResourceKind::PotentialFile
    );
    assert!(
        String::from_utf8(c.output)
            .unwrap()
            .contains("pair_coeff * * \"../../../../Si.eam\"\n")
    );
    let hidden = compile_with_options(
        &Source::new(
            source.path,
            text.replace("@{potential}", "Si.eam").into_bytes(),
        ),
        &options,
    );
    assert!(
        hidden
            .diagnostics
            .iter()
            .any(|d| d.code == "E_STAGE_RESOURCE")
    );
}

#[cfg(unix)]
#[test]
fn stage_instance_directories_cannot_alias_other_instances() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::load(dir.path()).unwrap();
    let stages = project.build_directory.join("stages");
    std::fs::create_dir_all(stages.join("other")).unwrap();
    std::os::unix::fs::symlink(stages.join("other"), stages.join("relax")).unwrap();
    let invocation = Invocation::parse("relax", None, &[]).unwrap();
    assert_eq!(
        invocation.directory(&project).unwrap_err().code,
        "E_PATH_ESCAPE"
    );
}
