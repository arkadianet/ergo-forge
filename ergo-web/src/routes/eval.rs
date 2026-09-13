//! Evaluation and its diagnostics share exactly one `eval_scenario` call.
use crate::{app::AppState, dto, error::ApiError, extract::ApiJson};
use axum::{extract::State, Json};
use ergo_sandbox::{source_positions::SourcePositions, Scenario};

/// Shared by eval, cost-spans and explain inside the existing engine budget.
pub(crate) fn run(
    scenario: &Scenario,
    cost_trace: bool,
) -> Result<(dto::EvalResponse, SourcePositions), ergo_sandbox::SandboxError> {
    let outcome = ergo_sandbox::eval_scenario(scenario)?;
    let positions = SourcePositions::for_run(scenario, &outcome)?;
    #[cfg(feature = "cost-trace")]
    let spans = cost_trace.then(|| {
        ergo_sandbox::cost_spans::cost_spans(
            scenario.source.as_deref().unwrap_or(""),
            &positions.tree.body,
            positions.map.as_ref(),
            positions.status,
            &outcome.cost_breakdown,
        )
    });
    let mut response = dto::EvalResponse::from_engine(outcome);
    if !cost_trace {
        response.hot_spots.clear();
    }
    #[cfg(feature = "cost-trace")]
    {
        response.cost_spans = spans;
    }
    response.map_status = positions.status;
    if let Some(source) = &scenario.source {
        for value in &mut response.values {
            if let Some(span) = positions.position(source, value.ir_id) {
                value.offset = Some(span.offset);
                value.line = Some(span.line);
                value.col = Some(span.col);
            }
        }
    }
    Ok((response, positions))
}
pub async fn eval_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(scenario): ApiJson<Scenario>,
) -> Result<Json<dto::EvalResponse>, ApiError> {
    let enabled = state.cfg.cost_trace;
    state
        .engine
        .run(move || run(&scenario, enabled).map(|(response, _)| Json(response)))
        .await
        .ok_or(ApiError::Internal)?
        .map_err(|e| ApiError::InvalidInput(e.to_string()))
}
/// Same envelope as eval: cost and value diagnostics are from this run.
#[cfg(feature = "cost-trace")]
pub async fn cost_spans_route(
    state: State<std::sync::Arc<AppState>>,
    scenario: ApiJson<Scenario>,
) -> Result<Json<dto::EvalResponse>, ApiError> {
    eval_route(state, scenario).await
}
