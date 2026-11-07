//! Declarative validation coverage, independent of lexical and expression parsing.
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub enum Requirement {
    PositiveTime,
    Temperature,
    Pressure,
    NonNegativeInt,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperandSchema {
    pub position: usize,
    pub requirement: Requirement,
}
#[derive(Clone, Debug, Serialize)]
pub struct KeywordSchema {
    pub keyword: &'static str,
    pub operands: &'static [Requirement],
}
#[derive(Clone, Debug, Serialize)]
pub struct CommandSchema {
    pub name: &'static str,
    pub coverage: &'static str,
    pub min_arguments: usize,
    pub max_arguments: Option<usize>,
    pub operands: &'static [OperandSchema],
    pub keywords: &'static [KeywordSchema],
    pub state_effect: &'static str,
    pub limitations: &'static str,
    pub lammps_version: Option<&'static str>,
}

pub static SCHEMAS: &[CommandSchema] = &[
    CommandSchema {
        name: "units",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: Some(1),
        operands: &[],
        keywords: &[],
        state_effect: "track unit style; require no box",
        limitations: "Only metal and real support physical interpolation",
        lammps_version: None,
    },
    CommandSchema {
        name: "atom_style",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "track raw style; require no box",
        limitations: "Style arguments and package availability are opaque",
        lammps_version: None,
    },
    CommandSchema {
        name: "read_data",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "box present; group and fix inventories become unknown",
        limitations: "Does not inspect files or optional fix callbacks",
        lammps_version: None,
    },
    CommandSchema {
        name: "read_restart",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "box present; units, atom style, and ID inventories become unknown",
        limitations: "Restart state is external and is not inspected",
        lammps_version: None,
    },
    CommandSchema {
        name: "create_box",
        coverage: "partial",
        min_arguments: 2,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "box present",
        limitations: "Regions and optional arguments are not checked",
        lammps_version: None,
    },
    CommandSchema {
        name: "group",
        coverage: "partial",
        min_arguments: 2,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "track declared and deleted group IDs",
        limitations: "Membership and group style arguments are not checked",
        lammps_version: None,
    },
    CommandSchema {
        name: "fix",
        coverage: "partial",
        min_arguments: 3,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "track fix ID and group reference",
        limitations: "Only nve and the described nvt/npt keyword prefix are recognized; other styles weaken state",
        lammps_version: None,
    },
    CommandSchema {
        name: "fix/nve",
        coverage: "partial",
        min_arguments: 3,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "track fix ID and group reference",
        limitations: "No numerical stability or integration correctness guarantee",
        lammps_version: None,
    },
    CommandSchema {
        name: "fix/nvt",
        coverage: "partial",
        min_arguments: 3,
        max_arguments: None,
        operands: &[],
        keywords: &[KeywordSchema {
            keyword: "temp",
            operands: &[
                Requirement::Temperature,
                Requirement::Temperature,
                Requirement::PositiveTime,
            ],
        }],
        state_effect: "track fix ID and group reference",
        limitations: "Keyword validation stops at the first opaque option",
        lammps_version: None,
    },
    CommandSchema {
        name: "fix/npt",
        coverage: "partial",
        min_arguments: 3,
        max_arguments: None,
        operands: &[],
        keywords: &[
            KeywordSchema {
                keyword: "temp",
                operands: &[
                    Requirement::Temperature,
                    Requirement::Temperature,
                    Requirement::PositiveTime,
                ],
            },
            KeywordSchema {
                keyword: "iso",
                operands: &[
                    Requirement::Pressure,
                    Requirement::Pressure,
                    Requirement::PositiveTime,
                ],
            },
        ],
        state_effect: "track fix ID and group reference",
        limitations: "Only temp and iso keyword prefixes are checked",
        lammps_version: None,
    },
    CommandSchema {
        name: "unfix",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: Some(1),
        operands: &[],
        keywords: &[],
        state_effect: "require and remove fix ID",
        limitations: "Runtime control flow and opaque callbacks can make existence unknown",
        lammps_version: None,
    },
    CommandSchema {
        name: "compute",
        coverage: "partial",
        min_arguments: 3,
        max_arguments: None,
        operands: &[],
        keywords: &[],
        state_effect: "track compute ID",
        limitations: "Compute styles and arguments are opaque and weaken other state",
        lammps_version: None,
    },
    CommandSchema {
        name: "uncompute",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: Some(1),
        operands: &[],
        keywords: &[],
        state_effect: "require and remove compute ID",
        limitations: "Does not validate references from other commands",
        lammps_version: None,
    },
    CommandSchema {
        name: "timestep",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: Some(1),
        operands: &[OperandSchema {
            position: 1,
            requirement: Requirement::PositiveTime,
        }],
        keywords: &[],
        state_effect: "preserve tracked state",
        limitations: "Runtime expressions are unknown",
        lammps_version: None,
    },
    CommandSchema {
        name: "run",
        coverage: "partial",
        min_arguments: 1,
        max_arguments: None,
        operands: &[OperandSchema {
            position: 1,
            requirement: Requirement::NonNegativeInt,
        }],
        keywords: &[],
        state_effect: "weaken state when options or opaque callbacks may execute commands",
        limitations: "Only the first argument is checked; optional arguments are opaque",
        lammps_version: None,
    },
];

pub fn find(name: &str) -> Option<&'static CommandSchema> {
    SCHEMAS.iter().find(|s| s.name == name)
}
