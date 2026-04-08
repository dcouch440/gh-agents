//! Canvas live sync — processes canvas change messages from the frontend.
//!
//! Each handler: verifies ownership -> resolves element -> mutates DB -> syncs filesystem -> broadcasts.

mod filesystem;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use tracing::info;
use uuid::Uuid;

use crate::server::services::ServiceError;
use crate::server::state::AppState;
use crate::server::ws::events::{ClientMessage, WorkflowEvent, WorkflowEventKind};
use crate::types::UserId;

/// Dispatch a canvas mutation message to the appropriate handler.
pub async fn handle_canvas_message(
    msg: ClientMessage,
    state: &AppState,
    user_id: Option<UserId>,
) -> Result<(), ServiceError> {
    let uid = user_id
        .ok_or_else(|| ServiceError::Forbidden("No user ID on WebSocket connection".to_string()))?;

    match msg {
        ClientMessage::CanvasElementMoved {
            workflow_id,
            element_id,
            x,
            y,
            width,
            height,
        } => {
            process_element_moved(state, uid.0, workflow_id, &element_id, x, y, width, height).await
        }
        ClientMessage::CanvasTextChanged {
            workflow_id,
            element_id,
            text,
        } => process_text_changed(state, uid.0, workflow_id, &element_id, &text).await,
        ClientMessage::CanvasNodeCreated {
            workflow_id,
            element_id,
            x,
            y,
            width,
            height,
            text,
        } => {
            process_node_created(
                state,
                uid.0,
                workflow_id,
                &element_id,
                x,
                y,
                width,
                height,
                &text,
            )
            .await
        }
        ClientMessage::CanvasEdgeCreated {
            workflow_id,
            element_id,
            source_element_id,
            target_element_id,
        } => {
            process_edge_created(
                state,
                uid.0,
                workflow_id,
                &element_id,
                &source_element_id,
                &target_element_id,
            )
            .await
        }
        ClientMessage::CanvasNodeDeleted {
            workflow_id,
            element_id,
        } => process_node_deleted(state, uid.0, workflow_id, &element_id).await,
        ClientMessage::CanvasEdgeDeleted {
            workflow_id,
            element_id,
        } => process_edge_deleted(state, uid.0, workflow_id, &element_id).await,
        _ => Ok(()), // Non-canvas messages — should never reach here
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_element_moved(
    state: &AppState,
    _user_id: Uuid,
    workflow_id: Uuid,
    element_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), ServiceError> {
    let repo = &*state.repos().workflows;

    // Resolve element_id -> step_id
    let maps = repo
        .list_element_maps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let map_row = maps.iter().find(|m| m.element_id == element_id);
    let step_id = match map_row.and_then(|m| m.step_id) {
        Some(id) => id,
        None => return Ok(()), // No mapping — element not yet persisted
    };

    // Update position in DB
    let mut step = repo
        .get_step(step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    step.position_x = Some(x);
    step.position_y = Some(y);
    step.width = Some(width);
    step.height = Some(height);
    step.display_order = ((y * 10_000.0) + x) as i32;
    repo.update_step(step)
        .await
        .map_err(ServiceError::Internal)?;

    Ok(())
}

async fn process_text_changed(
    state: &AppState,
    _user_id: Uuid,
    workflow_id: Uuid,
    element_id: &str,
    text: &str,
) -> Result<(), ServiceError> {
    let repo = &*state.repos().workflows;

    let maps = repo
        .list_element_maps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let map_row = maps.iter().find(|m| m.element_id == element_id);
    let step_id = match map_row.and_then(|m| m.step_id) {
        Some(id) => id,
        None => {
            tracing::warn!(
                workflow_id = %workflow_id,
                element_id = %element_id,
                map_count = maps.len(),
                "canvas_text_changed: no element map found — text edit dropped"
            );
            return Ok(());
        }
    };

    let mut step = repo
        .get_step(step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    step.description = text.to_string();
    repo.update_step(step.clone())
        .await
        .map_err(ServiceError::Internal)?;

    // Sync to filesystem
    if let Some(ref slug) = step.ref_id {
        let base_dir =
            crate::server::services::workflow_agent::resolve_base_dir(state, workflow_id);
        let _ = filesystem::write_node_file(&base_dir, slug, text);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_node_created(
    state: &AppState,
    user_id: Uuid,
    workflow_id: Uuid,
    element_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    text: &str,
) -> Result<(), ServiceError> {
    let repo = &*state.repos().workflows;
    use crate::server::services::workflow_agent::{next_unnamed_slug, slug_to_display_name};

    // Generate slug
    let existing_steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let existing_slugs: Vec<&str> = existing_steps
        .iter()
        .filter_map(|s| s.ref_id.as_deref())
        .collect();
    let slug = next_unnamed_slug(&existing_slugs);
    let display_name = slug_to_display_name(&slug);

    // Use the frontend's element_id as the step_id so IDs match everywhere —
    // no element map mismatch after get_board_elements regenerates boxes.
    let step_id = element_id
        .parse::<Uuid>()
        .unwrap_or_else(|_| Uuid::new_v4());

    // Create step
    let step = crate::db::WorkflowStepRow {
        id: step_id,
        workflow_id,
        agent_id: None,
        execution_mode: "workforce".to_string(),
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: String::new(),
        output_schema_id: None,
        output_variable_name: None,
        interactive_agent_id: None,
        for_each_label_field: None,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: ((y * 10_000.0) + x) as i32,
        version: 1,
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: Some(x),
        position_y: Some(y),
        width: Some(width),
        height: Some(height),
        name: Some(display_name.clone()),
        system_prompt_suffix: None,
        visible: true,
        description: text.to_string(),
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        child_workflow_id: None,
        ref_id: Some(slug.clone()),
        pinned: false,
        run_results_summary: String::new(),
        designer_handoff: String::new(),
    };

    let created = repo
        .create_step(step)
        .await
        .map_err(ServiceError::Internal)?;

    // Insert element map
    let map_row = crate::db::CanvasElementMapRow {
        workflow_id,
        element_id: element_id.to_string(),
        step_id: Some(created.id),
        edge_id: None,
        created_at: chrono::Utc::now(),
    };
    repo.upsert_element_map(map_row)
        .await
        .map_err(ServiceError::Internal)?;

    // Sync filesystem
    let base_dir = crate::server::services::workflow_agent::resolve_base_dir(state, workflow_id);
    let _ = filesystem::write_node_file(&base_dir, &slug, text);

    // Rewrite topology.json
    let all_steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let all_edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let _ = filesystem::rewrite_topology(&base_dir, &all_steps, &all_edges);

    // Broadcast
    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id,
        user_id: Some(user_id),
        kind: WorkflowEventKind::StepCreated {
            step_id: created.id,
            name: display_name,
        },
    });

    info!(workflow_id = %workflow_id, slug = %slug, "Canvas node created");
    Ok(())
}

async fn process_edge_created(
    state: &AppState,
    user_id: Uuid,
    workflow_id: Uuid,
    element_id: &str,
    source_element_id: &str,
    target_element_id: &str,
) -> Result<(), ServiceError> {
    let repo = &*state.repos().workflows;

    // Resolve source and target element IDs to step IDs
    let maps = repo
        .list_element_maps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let from_step_id = maps
        .iter()
        .find(|m| m.element_id == source_element_id)
        .and_then(|m| m.step_id)
        .ok_or_else(|| ServiceError::not_found("Source step for edge"))?;
    let to_step_id = maps
        .iter()
        .find(|m| m.element_id == target_element_id)
        .and_then(|m| m.step_id)
        .ok_or_else(|| ServiceError::not_found("Target step for edge"))?;

    // Create edge
    let edge = repo
        .add_edge(workflow_id, from_step_id, to_step_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Insert element map
    let map_row = crate::db::CanvasElementMapRow {
        workflow_id,
        element_id: element_id.to_string(),
        step_id: None,
        edge_id: Some(edge.id),
        created_at: chrono::Utc::now(),
    };
    repo.upsert_element_map(map_row)
        .await
        .map_err(ServiceError::Internal)?;

    // Rewrite topology.json
    let base_dir = crate::server::services::workflow_agent::resolve_base_dir(state, workflow_id);
    let all_steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let all_edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let _ = filesystem::rewrite_topology(&base_dir, &all_steps, &all_edges);

    // Broadcast
    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id,
        user_id: Some(user_id),
        kind: WorkflowEventKind::EdgeCreated {
            edge_id: edge.id,
            from_step_id,
            to_step_id,
        },
    });

    info!(workflow_id = %workflow_id, "Canvas edge created");
    Ok(())
}

async fn process_node_deleted(
    state: &AppState,
    user_id: Uuid,
    workflow_id: Uuid,
    element_id: &str,
) -> Result<(), ServiceError> {
    let repo = &*state.repos().workflows;

    let maps = repo
        .list_element_maps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let map_row = maps.iter().find(|m| m.element_id == element_id);
    let step_id = match map_row.and_then(|m| m.step_id) {
        Some(id) => id,
        None => return Ok(()),
    };

    // Get slug before deleting
    let step = repo
        .get_step(step_id)
        .await
        .map_err(ServiceError::Internal)?;
    let slug = step.as_ref().and_then(|s| s.ref_id.clone());

    // Delete step (cascades edges via FK)
    repo.delete_step(step_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Delete element map
    repo.delete_element_map(workflow_id, element_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Sync filesystem
    let base_dir = crate::server::services::workflow_agent::resolve_base_dir(state, workflow_id);
    if let Some(ref s) = slug {
        let _ = filesystem::remove_node_file(&base_dir, s);
    }
    let all_steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let all_edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let _ = filesystem::rewrite_topology(&base_dir, &all_steps, &all_edges);

    // Broadcast
    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id,
        user_id: Some(user_id),
        kind: WorkflowEventKind::StepDeleted { step_id },
    });

    info!(workflow_id = %workflow_id, "Canvas node deleted");
    Ok(())
}

async fn process_edge_deleted(
    state: &AppState,
    user_id: Uuid,
    workflow_id: Uuid,
    element_id: &str,
) -> Result<(), ServiceError> {
    let repo = &*state.repos().workflows;

    let maps = repo
        .list_element_maps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let map_row = maps.iter().find(|m| m.element_id == element_id);
    let edge_id = match map_row.and_then(|m| m.edge_id) {
        Some(id) => id,
        None => return Ok(()),
    };

    // Get edge details for broadcast
    let edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let edge = edges.iter().find(|e| e.id == edge_id);
    let (from_step_id, to_step_id) = match edge {
        Some(e) => (e.from_step_id, e.to_step_id),
        None => return Ok(()),
    };

    // Delete edge
    repo.remove_edge(from_step_id, to_step_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Delete element map
    repo.delete_element_map(workflow_id, element_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Rewrite topology.json
    let base_dir = crate::server::services::workflow_agent::resolve_base_dir(state, workflow_id);
    let all_steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let all_edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let _ = filesystem::rewrite_topology(&base_dir, &all_steps, &all_edges);

    // Broadcast
    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id,
        user_id: Some(user_id),
        kind: WorkflowEventKind::EdgeDeleted {
            edge_id,
            from_step_id,
            to_step_id,
        },
    });

    info!(workflow_id = %workflow_id, "Canvas edge deleted");
    Ok(())
}
