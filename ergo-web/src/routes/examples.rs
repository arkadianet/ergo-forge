//! `GET /api/v1/examples` and `/api/v1/examples/{id}` — the example gallery,
//! read from `EXAMPLES_DIR` (default `examples/contracts`). Ids are the
//! relative path without `.es`, slashes kept (`dexy/bank/bank`).

use std::path::{Path, PathBuf};

use axum::{extract::Path as AxumPath, Json};

use crate::{dto, error::ApiError};

fn examples_dir() -> PathBuf {
    if let Ok(d) = std::env::var("EXAMPLES_DIR") {
        return PathBuf::from(d);
    }
    // Repo root when run from the workspace; one up when run from the crate
    // (cargo test); the crate's own copy is not a thing, so no third guess.
    for c in ["examples/contracts", "../examples/contracts"] {
        let p = PathBuf::from(c);
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::from("examples/contracts")
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<dto::ExampleSummary>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let p = e.path();
        if p.is_dir() {
            let sub = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            walk(&p, &sub, out);
        } else if let Some(stem) = name.strip_suffix(".es") {
            let id = if prefix.is_empty() {
                stem.to_string()
            } else {
                format!("{prefix}/{stem}")
            };
            let group = prefix.split('/').next().unwrap_or("").to_string();
            out.push(dto::ExampleSummary {
                id,
                group,
                name: stem.to_string(),
            });
        }
    }
}

pub async fn list() -> Json<Vec<dto::ExampleSummary>> {
    let mut out = Vec::new();
    walk(&examples_dir(), "", &mut out);
    // basics first, then the real projects alphabetically.
    out.sort_by_key(|e| (e.group != "basics", e.id.clone()));
    Json(out)
}

pub async fn fetch(AxumPath(id): AxumPath<String>) -> Result<Json<dto::ExampleDto>, ApiError> {
    // Ids are relative paths; refuse anything that could escape the dir.
    if id.is_empty()
        || id
            .split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
        || id.contains('\\')
    {
        return Err(ApiError::NotFound(format!("no example `{id}`")));
    }
    let path = examples_dir().join(format!("{id}.es"));
    let source = std::fs::read_to_string(&path)
        .map_err(|_| ApiError::NotFound(format!("no example `{id}`")))?;
    let params = ergo_sandbox::compile::scan_params(&source);
    let template = source.contains("@contract");
    let (group, name) = match id.rsplit_once('/') {
        Some((g, n)) => (g.split('/').next().unwrap_or(g).to_string(), n.to_string()),
        None => (String::new(), id.clone()),
    };
    Ok(Json(dto::ExampleDto {
        id,
        group,
        name,
        source,
        params,
        template,
    }))
}
