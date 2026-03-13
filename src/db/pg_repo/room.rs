use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::db::traits::{
    CreateRoomInput, RoomMemberInput, RoomRepo, SaveRoomExecutionOutputInput, UpdateRoomInput,
};
use crate::db::{
    RoomExecutionOutputRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomTranscriptEntry,
};

use super::PgRepo;

#[async_trait]
impl RoomRepo for PgRepo {
    async fn create_room(&self, input: CreateRoomInput) -> Result<RoomRow> {
        let row = sqlx::query_as::<_, RoomRow>(
            "INSERT INTO rooms (user_id, collection_id, name, gatekeeper_enabled, gatekeeper_model_id, max_speakers_per_turn, max_turns, tools_enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(input.user_id)
        .bind(input.collection_id)
        .bind(&input.name)
        .bind(input.gatekeeper_enabled)
        .bind(&input.gatekeeper_model_id)
        .bind(input.max_speakers_per_turn)
        .bind(input.max_turns)
        .bind(input.tools_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_room(&self, id: Uuid) -> Result<Option<RoomRow>> {
        let row = sqlx::query_as::<_, RoomRow>("SELECT * FROM rooms WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn update_room(&self, input: UpdateRoomInput) -> Result<RoomRow> {
        let row = sqlx::query_as::<_, RoomRow>(
            "UPDATE rooms SET \
                name = COALESCE($2, name), \
                gatekeeper_enabled = COALESCE($3, gatekeeper_enabled), \
                gatekeeper_model_id = COALESCE($4, gatekeeper_model_id), \
                max_speakers_per_turn = COALESCE($5, max_speakers_per_turn), \
                max_turns = COALESCE($6, max_turns), \
                tools_enabled = COALESCE($7, tools_enabled), \
                updated_at = NOW() \
            WHERE id = $1 RETURNING *",
        )
        .bind(input.id)
        .bind(input.name)
        .bind(input.gatekeeper_enabled)
        .bind(input.gatekeeper_model_id)
        .bind(input.max_speakers_per_turn)
        .bind(input.max_turns)
        .bind(input.tools_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_room(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Room members ---

    async fn list_room_members(&self, room_id: Uuid) -> Result<Vec<RoomMemberRow>> {
        let rows = sqlx::query_as::<_, RoomMemberRow>("SELECT room_id, agent_id, display_name, role_description, display_order FROM room_members WHERE room_id = $1 ORDER BY display_order ASC")
            .bind(room_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn add_room_member(
        &self,
        room_id: Uuid,
        agent_id: Uuid,
        display_name: Option<String>,
        role_description: String,
        display_order: i32,
    ) -> Result<()> {
        sqlx::query("INSERT INTO room_members (room_id, agent_id, display_name, role_description, display_order) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING")
            .bind(room_id)
            .bind(agent_id)
            .bind(display_name)
            .bind(role_description)
            .bind(display_order)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_room_member(&self, room_id: Uuid, agent_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM room_members WHERE room_id = $1 AND agent_id = $2")
            .bind(room_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn set_room_members(&self, room_id: Uuid, members: &[RoomMemberInput]) -> Result<()> {
        let members = members.to_vec();
        run_serializable!(self.pool, |tx| {
            sqlx::query("DELETE FROM room_members WHERE room_id = $1")
                .bind(room_id)
                .execute(&mut *tx)
                .await?;

            for member in &members {
                sqlx::query("INSERT INTO room_members (room_id, agent_id, display_name, role_description, display_order) VALUES ($1, $2, $3, $4, $5)")
                    .bind(room_id)
                    .bind(member.agent_id)
                    .bind(member.display_name.as_deref())
                    .bind(&member.role_description)
                    .bind(member.display_order)
                    .execute(&mut *tx)
                    .await?;
            }
            Ok(())
        })
    }

    // --- Room sessions ---

    async fn create_room_session(&self, room_id: Uuid) -> Result<RoomSessionRow> {
        let row = sqlx::query_as::<_, RoomSessionRow>(
            "INSERT INTO room_sessions (room_id) VALUES ($1) RETURNING *",
        )
        .bind(room_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_room_session(&self, id: Uuid) -> Result<Option<RoomSessionRow>> {
        let row = sqlx::query_as::<_, RoomSessionRow>("SELECT * FROM room_sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn update_room_session_status(&self, id: Uuid, status: &str) -> Result<()> {
        let completed_at = if status == "completed" || status == "cancelled" {
            Some(Utc::now())
        } else {
            None
        };
        sqlx::query("UPDATE room_sessions SET status = $2, completed_at = COALESCE($3, completed_at) WHERE id = $1")
            .bind(id)
            .bind(status)
            .bind(completed_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn increment_room_session_turn(&self, id: Uuid) -> Result<i32> {
        let row: (i32,) = sqlx::query_as("UPDATE room_sessions SET current_turn = current_turn + 1 WHERE id = $1 RETURNING current_turn")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn set_transcript_summary(&self, id: Uuid, summary: &str) -> Result<()> {
        sqlx::query("UPDATE room_sessions SET transcript_summary = $2 WHERE id = $1")
            .bind(id)
            .bind(summary)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Room transcript ---

    async fn get_room_transcript(&self, room_session_id: Uuid) -> Result<Vec<RoomTranscriptEntry>> {
        let rows = sqlx::query_as::<_, RoomTranscriptEntry>(
            "SELECT \
                COALESCE(rm.display_name, a.name) AS agent_name, \
                COALESCE(rm.role_description, '') AS role_description, \
                em.content, \
                ae.speaker_order, \
                em.created_at \
            FROM execution_messages em \
            JOIN agent_executions ae ON em.agent_execution_id = ae.id \
            JOIN agents a ON ae.agent_id = a.id \
            LEFT JOIN room_members rm ON rm.agent_id = ae.agent_id \
                AND rm.room_id = (SELECT room_id FROM room_sessions WHERE id = $1) \
            WHERE ae.room_session_id = $1 \
                AND em.role IN ('user', 'assistant') \
            ORDER BY em.created_at ASC",
        )
        .bind(room_session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Room Execution Outputs (Phase 3) ---

    async fn save_room_execution_output(
        &self,
        input: SaveRoomExecutionOutputInput,
    ) -> Result<RoomExecutionOutputRow> {
        let row = sqlx::query_as::<_, RoomExecutionOutputRow>(
            "INSERT INTO room_execution_outputs
             (room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at"
        )
        .bind(input.room_session_id)
        .bind(input.agent_execution_id)
        .bind(input.agent_id)
        .bind(input.speaker_order)
        .bind(input.turn_number)
        .bind(&input.output_name)
        .bind(&input.structured_output)
        .bind(&input.raw_output)
        .bind(input.schema_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_room_execution_outputs(
        &self,
        room_session_id: Uuid,
        turn_number: Option<i32>,
    ) -> Result<Vec<RoomExecutionOutputRow>> {
        let rows = if let Some(turn) = turn_number {
            sqlx::query_as::<_, RoomExecutionOutputRow>(
                "SELECT id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at
                 FROM room_execution_outputs
                 WHERE room_session_id = $1 AND turn_number = $2
                 ORDER BY speaker_order"
            )
            .bind(room_session_id)
            .bind(turn)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, RoomExecutionOutputRow>(
                "SELECT id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at
                 FROM room_execution_outputs
                 WHERE room_session_id = $1
                 ORDER BY turn_number, speaker_order"
            )
            .bind(room_session_id)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    async fn get_room_outputs_by_schema(
        &self,
        room_session_id: Uuid,
        schema_id: Uuid,
    ) -> Result<Vec<RoomExecutionOutputRow>> {
        let rows = sqlx::query_as::<_, RoomExecutionOutputRow>(
            "SELECT id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at
             FROM room_execution_outputs
             WHERE room_session_id = $1 AND schema_id = $2
             ORDER BY turn_number, speaker_order"
        )
        .bind(room_session_id)
        .bind(schema_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
