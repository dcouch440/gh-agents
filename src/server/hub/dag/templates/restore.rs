//! Restore a workflow's live state from a frozen template snapshot.
//!
//! This is the inverse of `capture_workflow_snapshot()`. It replaces all
//! workflow steps, edges, ports, routing rules, protocols, room configs,
//! mission briefs, agent rosters, and agents with the snapshot's frozen state.
//!
//! The entire operation runs in a single Postgres transaction for atomicity.

use sqlx::PgPool;
use uuid::Uuid;

use super::WorkflowSnapshot;

/// Replace the live workflow definition with the snapshot's frozen state.
///
/// All existing steps (and their cascaded children) are deleted, then the
/// snapshot's entities are inserted in dependency order. Agents and tools
/// are upserted since they may be shared across workflows.
pub(crate) async fn restore_workflow_from_snapshot(
    pool: &PgPool,
    workflow_id: Uuid,
    snapshot: &WorkflowSnapshot,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    // ── Delete phase ────────────────────────────────────────────────────
    // All child tables have ON DELETE CASCADE from workflow_steps(id),
    // so this single DELETE cascades to: step_inputs, step_outputs,
    // step_routing_rules, step_documents, workflow_step_protocols,
    // protocol_document_defs, room_step_configs, room_step_members,
    // task_mission_briefs (→ task_agent_roster), assistant_notes,
    // beliefs, agent_designer_runs, and workflow_step_edges.
    sqlx::query("DELETE FROM workflow_steps WHERE workflow_id = $1")
        .bind(workflow_id)
        .execute(&mut *tx)
        .await?;

    // ── Insert phase: agents + tools (shared, upserted) ─────────────────
    for agent in snapshot.agents.values() {
        sqlx::query(
            r#"INSERT INTO agents (id, user_id, name, system_prompt, persona_style, model_provider,
                model_id, model_max_tokens, model_temperature, status, router_mode,
                output_schema_id, default_reasoning_trace, is_system)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                system_prompt = EXCLUDED.system_prompt,
                persona_style = EXCLUDED.persona_style,
                model_provider = EXCLUDED.model_provider,
                model_id = EXCLUDED.model_id,
                model_max_tokens = EXCLUDED.model_max_tokens,
                model_temperature = EXCLUDED.model_temperature,
                status = EXCLUDED.status,
                router_mode = EXCLUDED.router_mode,
                output_schema_id = EXCLUDED.output_schema_id,
                default_reasoning_trace = EXCLUDED.default_reasoning_trace,
                is_system = EXCLUDED.is_system,
                version = agents.version + 1"#,
        )
        .bind(agent.id)
        .bind(agent.user_id)
        .bind(&agent.name)
        .bind(&agent.system_prompt)
        .bind(&agent.persona_style)
        .bind(&agent.model_provider)
        .bind(&agent.model_id)
        .bind(agent.model_max_tokens)
        .bind(agent.model_temperature)
        .bind(&agent.status)
        .bind(agent.router_mode)
        .bind(agent.output_schema_id)
        .bind(agent.default_reasoning_trace)
        .bind(agent.is_system)
        .execute(&mut *tx)
        .await?;
    }

    for (agent_id, tools) in &snapshot.agent_tools {
        // Clear existing tool assignments, then re-insert from snapshot
        sqlx::query("DELETE FROM agent_tools WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;

        for tool in tools {
            // Upsert the tool definition
            sqlx::query(
                r#"INSERT INTO tools (id, name, display_name, description, parameters)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    display_name = EXCLUDED.display_name,
                    description = EXCLUDED.description,
                    parameters = EXCLUDED.parameters,
                    version = tools.version + 1"#,
            )
            .bind(tool.id)
            .bind(&tool.name)
            .bind(&tool.display_name)
            .bind(&tool.description)
            .bind(&tool.parameters)
            .execute(&mut *tx)
            .await?;

            // Link agent → tool
            sqlx::query("INSERT INTO agent_tools (agent_id, tool_id) VALUES ($1, $2)")
                .bind(agent_id)
                .bind(tool.id)
                .execute(&mut *tx)
                .await?;
        }
    }

    // ── Insert phase: steps ─────────────────────────────────────────────
    for step in &snapshot.steps {
        sqlx::query(
            r#"INSERT INTO workflow_steps (
                id, workflow_id, agent_id, execution_mode, agent_execution_mode,
                for_each_ref, prompt_template_id, prompt_template,
                output_schema_id, output_variable_name, interactive_agent_id,
                for_each_label_field, room_id, routing_mode, routing_field,
                display_order, version, reasoning_trace, verification_agent_ids,
                position_x, position_y, width, height, name,
                system_prompt_suffix, visible, description,
                board_context_cache, board_context_updated_at,
                goal_summary, goal_summary_updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31
            )"#,
        )
        .bind(step.id)
        .bind(workflow_id) // force to target workflow
        .bind(step.agent_id)
        .bind(&step.execution_mode)
        .bind(&step.agent_execution_mode)
        .bind(&step.for_each_ref)
        .bind(step.prompt_template_id)
        .bind(&step.prompt_template)
        .bind(step.output_schema_id)
        .bind(&step.output_variable_name)
        .bind(step.interactive_agent_id)
        .bind(&step.for_each_label_field)
        .bind(step.room_id)
        .bind(&step.routing_mode)
        .bind(&step.routing_field)
        .bind(step.display_order)
        .bind(step.version)
        .bind(step.reasoning_trace)
        .bind(&step.verification_agent_ids)
        .bind(step.position_x)
        .bind(step.position_y)
        .bind(step.width)
        .bind(step.height)
        .bind(&step.name)
        .bind(&step.system_prompt_suffix)
        .bind(step.visible)
        .bind(&step.description)
        .bind(&step.board_context_cache)
        .bind(step.board_context_updated_at)
        .bind(&step.goal_summary)
        .bind(step.goal_summary_updated_at)
        .execute(&mut *tx)
        .await?;
    }

    // ── Insert phase: edges ─────────────────────────────────────────────
    for edge in &snapshot.edges {
        sqlx::query(
            r#"INSERT INTO workflow_step_edges (
                id, workflow_id, from_step_id, to_step_id,
                from_output_port, to_input_port, transform_jsonpath,
                condition_type, condition_value, edge_label
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
        )
        .bind(edge.id)
        .bind(workflow_id)
        .bind(edge.from_step_id)
        .bind(edge.to_step_id)
        .bind(&edge.from_output_port)
        .bind(&edge.to_input_port)
        .bind(&edge.transform_jsonpath)
        .bind(&edge.condition_type)
        .bind(&edge.condition_value)
        .bind(&edge.edge_label)
        .execute(&mut *tx)
        .await?;
    }

    // ── Insert phase: step inputs ───────────────────────────────────────
    for inputs in snapshot.step_inputs.values() {
        for input in inputs {
            sqlx::query(
                r#"INSERT INTO step_inputs (id, workflow_step_id, port_name, port_type,
                    required, default_value, description, json_schema)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
            )
            .bind(input.id)
            .bind(input.workflow_step_id)
            .bind(&input.port_name)
            .bind(&input.port_type)
            .bind(input.required)
            .bind(&input.default_value)
            .bind(&input.description)
            .bind(&input.json_schema)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── Insert phase: step outputs ──────────────────────────────────────
    for outputs in snapshot.step_outputs.values() {
        for output in outputs {
            sqlx::query(
                r#"INSERT INTO step_outputs (id, workflow_step_id, port_name, port_type,
                    json_path, description, json_schema)
                VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            )
            .bind(output.id)
            .bind(output.workflow_step_id)
            .bind(&output.port_name)
            .bind(&output.port_type)
            .bind(&output.json_path)
            .bind(&output.description)
            .bind(&output.json_schema)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── Insert phase: routing rules ─────────────────────────────────────
    for rules in snapshot.routing_rules.values() {
        for rule in rules {
            sqlx::query(
                r#"INSERT INTO step_routing_rules (id, workflow_step_id, label_value,
                    description, agent_id, display_order)
                VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(rule.id)
            .bind(rule.workflow_step_id)
            .bind(&rule.label_value)
            .bind(&rule.description)
            .bind(rule.agent_id)
            .bind(rule.display_order)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── Insert phase: document definitions ──────────────────────────────
    for defs in snapshot.document_defs.values() {
        for def in defs {
            sqlx::query(
                r#"INSERT INTO protocol_document_defs (id, step_id, name, description,
                    target_length, display_order, protocol_id, document_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
            )
            .bind(def.id)
            .bind(def.step_id)
            .bind(&def.name)
            .bind(&def.description)
            .bind(def.target_length)
            .bind(def.display_order)
            .bind(def.protocol_id)
            .bind(def.document_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── Insert phase: workflow step protocols ────────────────────────────
    // Skip gracefully if the referenced protocol_id no longer exists (FK).
    for proto in snapshot.protocols.values() {
        let result = sqlx::query(
            r#"INSERT INTO workflow_step_protocols (id, workflow_step_id, protocol_id, applied_expansion)
            VALUES ($1, $2, $3, $4)"#,
        )
        .bind(proto.id)
        .bind(proto.workflow_step_id)
        .bind(proto.protocol_id)
        .bind(&proto.applied_expansion)
        .execute(&mut *tx)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                step_id = %proto.workflow_step_id,
                protocol_id = %proto.protocol_id,
                "Skipping protocol linkage during rebase (FK missing?): {e}"
            );
        }
    }

    // ── Insert phase: room configs ──────────────────────────────────────
    for config in snapshot.room_configs.values() {
        sqlx::query(
            r#"INSERT INTO room_step_configs (id, step_id, meeting_purpose, max_turns,
                interaction_mode, gatekeeper_enabled)
            VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(config.id)
        .bind(config.step_id)
        .bind(&config.meeting_purpose)
        .bind(config.max_turns)
        .bind(&config.interaction_mode)
        .bind(config.gatekeeper_enabled)
        .execute(&mut *tx)
        .await?;
    }

    // ── Insert phase: room members ──────────────────────────────────────
    for members in snapshot.room_members.values() {
        for member in members {
            sqlx::query(
                r#"INSERT INTO room_step_members (id, step_id, name, role, perspective, display_order)
                VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(member.id)
            .bind(member.step_id)
            .bind(&member.name)
            .bind(&member.role)
            .bind(&member.perspective)
            .bind(member.display_order)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── Insert phase: mission briefs ────────────────────────────────────
    for brief in snapshot.mission_briefs.values() {
        sqlx::query(
            r#"INSERT INTO task_mission_briefs (id, step_id, task_description,
                available_capabilities, failure_mode, downstream_context)
            VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(brief.id)
        .bind(brief.step_id)
        .bind(&brief.task_description)
        .bind(&brief.available_capabilities)
        .bind(&brief.failure_mode)
        .bind(&brief.downstream_context)
        .execute(&mut *tx)
        .await?;
    }

    // ── Insert phase: agent rosters ─────────────────────────────────────
    for roster in snapshot.agent_rosters.values() {
        for entry in roster {
            sqlx::query(
                r#"INSERT INTO task_agent_roster (id, mission_brief_id, name,
                    role_description, capabilities, execution_order)
                VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(entry.id)
            .bind(entry.mission_brief_id)
            .bind(&entry.name)
            .bind(&entry.role_description)
            .bind(&entry.capabilities)
            .bind(entry.execution_order)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    tracing::info!(
        workflow_id = %workflow_id,
        steps = snapshot.steps.len(),
        edges = snapshot.edges.len(),
        agents = snapshot.agents.len(),
        "Workflow restored from snapshot"
    );

    Ok(())
}
