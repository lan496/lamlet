//! Provenance and coverage of the indexed official manual, independent of schemas.
use serde_json::{Value, json};
use std::sync::OnceLock;

fn catalog() -> &'static Value {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../data/lammps_docs.json"))
            .expect("generated documentation catalog must be valid JSON")
    })
}

pub fn query(query: Option<&str>) -> Value {
    let catalog = catalog();
    let normalized = query.map(|q| q.replacen('/', " ", 1));
    let pages: Vec<_> = catalog["pages"]
        .as_array()
        .expect("catalog pages")
        .iter()
        .filter(|page| {
            normalized.as_ref().is_some_and(|q| {
                page["path"]
                    .as_str()
                    .is_some_and(|p| p.trim_end_matches(".rst") == q)
                    || page["commands"].as_array().is_some_and(|commands| {
                        commands.iter().any(|c| c.as_str().is_some_and(|c| c == q))
                    })
            })
        })
        .collect();
    json!({"version": catalog["version"], "revision": catalog["revision"],
        "repository": catalog["repository"], "page_count": catalog["page_count"],
        "source_bytes": catalog["source_bytes"], "candidate_paragraphs": catalog["candidate_paragraphs"],
        "scope": catalog["scope"], "coverage": catalog["coverage"], "pages": pages})
}
