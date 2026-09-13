//! Explain values from one full scenario reduction. No extracted-expression eval.
use crate::{app::AppState, dto, error::ApiError, extract::ApiJson};
use axum::{extract::State, Json};
use ergo_sandbox::{source_positions::SourcePosition, Scenario};
use ergo_ser::opcode::{node_opcode, preorder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ExplainRequest {
    pub scenario: Scenario,
    pub selection: Selection,
}
/// UTF-8 byte interval [offset, offset + length). A point selection instead
/// accepts a compiler-style, one-based line/UTF-16-column pair (exact start).
#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Selection {
    Range { offset: u32, length: u32 },
    Point { line: u32, col: u32 },
}
impl Selection {
    fn range(&self, source: &str) -> Result<(u32, u32, bool), ApiError> {
        let invalid = || {
            ApiError::InvalidInput("selection must be a nonempty source byte range or an exact one-based line/col position".into())
        };
        match *self {
            Self::Range { offset, length } => {
                let end = offset.checked_add(length).ok_or_else(invalid)?;
                if length == 0
                    || end as usize > source.len()
                    || !source.is_char_boundary(offset as usize)
                    || !source.is_char_boundary(end as usize)
                {
                    return Err(invalid());
                }
                Ok((offset, end, offset == 0 && end as usize == source.len()))
            }
            Self::Point { line, col } => {
                if line == 0 || col == 0 {
                    return Err(invalid());
                }
                // Use the pinned compiler's convention (including CRLF),
                // never silently pick a nearby byte or split a UTF-8 scalar.
                let mut target = col as usize - 1;
                let mut lines = source.lines();
                for _ in 1..line {
                    target += lines.next().ok_or_else(invalid)?.encode_utf16().count() + 1;
                }
                if col as usize > lines.next().ok_or_else(invalid)?.encode_utf16().count() + 1 {
                    return Err(invalid());
                }
                let mut units = 0;
                let offset = source
                    .char_indices()
                    .find_map(|(i, c)| {
                        let here = units;
                        units += c.len_utf16();
                        (here == target).then_some(i as u32)
                    })
                    .ok_or_else(invalid)?;
                if ergo_compiler::span::line_col(source, offset) != (line, col) {
                    return Err(invalid());
                }
                Ok((offset, offset + 1, false))
            }
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainedValue {
    /// Index into this response's full-run values (repeated lambda calls remain ordered).
    pub trace_index: usize,
    pub value: String,
    pub truncated: bool,
    pub residual: Option<String>,
    pub residual_note: Option<&'static str>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Expression {
    pub ir_id: u64,
    pub opcode: String,
    pub span: Option<SourcePosition>,
    pub values: Vec<ExplainedValue>,
    /// Full-run reducedTo is available for the root, even for trivial fast paths.
    pub residual: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResponse {
    #[serde(flatten)]
    pub run: dto::EvalResponse,
    pub selection_rule: &'static str,
    pub message: &'static str,
    pub expression: Option<Expression>,
}
pub async fn explain_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(req): ApiJson<ExplainRequest>,
) -> Result<Json<ExplainResponse>, ApiError> {
    let source = req
        .scenario
        .source
        .as_ref()
        .ok_or_else(|| ApiError::InvalidInput("explain requires scenario.source".into()))?;
    let range = req.selection.range(source)?;
    let enabled = state.cfg.cost_trace;
    state
        .engine
        .run(move || explain(req.scenario, range, enabled))
        .await
        .ok_or(ApiError::Internal)?
        .map(Json)
        .map_err(|e| ApiError::InvalidInput(e.to_string()))
}
fn explain(
    sc: Scenario,
    (start, end, whole): (u32, u32, bool),
    enabled: bool,
) -> Result<ExplainResponse, ergo_sandbox::SandboxError> {
    let (run, positions) = super::eval::run(&sc, enabled)?;
    let source = sc.source.as_deref().unwrap_or("");
    let rule = if whole {
        "whole-contract-root"
    } else {
        "first-start-in-selection-then-innermost"
    };
    let mut result = ExplainResponse {
        run,
        selection_rule: rule,
        message: "no evaluated expression starts in this selection",
        expression: None,
    };
    if positions.map.is_none() {
        result.message = "source map unavailable or misaligned; no expression was attributed";
        return Ok(result);
    }
    // Whole-document selection explicitly denotes the tree root, including
    // a compiler-synthesized Block with no source offset. No span is invented.
    let id = if whole {
        Some(0)
    } else {
        positions
            .map
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(_, off)| *off >= start && *off < end)
            .min_by_key(|(id, off)| {
                let size = preorder(&positions.tree.body)
                    .find(|(i, _)| i == id)
                    .map(|(_, e)| preorder(e).count())
                    .unwrap_or(usize::MAX);
                (*off, size, *id)
            })
            .map(|(id, _)| id)
    };
    let Some(id) = id else { return Ok(result) };
    let values: Vec<_> = result.run.values.iter().enumerate().filter(|(_,v)|v.ir_id==id).map(|(trace_index,v)| {
        let truncated=v.value.ends_with("...");
        let residual=if id==0 { result.run.reduced_to.clone() } else { residual_from_trace(&v.value) };
        let residual_note=if residual.is_none() && v.value.starts_with("SigmaProp(") {
            Some("the engine supplied only a truncated or unsupported sigma rendering; no complete residual was inferred")
        } else { None };
        ExplainedValue { trace_index,value:v.value.clone(),truncated,residual,residual_note }
    }).collect();
    if values.is_empty() && (!whole || result.run.reduced_to.is_none()) {
        return Ok(result);
    }
    let opcode = preorder(&positions.tree.body)
        .find(|(i, _)| *i == id)
        .map(|(_, e)| node_opcode(e))
        .unwrap();
    result.expression = Some(Expression {
        ir_id: id,
        opcode: format!(
            "{} (0x{opcode:02X})",
            ergo_sandbox::inspect::opcode_name(opcode).unwrap_or("Constant")
        ),
        span: positions.position(source, id),
        values,
        residual: if id == 0 {
            result.run.reduced_to.clone()
        } else {
            None
        },
    });
    result.message = "values recorded by this synthetic sandbox reduction";
    Ok(result)
}

/// Decode only COMPLETE pinned-engine debug renderings, then use the same
/// printer as reducedTo. This is presentation, never another evaluation.
/// The 150-byte recorder truncates most compound propositions: refuse them.
fn residual_from_trace(value: &str) -> Option<String> {
    let inner = value.strip_prefix("SigmaProp(")?.strip_suffix(')')?;
    let mut parser = SigmaText(inner);
    let sb = parser.proposition()?;
    if !parser.0.is_empty() {
        return None;
    }
    Some(ergo_sandbox::inspect::sigma_boolean_pretty(&sb))
}
struct SigmaText<'a>(&'a str);
impl SigmaText<'_> {
    fn eat(&mut self, token: &str) -> bool {
        if let Some(rest) = self.0.strip_prefix(token) {
            self.0 = rest;
            true
        } else {
            false
        }
    }
    fn proposition(&mut self) -> Option<ergo_ser::sigma_value::SigmaBoolean> {
        use ergo_ser::sigma_value::SigmaBoolean as S;
        if self.eat("TrivialProp(true)") {
            return Some(S::TrivialProp(true));
        }
        if self.eat("TrivialProp(false)") {
            return Some(S::TrivialProp(false));
        }
        if self.eat("ProveDlog(GroupElement(") {
            let hex = self.0.get(..66)?;
            let bytes: [u8; 33] = hex::decode(hex).ok()?.try_into().ok()?;
            self.0 = &self.0[66..];
            return self.eat("))").then(|| {
                S::ProveDlog(ergo_primitives::group_element::GroupElement::from_bytes(
                    bytes,
                ))
            });
        }
        let and = if self.eat("Cand([") {
            true
        } else if self.eat("Cor([") {
            false
        } else {
            return None;
        };
        let mut children = Vec::new();
        if !self.eat("])") {
            loop {
                children.push(self.proposition()?);
                if self.eat("])") {
                    break;
                }
                if !self.eat(", ") {
                    return None;
                }
            }
        }
        Some(if and {
            S::Cand(children)
        } else {
            S::Cor(children)
        })
    }
}
#[cfg(test)]
mod tests {
    use super::residual_from_trace;
    #[test]
    fn residual_rendering_is_lossless_or_explicitly_unavailable() {
        use ergo_primitives::group_element::GroupElement;
        use ergo_ser::sigma_value::SigmaBoolean as S;
        for sb in [
            S::TrivialProp(true),
            S::ProveDlog(GroupElement::from_bytes([2; 33])),
            S::Cand(vec![S::TrivialProp(true), S::TrivialProp(false)]),
        ] {
            assert_eq!(
                residual_from_trace(&format!("SigmaProp({sb:?})")),
                Some(ergo_sandbox::inspect::sigma_boolean_pretty(&sb))
            );
        }
        for s in [
            "SigmaProp(Cand([ProveDlog(GroupElement(02...",
            "Bool(true)",
            "SigmaProp(TrivialProp(true))trailing",
        ] {
            assert_eq!(residual_from_trace(s), None);
        }
    }
}
