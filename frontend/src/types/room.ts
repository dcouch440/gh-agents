// ============================================================================
// Room Types
// ============================================================================

export type Room = {
  id: string
  user_id: string
  collection_id: string | null
  name: string
  gatekeeper_enabled: boolean
  gatekeeper_model_id: string
  max_speakers_per_turn: number
  max_turns: number
  tools_enabled: boolean
  created_at: string
  updated_at: string
}

export type RoomMember = {
  room_id: string
  agent_id: string
  display_name: string | null
  role_description: string
  display_order: number
}

export type RoomSession = {
  id: string
  room_id: string
  run_id: string | null
  status: string
  current_turn: number
  transcript_summary: string | null
  started_at: string
  completed_at: string | null
}

export type RoomTranscriptEntry = {
  agent_name: string
  role_description: string
  content: string
  speaker_order: number | null
  created_at: string
}

export type RoomOutput = {
  id: string
  agent_id: string
  speaker_order: number
  turn_number: number
  output_name: string
  structured_output: Record<string, unknown>
  raw_output: string
}

export type CreateRoomRequest = {
  collection_id?: string | null
  name: string
  gatekeeper_enabled?: boolean
  gatekeeper_model_id?: string
  max_speakers_per_turn?: number
  max_turns?: number
  tools_enabled?: boolean
}

export type UpdateRoomRequest = {
  name?: string
  gatekeeper_enabled?: boolean
  gatekeeper_model_id?: string
  max_speakers_per_turn?: number
  max_turns?: number
  tools_enabled?: boolean
}

export type AddRoomMemberRequest = {
  agent_id: string
  display_name?: string | null
  role_description: string
  display_order?: number
}

export type SetRoomMembersRequest = {
  members: AddRoomMemberRequest[]
}

export type RoomMessageRequest = {
  content: string
}
