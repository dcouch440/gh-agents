-- ============================================================================
-- Migration 0001: Initial Schema
-- ============================================================================
-- Purpose: Consolidated initial schema for nexor database
--
-- This migration represents the complete schema from 70+ incremental
-- migrations (001-071), consolidated into a single clean starting point.
--
-- Key features:
-- - Full agent, task, workflow, and execution tracking
-- - Port-based workflow system with label routing
-- - Document and tool management
-- - Session and room-based collaboration
-- - System configuration
--
-- Notable: step_routing_rules includes 'description' column for Phase 5B
-- downstream routing context injection (PHASE_5B_DOWNSTREAM_ROUTING_CONTEXT.md)
--
-- Date: 2026-02-05
-- Archived migrations: migrations_archive/ (001-071)
-- ============================================================================

-- PostgreSQL database dump
--


-- Dumped from database version 16.11
-- Dumped by pg_dump version 16.11

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: public; Type: SCHEMA; Schema: -; Owner: nexor
--

-- *not* creating schema, since initdb creates it


ALTER SCHEMA public OWNER TO nexor;

--
-- Name: SCHEMA public; Type: COMMENT; Schema: -; Owner: nexor
--

COMMENT ON SCHEMA public IS '';


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: agent_context; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agent_context (
    agent_id uuid NOT NULL,
    document_id uuid NOT NULL
);


ALTER TABLE public.agent_context OWNER TO nexor;

--
-- Name: agent_executions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agent_executions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    agent_id uuid NOT NULL,
    workflow_step_id uuid,
    is_interactive boolean DEFAULT false NOT NULL,
    parent_agent_execution_id uuid,
    system_prompt_rendered text NOT NULL,
    input text NOT NULL,
    output text,
    structured_output jsonb,
    status text DEFAULT 'running'::text NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    selected_mode_id uuid,
    room_session_id uuid,
    speaker_order integer,
    workflow_execution_id uuid,
    selected_router_mode_id uuid,
    routing_analysis jsonb,
    selected_routing_document_id uuid
);


ALTER TABLE public.agent_executions OWNER TO nexor;

--
-- Name: COLUMN agent_executions.structured_output; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.agent_executions.structured_output IS 'Standard output envelope: {status, data, metadata, error}.
     For execution_mode="for_each", data is an array of iteration envelopes.
     For single execution, data contains the actual output.';


--
-- Name: COLUMN agent_executions.routing_analysis; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.agent_executions.routing_analysis IS 'For cavernous routing executions: Document search results and selection reasoning.
     Format: {
       "search_query": "...",
       "documents_found": [{"id": "uuid", "title": "routing:...", "score": 0.95}],
       "selected_document_id": "uuid",
       "reasoning": "Selected because...",
       "collaborative_selection": false
     }';


--
-- Name: COLUMN agent_executions.selected_routing_document_id; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.agent_executions.selected_routing_document_id IS 'For cavernous routing: Reference to the routing config document that was selected and applied for this execution';


--
-- Name: agent_executions_backup; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agent_executions_backup (
    id uuid,
    stage_execution_id uuid,
    agent_id uuid,
    workflow_step_id uuid,
    is_interactive boolean,
    parent_agent_execution_id uuid,
    system_prompt_rendered text,
    input text,
    output text,
    structured_output jsonb,
    status text,
    started_at timestamp with time zone,
    completed_at timestamp with time zone,
    selected_mode_id uuid,
    room_session_id uuid,
    speaker_order integer
);


ALTER TABLE public.agent_executions_backup OWNER TO nexor;

--
-- Name: agent_modes; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agent_modes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    agent_id uuid NOT NULL,
    name text NOT NULL,
    system_prompt_suffix text,
    temperature_override double precision,
    model_override text,
    tool_overrides text[],
    classifier_hint text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    version integer DEFAULT 1 NOT NULL
);


ALTER TABLE public.agent_modes OWNER TO nexor;

--
-- Name: TABLE agent_modes; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.agent_modes IS 'DEPRECATED: Use tool_router_modes instead.
Migrate data via Phase 10 migration script.
Will be dropped after verification.
DO NOT add new agent_modes - use tool_router_modes.';


--
-- Name: agent_modes_versions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agent_modes_versions (
    id uuid NOT NULL,
    version integer NOT NULL,
    agent_id uuid NOT NULL,
    name text NOT NULL,
    system_prompt_suffix text,
    temperature_override double precision,
    model_override text,
    tool_overrides text[],
    classifier_hint text NOT NULL,
    changed_by uuid,
    changed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.agent_modes_versions OWNER TO nexor;

--
-- Name: agent_tools; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agent_tools (
    agent_id uuid NOT NULL,
    tool_id uuid NOT NULL
);


ALTER TABLE public.agent_tools OWNER TO nexor;

--
-- Name: agents; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agents (
    id uuid NOT NULL,
    name text NOT NULL,
    system_prompt text DEFAULT ''::text NOT NULL,
    persona_style text DEFAULT 'casual'::text,
    model_provider text DEFAULT 'anthropic'::text NOT NULL,
    model_id text NOT NULL,
    model_max_tokens integer DEFAULT 4096 NOT NULL,
    model_temperature real DEFAULT 0.7 NOT NULL,
    current_task uuid,
    status text DEFAULT 'idle'::text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    user_id uuid NOT NULL,
    router_mode boolean DEFAULT false,
    version integer DEFAULT 1 NOT NULL,
    output_schema_id uuid,
    router_id uuid
);


ALTER TABLE public.agents OWNER TO nexor;

--
-- Name: agents_versions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.agents_versions (
    id uuid NOT NULL,
    version integer NOT NULL,
    tier text,
    name text NOT NULL,
    system_prompt text NOT NULL,
    persona_style text,
    model_provider text NOT NULL,
    model_id text NOT NULL,
    model_max_tokens integer NOT NULL,
    model_temperature real NOT NULL,
    status text,
    router_mode boolean,
    changed_by uuid,
    changed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.agents_versions OWNER TO nexor;

--
-- Name: auth_config; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.auth_config (
    id integer NOT NULL,
    password_hash text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT auth_config_id_check CHECK ((id = 1))
);


ALTER TABLE public.auth_config OWNER TO nexor;

--
-- Name: chat_messages; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.chat_messages (
    id uuid NOT NULL,
    role text NOT NULL,
    content text NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    user_id uuid NOT NULL,
    session_id uuid,
    CONSTRAINT chat_messages_role_check CHECK ((role = ANY (ARRAY['user'::text, 'assistant'::text])))
);


ALTER TABLE public.chat_messages OWNER TO nexor;

--
-- Name: chat_sessions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.chat_sessions (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    mode_id text NOT NULL,
    title text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    summary text DEFAULT ''::text NOT NULL,
    agent_id uuid,
    draft_config jsonb
);


ALTER TABLE public.chat_sessions OWNER TO nexor;

--
-- Name: collection_runs; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.collection_runs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    collection_id uuid NOT NULL,
    user_id uuid NOT NULL,
    status text NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    error text
);


ALTER TABLE public.collection_runs OWNER TO nexor;

--
-- Name: collection_workflow_edges; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.collection_workflow_edges (
    from_workflow_id uuid NOT NULL,
    to_workflow_id uuid NOT NULL,
    collection_id uuid NOT NULL
);


ALTER TABLE public.collection_workflow_edges OWNER TO nexor;

--
-- Name: collection_workflows; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.collection_workflows (
    collection_id uuid NOT NULL,
    workflow_id uuid NOT NULL,
    display_order integer DEFAULT 0 NOT NULL,
    execution_mode text
);


ALTER TABLE public.collection_workflows OWNER TO nexor;

--
-- Name: context_store; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.context_store (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    session_id uuid NOT NULL,
    source text NOT NULL,
    priority real DEFAULT 0.5 NOT NULL,
    content text NOT NULL,
    metadata jsonb,
    status text DEFAULT 'active'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone
);


ALTER TABLE public.context_store OWNER TO nexor;

--
-- Name: documents; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.documents (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    session_id uuid,
    title text NOT NULL,
    content text DEFAULT ''::text NOT NULL,
    summary text DEFAULT ''::text,
    doc_type text DEFAULT 'architecture'::text,
    ref_tag text DEFAULT ''::text,
    tags text[] DEFAULT '{}'::text[],
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.documents OWNER TO nexor;

--
-- Name: execution_messages; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.execution_messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    agent_execution_id uuid NOT NULL,
    role text NOT NULL,
    content text NOT NULL,
    tool_call_id text,
    input_tokens bigint DEFAULT 0 NOT NULL,
    output_tokens bigint DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.execution_messages OWNER TO nexor;

--
-- Name: mode_required_capabilities; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.mode_required_capabilities (
    mode_id uuid NOT NULL,
    capability_id uuid NOT NULL,
    is_required boolean DEFAULT true NOT NULL
);


ALTER TABLE public.mode_required_capabilities OWNER TO nexor;

--
-- Name: TABLE mode_required_capabilities; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.mode_required_capabilities IS 'Defines which capabilities a mode requires. Mode resolver will auto-select tools providing these capabilities.';


--
-- Name: output_schemas; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.output_schemas (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    name text NOT NULL,
    schema jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    version integer DEFAULT 1 NOT NULL
);


ALTER TABLE public.output_schemas OWNER TO nexor;

--
-- Name: output_schemas_versions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.output_schemas_versions (
    id uuid NOT NULL,
    version integer NOT NULL,
    name text NOT NULL,
    schema jsonb NOT NULL,
    changed_by uuid,
    changed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.output_schemas_versions OWNER TO nexor;

--
-- Name: pipelines_backup; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.pipelines_backup (
    id uuid,
    user_id uuid,
    name text,
    created_at timestamp with time zone
);


ALTER TABLE public.pipelines_backup OWNER TO nexor;

--
-- Name: pr_merge_queue; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.pr_merge_queue (
    id uuid NOT NULL,
    repo_owner text NOT NULL,
    repo_name text NOT NULL,
    pr_number integer NOT NULL,
    queue_position integer NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    conflict_info text,
    error_message text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    user_id uuid NOT NULL
);


ALTER TABLE public.pr_merge_queue OWNER TO nexor;

--
-- Name: prompt_templates; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.prompt_templates (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    name text NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    version integer DEFAULT 1 NOT NULL
);


ALTER TABLE public.prompt_templates OWNER TO nexor;

--
-- Name: prompt_templates_versions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.prompt_templates_versions (
    id uuid NOT NULL,
    version integer NOT NULL,
    name text NOT NULL,
    content text NOT NULL,
    changed_by uuid,
    changed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.prompt_templates_versions OWNER TO nexor;

--
-- Name: results; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.results (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    agent_execution_id uuid NOT NULL,
    output_schema_id uuid,
    name text NOT NULL,
    data jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.results OWNER TO nexor;

--
-- Name: room_execution_outputs; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.room_execution_outputs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    room_session_id uuid NOT NULL,
    agent_execution_id uuid NOT NULL,
    agent_id uuid NOT NULL,
    speaker_order integer NOT NULL,
    turn_number integer NOT NULL,
    output_name text NOT NULL,
    structured_output jsonb NOT NULL,
    raw_output text NOT NULL,
    schema_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.room_execution_outputs OWNER TO nexor;

--
-- Name: TABLE room_execution_outputs; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.room_execution_outputs IS 'Structured outputs from room members for agent-to-agent data passing.
     Next speakers receive previous agents structured data, not just text transcripts.';


--
-- Name: room_members; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.room_members (
    room_id uuid NOT NULL,
    agent_id uuid NOT NULL,
    display_name text,
    role_description text NOT NULL,
    display_order integer DEFAULT 0 NOT NULL,
    input_schema_id uuid,
    output_schema_id uuid,
    output_name text
);


ALTER TABLE public.room_members OWNER TO nexor;

--
-- Name: COLUMN room_members.input_schema_id; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.room_members.input_schema_id IS 'Optional: Schema of structured inputs this agent can consume. Gatekeeper uses this for informed speaker selection.';


--
-- Name: COLUMN room_members.output_schema_id; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.room_members.output_schema_id IS 'Optional: Schema this agent produces. System validates output against this schema if present.';


--
-- Name: COLUMN room_members.output_name; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.room_members.output_name IS 'Semantic name for this agent''s output (e.g., "requirements_analysis", "architecture_plan"). Other agents reference this.';


--
-- Name: room_sessions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.room_sessions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    room_id uuid NOT NULL,
    status text DEFAULT 'active'::text NOT NULL,
    current_turn integer DEFAULT 0 NOT NULL,
    transcript_summary text,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    structured_outputs jsonb,
    final_decision jsonb
);


ALTER TABLE public.room_sessions OWNER TO nexor;

--
-- Name: COLUMN room_sessions.structured_outputs; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.room_sessions.structured_outputs IS 'Accumulated structured outputs from all speakers. Format: {
       "requirements": {"data": {...}, "agent_id": "...", "turn": 1},
       "architecture": {"data": {...}, "agent_id": "...", "turn": 2}
     }';


--
-- Name: COLUMN room_sessions.final_decision; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.room_sessions.final_decision IS 'Final aggregated output from the room session, determined by room.aggregation_mode';


--
-- Name: room_sessions_backup; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.room_sessions_backup (
    id uuid,
    room_id uuid,
    run_id uuid,
    status text,
    current_turn integer,
    transcript_summary text,
    started_at timestamp with time zone,
    completed_at timestamp with time zone
);


ALTER TABLE public.room_sessions_backup OWNER TO nexor;

--
-- Name: rooms; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.rooms (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    name text NOT NULL,
    gatekeeper_enabled boolean DEFAULT false NOT NULL,
    gatekeeper_model_id text DEFAULT 'claude-haiku-4-20250414'::text NOT NULL,
    max_speakers_per_turn integer DEFAULT 4 NOT NULL,
    max_turns integer DEFAULT 20 NOT NULL,
    tools_enabled boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    collection_id uuid,
    default_output_schema_id uuid,
    aggregation_mode text DEFAULT 'final_speaker'::text
);


ALTER TABLE public.rooms OWNER TO nexor;

--
-- Name: COLUMN rooms.default_output_schema_id; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.rooms.default_output_schema_id IS 'Default output schema for room members (can be overridden per member)';


--
-- Name: COLUMN rooms.aggregation_mode; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.rooms.aggregation_mode IS 'How to aggregate room outputs into final result:
     - "final_speaker": Use last speaker''s output
     - "consensus": Synthesize consensus from all speakers
     - "all_outputs": Return array of all speaker outputs';


--
-- Name: rooms_backup; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.rooms_backup (
    id uuid,
    user_id uuid,
    pipeline_id uuid,
    name text,
    gatekeeper_enabled boolean,
    gatekeeper_model_id text,
    max_speakers_per_turn integer,
    max_turns integer,
    tools_enabled boolean,
    created_at timestamp with time zone,
    updated_at timestamp with time zone
);


ALTER TABLE public.rooms_backup OWNER TO nexor;

--
-- Name: router_requests; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.router_requests (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    session_id uuid NOT NULL,
    agent_execution_id uuid,
    intent text NOT NULL,
    priority text DEFAULT 'normal'::text NOT NULL,
    callback_hint text,
    routed_tool text,
    routed_args jsonb,
    is_async boolean DEFAULT false NOT NULL,
    passdown text,
    chain jsonb,
    status text DEFAULT 'pending'::text NOT NULL,
    result text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone
);


ALTER TABLE public.router_requests OWNER TO nexor;

--
-- Name: step_documents; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.step_documents (
    step_id uuid NOT NULL,
    document_id uuid NOT NULL
);


ALTER TABLE public.step_documents OWNER TO nexor;

--
-- Name: step_inputs; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.step_inputs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    workflow_step_id uuid NOT NULL,
    port_name text NOT NULL,
    port_type text NOT NULL,
    required boolean DEFAULT false NOT NULL,
    default_value jsonb,
    description text,
    json_schema jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.step_inputs OWNER TO nexor;

--
-- Name: TABLE step_inputs; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.step_inputs IS 'Input port definitions for workflow steps. Defines what data each step requires to execute.';


--
-- Name: step_outputs; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.step_outputs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    workflow_step_id uuid NOT NULL,
    port_name text NOT NULL,
    port_type text NOT NULL,
    json_path text NOT NULL,
    description text,
    json_schema jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.step_outputs OWNER TO nexor;

--
-- Name: TABLE step_outputs; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.step_outputs IS 'Output port definitions for workflow steps. Defines what data each step produces and where to find it in the output envelope.';


--
-- Name: step_routing_rules; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.step_routing_rules (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    workflow_step_id uuid NOT NULL,
    label_value text NOT NULL,
    agent_id uuid NOT NULL,
    description text,
    display_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.step_routing_rules OWNER TO nexor;

--
-- Name: TABLE step_routing_rules; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.step_routing_rules IS 'Label-based routing configuration for for-each steps. Maps label/category values to specialist agents.';


--
-- Name: system_config; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.system_config (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    config_type text NOT NULL,
    config_key text NOT NULL,
    config_value jsonb NOT NULL,
    description text,
    created_by uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.system_config OWNER TO nexor;

--
-- Name: TABLE system_config; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.system_config IS 'Master system configuration (admin-controlled). Defines capabilities, execution constraints, routing strategies, and system agents.';


--
-- Name: tasks; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tasks (
    id uuid NOT NULL,
    slice_id uuid,
    title text NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    assigned_agent uuid,
    status text DEFAULT 'pending'::text NOT NULL,
    priority text DEFAULT 'normal'::text NOT NULL,
    context_files jsonb DEFAULT '[]'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    metadata jsonb,
    user_id uuid NOT NULL,
    retry_count integer DEFAULT 0 NOT NULL,
    max_retries integer DEFAULT 3 NOT NULL,
    last_error text
);


ALTER TABLE public.tasks OWNER TO nexor;

--
-- Name: token_ledger; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.token_ledger (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    agent_execution_id uuid,
    model_id text NOT NULL,
    input_tokens bigint NOT NULL,
    output_tokens bigint NOT NULL,
    cost_usd real NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.token_ledger OWNER TO nexor;

--
-- Name: tool_capabilities; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tool_capabilities (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    capability_key text NOT NULL,
    display_name text NOT NULL,
    category text NOT NULL,
    safety_level text DEFAULT 'safe'::text NOT NULL,
    description text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT tool_capabilities_capability_key_check CHECK ((capability_key ~ '^[a-z][a-z0-9_]*$'::text))
);


ALTER TABLE public.tool_capabilities OWNER TO nexor;

--
-- Name: TABLE tool_capabilities; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.tool_capabilities IS 'Semantic capability taxonomy for tools. Enables mode-based tool selection by required capabilities.';


--
-- Name: tool_capability_assignments; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tool_capability_assignments (
    tool_id uuid NOT NULL,
    capability_id uuid NOT NULL
);


ALTER TABLE public.tool_capability_assignments OWNER TO nexor;

--
-- Name: TABLE tool_capability_assignments; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.tool_capability_assignments IS 'Maps tools to the capabilities they provide. A tool can provide multiple capabilities.';


--
-- Name: tool_router_mode_tools; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tool_router_mode_tools (
    mode_id uuid NOT NULL,
    tool_id uuid NOT NULL
);


ALTER TABLE public.tool_router_mode_tools OWNER TO nexor;

--
-- Name: tool_router_modes; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tool_router_modes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    router_id uuid NOT NULL,
    mode_key text NOT NULL,
    display_name text NOT NULL,
    description text NOT NULL,
    system_prompt text NOT NULL,
    temperature real DEFAULT 0.7 NOT NULL,
    max_tokens integer DEFAULT 4096 NOT NULL,
    append_to_agent_system_prompt boolean DEFAULT false NOT NULL,
    append_to_agent_tools boolean DEFAULT true NOT NULL,
    display_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT tool_router_modes_max_tokens_check CHECK ((max_tokens > 0)),
    CONSTRAINT tool_router_modes_mode_key_check CHECK ((mode_key ~ '^[a-z][a-z0-9_]*$'::text)),
    CONSTRAINT tool_router_modes_temperature_check CHECK (((temperature >= (0.0)::double precision) AND (temperature <= (2.0)::double precision)))
);


ALTER TABLE public.tool_router_modes OWNER TO nexor;

--
-- Name: tool_router_tools; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tool_router_tools (
    router_id uuid NOT NULL,
    tool_id uuid NOT NULL
);


ALTER TABLE public.tool_router_tools OWNER TO nexor;

--
-- Name: tool_routers; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tool_routers (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    system_prompt text NOT NULL,
    model_id text NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    parent_router_id uuid,
    level integer DEFAULT 1 NOT NULL,
    CONSTRAINT tool_routers_level_check CHECK ((level = ANY (ARRAY[1, 2, 3])))
);


ALTER TABLE public.tool_routers OWNER TO nexor;

--
-- Name: tools; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tools (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    name text NOT NULL,
    display_name text NOT NULL,
    description text NOT NULL,
    parameters jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    version integer DEFAULT 1 NOT NULL
);


ALTER TABLE public.tools OWNER TO nexor;

--
-- Name: tools_versions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.tools_versions (
    id uuid NOT NULL,
    version integer NOT NULL,
    name text NOT NULL,
    display_name text NOT NULL,
    description text NOT NULL,
    parameters jsonb NOT NULL,
    changed_by uuid,
    changed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.tools_versions OWNER TO nexor;

--
-- Name: users; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    email text NOT NULL,
    password_hash text,
    github_id bigint,
    github_login text,
    github_token_encrypted text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.users OWNER TO nexor;

--
-- Name: workflow_collections; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflow_collections (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    execution_mode text DEFAULT 'parallel'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.workflow_collections OWNER TO nexor;

--
-- Name: workflow_executions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflow_executions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    collection_run_id uuid NOT NULL,
    workflow_id uuid NOT NULL,
    user_id uuid NOT NULL,
    status text NOT NULL,
    started_at timestamp with time zone,
    completed_at timestamp with time zone,
    outputs jsonb,
    error text
);


ALTER TABLE public.workflow_executions OWNER TO nexor;

--
-- Name: workflow_step_agents; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflow_step_agents (
    step_id uuid NOT NULL,
    agent_id uuid NOT NULL,
    execution_strategy text NOT NULL,
    agent_order integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.workflow_step_agents OWNER TO nexor;

--
-- Name: workflow_step_edges; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflow_step_edges (
    from_step_id uuid NOT NULL,
    to_step_id uuid NOT NULL,
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    from_output_port text,
    to_input_port text,
    transform_jsonpath text,
    condition_type text,
    condition_value jsonb,
    edge_label text,
    workflow_id uuid NOT NULL
);


ALTER TABLE public.workflow_step_edges OWNER TO nexor;

--
-- Name: TABLE workflow_step_edges; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.workflow_step_edges IS 'DAG edges connecting workflow steps. Now supports port-based connections with optional transformations.';


--
-- Name: COLUMN workflow_step_edges.from_output_port; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_step_edges.from_output_port IS 'Source step output port name. System automatically reads from envelope.data.<from_output_port>';


--
-- Name: COLUMN workflow_step_edges.to_input_port; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_step_edges.to_input_port IS 'Target step input port name. Mapped data becomes available as input.<to_input_port>';


--
-- Name: COLUMN workflow_step_edges.transform_jsonpath; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_step_edges.transform_jsonpath IS 'Optional JSONPath transformation applied to data flowing through edge (e.g., "$.items[*].name" to extract names)';


--
-- Name: workflow_steps; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflow_steps (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    workflow_id uuid NOT NULL,
    agent_id uuid NOT NULL,
    execution_mode text DEFAULT 'single'::text NOT NULL,
    for_each_ref text,
    prompt_template_id uuid,
    prompt_template text DEFAULT ''::text NOT NULL,
    output_schema_id uuid,
    output_variable_name text,
    interactive_agent_id uuid,
    display_order integer DEFAULT 0 NOT NULL,
    for_each_label_field text,
    version integer DEFAULT 1 NOT NULL,
    room_id uuid,
    agent_execution_mode text,
    position_x double precision,
    position_y double precision,
    width double precision DEFAULT 200,
    height double precision DEFAULT 100,
    routing_mode text,
    routing_field text,
    cavernous_config_document_id uuid
);


ALTER TABLE public.workflow_steps OWNER TO nexor;

--
-- Name: TABLE workflow_steps; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON TABLE public.workflow_steps IS 'Workflow DAG nodes. Note: output_variable_name column is deprecated - use step_outputs table for port definitions.';


--
-- Name: COLUMN workflow_steps.execution_mode; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_steps.execution_mode IS 'Execution strategy:
     - "single": Execute once with step agent (TIER 1 - Static)
     - "for_each": Iterate over array, with optional label routing (TIER 2 - Label-based)
     - "cavernous": Document-based dynamic routing with agent collaboration (TIER 3 - Cavernous)
     - "room": Multi-agent room discussion';


--
-- Name: COLUMN workflow_steps.position_x; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_steps.position_x IS 'X coordinate for visual canvas positioning (future UI)';


--
-- Name: COLUMN workflow_steps.position_y; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_steps.position_y IS 'Y coordinate for visual canvas positioning (future UI)';


--
-- Name: COLUMN workflow_steps.routing_mode; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_steps.routing_mode IS 'Routing strategy:
     - NULL: Use step agent_id directly (static agent)
     - "label": Route array items by label/category field to specialist agents (TIER 2)
     - "cavernous": Document-based dynamic routing with agent collaboration (TIER 3)';


--
-- Name: COLUMN workflow_steps.routing_field; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_steps.routing_field IS 'For routing_mode="label": which field in array items contains the routing label/category';


--
-- Name: COLUMN workflow_steps.cavernous_config_document_id; Type: COMMENT; Schema: public; Owner: nexor
--

COMMENT ON COLUMN public.workflow_steps.cavernous_config_document_id IS 'For routing_mode="cavernous": document containing routing configuration JSON';


--
-- Name: workflow_steps_versions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflow_steps_versions (
    id uuid NOT NULL,
    version integer NOT NULL,
    workflow_id uuid NOT NULL,
    agent_id uuid NOT NULL,
    execution_mode text NOT NULL,
    for_each_ref text,
    prompt_template_id uuid,
    prompt_template text NOT NULL,
    output_schema_id uuid,
    output_variable_name text,
    interactive_agent_id uuid,
    for_each_label_field text,
    display_order integer NOT NULL,
    changed_by uuid,
    changed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.workflow_steps_versions OWNER TO nexor;

--
-- Name: workflows; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflows (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    name text NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    version integer DEFAULT 1 NOT NULL,
    execution_mode text DEFAULT 'parallel'::text NOT NULL
);


ALTER TABLE public.workflows OWNER TO nexor;

--
-- Name: workflows_versions; Type: TABLE; Schema: public; Owner: nexor
--

CREATE TABLE public.workflows_versions (
    id uuid NOT NULL,
    version integer NOT NULL,
    name text NOT NULL,
    description text NOT NULL,
    changed_by uuid,
    changed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.workflows_versions OWNER TO nexor;

--
-- Name: agent_context agent_context_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_context
    ADD CONSTRAINT agent_context_pkey PRIMARY KEY (agent_id, document_id);


--
-- Name: agent_executions agent_executions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_pkey PRIMARY KEY (id);


--
-- Name: agent_modes agent_modes_agent_id_name_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_modes
    ADD CONSTRAINT agent_modes_agent_id_name_key UNIQUE (agent_id, name);


--
-- Name: agent_modes agent_modes_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_modes
    ADD CONSTRAINT agent_modes_pkey PRIMARY KEY (id);


--
-- Name: agent_modes_versions agent_modes_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_modes_versions
    ADD CONSTRAINT agent_modes_versions_pkey PRIMARY KEY (id, version);


--
-- Name: agent_tools agent_tools_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_tools
    ADD CONSTRAINT agent_tools_pkey PRIMARY KEY (agent_id, tool_id);


--
-- Name: agents agents_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_pkey PRIMARY KEY (id);


--
-- Name: agents_versions agents_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agents_versions
    ADD CONSTRAINT agents_versions_pkey PRIMARY KEY (id, version);


--
-- Name: auth_config auth_config_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.auth_config
    ADD CONSTRAINT auth_config_pkey PRIMARY KEY (id);


--
-- Name: chat_messages chat_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.chat_messages
    ADD CONSTRAINT chat_messages_pkey PRIMARY KEY (id);


--
-- Name: chat_sessions chat_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.chat_sessions
    ADD CONSTRAINT chat_sessions_pkey PRIMARY KEY (id);


--
-- Name: collection_runs collection_runs_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_runs
    ADD CONSTRAINT collection_runs_pkey PRIMARY KEY (id);


--
-- Name: collection_workflow_edges collection_workflow_edges_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_workflow_edges
    ADD CONSTRAINT collection_workflow_edges_pkey PRIMARY KEY (from_workflow_id, to_workflow_id, collection_id);


--
-- Name: collection_workflows collection_workflows_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_workflows
    ADD CONSTRAINT collection_workflows_pkey PRIMARY KEY (collection_id, workflow_id);


--
-- Name: context_store context_store_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.context_store
    ADD CONSTRAINT context_store_pkey PRIMARY KEY (id);


--
-- Name: documents documents_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_pkey PRIMARY KEY (id);


--
-- Name: execution_messages execution_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.execution_messages
    ADD CONSTRAINT execution_messages_pkey PRIMARY KEY (id);


--
-- Name: mode_required_capabilities mode_required_capabilities_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.mode_required_capabilities
    ADD CONSTRAINT mode_required_capabilities_pkey PRIMARY KEY (mode_id, capability_id);


--
-- Name: output_schemas output_schemas_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.output_schemas
    ADD CONSTRAINT output_schemas_pkey PRIMARY KEY (id);


--
-- Name: output_schemas output_schemas_user_id_name_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.output_schemas
    ADD CONSTRAINT output_schemas_user_id_name_key UNIQUE (user_id, name);


--
-- Name: output_schemas_versions output_schemas_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.output_schemas_versions
    ADD CONSTRAINT output_schemas_versions_pkey PRIMARY KEY (id, version);


--
-- Name: pr_merge_queue pr_merge_queue_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.pr_merge_queue
    ADD CONSTRAINT pr_merge_queue_pkey PRIMARY KEY (id);


--
-- Name: pr_merge_queue pr_merge_queue_repo_owner_repo_name_pr_number_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.pr_merge_queue
    ADD CONSTRAINT pr_merge_queue_repo_owner_repo_name_pr_number_key UNIQUE (repo_owner, repo_name, pr_number);


--
-- Name: prompt_templates prompt_templates_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.prompt_templates
    ADD CONSTRAINT prompt_templates_pkey PRIMARY KEY (id);


--
-- Name: prompt_templates prompt_templates_user_id_name_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.prompt_templates
    ADD CONSTRAINT prompt_templates_user_id_name_key UNIQUE (user_id, name);


--
-- Name: prompt_templates_versions prompt_templates_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.prompt_templates_versions
    ADD CONSTRAINT prompt_templates_versions_pkey PRIMARY KEY (id, version);


--
-- Name: results results_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.results
    ADD CONSTRAINT results_pkey PRIMARY KEY (id);


--
-- Name: room_execution_outputs room_execution_outputs_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_execution_outputs
    ADD CONSTRAINT room_execution_outputs_pkey PRIMARY KEY (id);


--
-- Name: room_execution_outputs room_execution_outputs_room_session_id_turn_number_output_n_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_execution_outputs
    ADD CONSTRAINT room_execution_outputs_room_session_id_turn_number_output_n_key UNIQUE (room_session_id, turn_number, output_name);


--
-- Name: room_members room_members_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_members
    ADD CONSTRAINT room_members_pkey PRIMARY KEY (room_id, agent_id);


--
-- Name: room_sessions room_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_sessions
    ADD CONSTRAINT room_sessions_pkey PRIMARY KEY (id);


--
-- Name: rooms rooms_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_pkey PRIMARY KEY (id);


--
-- Name: router_requests router_requests_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.router_requests
    ADD CONSTRAINT router_requests_pkey PRIMARY KEY (id);


--
-- Name: step_documents step_documents_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_documents
    ADD CONSTRAINT step_documents_pkey PRIMARY KEY (step_id, document_id);


--
-- Name: step_inputs step_inputs_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_inputs
    ADD CONSTRAINT step_inputs_pkey PRIMARY KEY (id);


--
-- Name: step_inputs step_inputs_workflow_step_id_port_name_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_inputs
    ADD CONSTRAINT step_inputs_workflow_step_id_port_name_key UNIQUE (workflow_step_id, port_name);


--
-- Name: step_outputs step_outputs_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_outputs
    ADD CONSTRAINT step_outputs_pkey PRIMARY KEY (id);


--
-- Name: step_outputs step_outputs_workflow_step_id_port_name_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_outputs
    ADD CONSTRAINT step_outputs_workflow_step_id_port_name_key UNIQUE (workflow_step_id, port_name);


--
-- Name: step_routing_rules step_routing_rules_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_routing_rules
    ADD CONSTRAINT step_routing_rules_pkey PRIMARY KEY (id);


--
-- Name: step_routing_rules step_routing_rules_workflow_step_id_label_value_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_routing_rules
    ADD CONSTRAINT step_routing_rules_workflow_step_id_label_value_key UNIQUE (workflow_step_id, label_value);


--
-- Name: system_config system_config_config_key_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.system_config
    ADD CONSTRAINT system_config_config_key_key UNIQUE (config_key);


--
-- Name: system_config system_config_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.system_config
    ADD CONSTRAINT system_config_pkey PRIMARY KEY (id);


--
-- Name: tasks tasks_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_pkey PRIMARY KEY (id);


--
-- Name: token_ledger token_ledger_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.token_ledger
    ADD CONSTRAINT token_ledger_pkey PRIMARY KEY (id);


--
-- Name: tool_capabilities tool_capabilities_capability_key_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_capabilities
    ADD CONSTRAINT tool_capabilities_capability_key_key UNIQUE (capability_key);


--
-- Name: tool_capabilities tool_capabilities_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_capabilities
    ADD CONSTRAINT tool_capabilities_pkey PRIMARY KEY (id);


--
-- Name: tool_capability_assignments tool_capability_assignments_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_capability_assignments
    ADD CONSTRAINT tool_capability_assignments_pkey PRIMARY KEY (tool_id, capability_id);


--
-- Name: tool_router_mode_tools tool_router_mode_tools_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_mode_tools
    ADD CONSTRAINT tool_router_mode_tools_pkey PRIMARY KEY (mode_id, tool_id);


--
-- Name: tool_router_modes tool_router_modes_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_modes
    ADD CONSTRAINT tool_router_modes_pkey PRIMARY KEY (id);


--
-- Name: tool_router_tools tool_router_tools_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_tools
    ADD CONSTRAINT tool_router_tools_pkey PRIMARY KEY (router_id, tool_id);


--
-- Name: tool_routers tool_routers_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_routers
    ADD CONSTRAINT tool_routers_pkey PRIMARY KEY (id);


--
-- Name: tools tools_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tools
    ADD CONSTRAINT tools_pkey PRIMARY KEY (id);


--
-- Name: tools tools_user_id_name_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tools
    ADD CONSTRAINT tools_user_id_name_key UNIQUE (user_id, name);


--
-- Name: tools_versions tools_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tools_versions
    ADD CONSTRAINT tools_versions_pkey PRIMARY KEY (id, version);


--
-- Name: tool_router_modes unique_mode_key_per_router; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_modes
    ADD CONSTRAINT unique_mode_key_per_router UNIQUE (router_id, mode_key);


--
-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: users users_github_id_key; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_github_id_key UNIQUE (github_id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: workflow_collections workflow_collections_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_collections
    ADD CONSTRAINT workflow_collections_pkey PRIMARY KEY (id);


--
-- Name: workflow_executions workflow_executions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_executions
    ADD CONSTRAINT workflow_executions_pkey PRIMARY KEY (id);


--
-- Name: workflow_step_agents workflow_step_agents_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_agents
    ADD CONSTRAINT workflow_step_agents_pkey PRIMARY KEY (step_id, agent_id);


--
-- Name: workflow_step_edges workflow_step_edges_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_pkey PRIMARY KEY (id);


--
-- Name: workflow_step_edges workflow_step_edges_workflow_from_to_unique; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_workflow_from_to_unique UNIQUE (workflow_id, from_step_id, to_step_id);


--
-- Name: workflow_steps workflow_steps_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_pkey PRIMARY KEY (id);


--
-- Name: workflow_steps_versions workflow_steps_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps_versions
    ADD CONSTRAINT workflow_steps_versions_pkey PRIMARY KEY (id, version);


--
-- Name: workflows workflows_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflows
    ADD CONSTRAINT workflows_pkey PRIMARY KEY (id);


--
-- Name: workflows_versions workflows_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflows_versions
    ADD CONSTRAINT workflows_versions_pkey PRIMARY KEY (id, version);


--
-- Name: idx_agent_context_agent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_context_agent ON public.agent_context USING btree (agent_id);


--
-- Name: idx_agent_executions_agent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_agent ON public.agent_executions USING btree (agent_id);


--
-- Name: idx_agent_executions_parent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_parent ON public.agent_executions USING btree (parent_agent_execution_id);


--
-- Name: idx_agent_executions_room; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_room ON public.agent_executions USING btree (room_session_id) WHERE (room_session_id IS NOT NULL);


--
-- Name: idx_agent_executions_router_mode; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_router_mode ON public.agent_executions USING btree (selected_router_mode_id);


--
-- Name: idx_agent_executions_routing_analysis; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_routing_analysis ON public.agent_executions USING gin (routing_analysis) WHERE (routing_analysis IS NOT NULL);


--
-- Name: idx_agent_executions_routing_doc; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_routing_doc ON public.agent_executions USING btree (selected_routing_document_id) WHERE (selected_routing_document_id IS NOT NULL);


--
-- Name: idx_agent_executions_started; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_started ON public.agent_executions USING btree (started_at DESC);


--
-- Name: idx_agent_executions_status; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_status ON public.agent_executions USING btree (status);


--
-- Name: idx_agent_executions_step; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_step ON public.agent_executions USING btree (workflow_step_id);


--
-- Name: idx_agent_executions_workflow_execution_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_executions_workflow_execution_id ON public.agent_executions USING btree (workflow_execution_id);


--
-- Name: idx_agent_tools_tool; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agent_tools_tool ON public.agent_tools USING btree (tool_id);


--
-- Name: idx_agents_output_schema; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agents_output_schema ON public.agents USING btree (output_schema_id);


--
-- Name: idx_agents_router; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agents_router ON public.agents USING btree (router_id);


--
-- Name: idx_agents_status; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agents_status ON public.agents USING btree (status);


--
-- Name: idx_agents_user_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_agents_user_id ON public.agents USING btree (user_id);


--
-- Name: idx_chat_messages_session; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_chat_messages_session ON public.chat_messages USING btree (session_id, "timestamp");


--
-- Name: idx_chat_messages_timestamp; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_chat_messages_timestamp ON public.chat_messages USING btree ("timestamp");


--
-- Name: idx_chat_messages_user_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_chat_messages_user_id ON public.chat_messages USING btree (user_id);


--
-- Name: idx_chat_sessions_has_draft_config; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_chat_sessions_has_draft_config ON public.chat_sessions USING btree (((draft_config IS NOT NULL)));


--
-- Name: idx_chat_sessions_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_chat_sessions_user ON public.chat_sessions USING btree (user_id, updated_at DESC);


--
-- Name: idx_collection_runs_collection_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_runs_collection_id ON public.collection_runs USING btree (collection_id);


--
-- Name: idx_collection_runs_status; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_runs_status ON public.collection_runs USING btree (status);


--
-- Name: idx_collection_runs_user_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_runs_user_id ON public.collection_runs USING btree (user_id);


--
-- Name: idx_collection_workflow_edges_collection_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_workflow_edges_collection_id ON public.collection_workflow_edges USING btree (collection_id);


--
-- Name: idx_collection_workflow_edges_from_workflow_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_workflow_edges_from_workflow_id ON public.collection_workflow_edges USING btree (from_workflow_id);


--
-- Name: idx_collection_workflow_edges_to_workflow_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_workflow_edges_to_workflow_id ON public.collection_workflow_edges USING btree (to_workflow_id);


--
-- Name: idx_collection_workflows_collection_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_workflows_collection_id ON public.collection_workflows USING btree (collection_id);


--
-- Name: idx_collection_workflows_workflow_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_collection_workflows_workflow_id ON public.collection_workflows USING btree (workflow_id);


--
-- Name: idx_context_store_priority; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_context_store_priority ON public.context_store USING btree (session_id, priority DESC);


--
-- Name: idx_context_store_session; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_context_store_session ON public.context_store USING btree (session_id, status);


--
-- Name: idx_documents_ref_tag; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_documents_ref_tag ON public.documents USING btree (ref_tag);


--
-- Name: idx_documents_search; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_documents_search ON public.documents USING gin (to_tsvector('english'::regconfig, ((title || ' '::text) || content)));


--
-- Name: idx_documents_session; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_documents_session ON public.documents USING btree (session_id);


--
-- Name: idx_documents_tags; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_documents_tags ON public.documents USING gin (tags);


--
-- Name: idx_documents_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_documents_user ON public.documents USING btree (user_id);


--
-- Name: idx_execution_messages_created; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_execution_messages_created ON public.execution_messages USING btree (created_at);


--
-- Name: idx_execution_messages_execution; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_execution_messages_execution ON public.execution_messages USING btree (agent_execution_id);


--
-- Name: idx_execution_messages_role; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_execution_messages_role ON public.execution_messages USING btree (agent_execution_id, role);


--
-- Name: idx_mode_required_capabilities_mode; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_mode_required_capabilities_mode ON public.mode_required_capabilities USING btree (mode_id);


--
-- Name: idx_output_schemas_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_output_schemas_user ON public.output_schemas USING btree (user_id);


--
-- Name: idx_pr_merge_queue_user_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_pr_merge_queue_user_id ON public.pr_merge_queue USING btree (user_id);


--
-- Name: idx_pr_queue_position; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_pr_queue_position ON public.pr_merge_queue USING btree (repo_owner, repo_name, queue_position);


--
-- Name: idx_pr_queue_status; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_pr_queue_status ON public.pr_merge_queue USING btree (status);


--
-- Name: idx_prompt_templates_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_prompt_templates_user ON public.prompt_templates USING btree (user_id);


--
-- Name: idx_results_execution; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_results_execution ON public.results USING btree (agent_execution_id);


--
-- Name: idx_results_schema; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_results_schema ON public.results USING btree (output_schema_id);


--
-- Name: idx_results_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_results_user ON public.results USING btree (user_id);


--
-- Name: idx_room_members_agent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_members_agent ON public.room_members USING btree (agent_id);


--
-- Name: idx_room_members_input_schema; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_members_input_schema ON public.room_members USING btree (input_schema_id) WHERE (input_schema_id IS NOT NULL);


--
-- Name: idx_room_members_output_schema; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_members_output_schema ON public.room_members USING btree (output_schema_id) WHERE (output_schema_id IS NOT NULL);


--
-- Name: idx_room_outputs_agent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_outputs_agent ON public.room_execution_outputs USING btree (agent_id);


--
-- Name: idx_room_outputs_schema; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_outputs_schema ON public.room_execution_outputs USING btree (schema_id) WHERE (schema_id IS NOT NULL);


--
-- Name: idx_room_outputs_session; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_outputs_session ON public.room_execution_outputs USING btree (room_session_id, turn_number);


--
-- Name: idx_room_sessions_outputs; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_sessions_outputs ON public.room_sessions USING gin (structured_outputs) WHERE (structured_outputs IS NOT NULL);


--
-- Name: idx_room_sessions_room; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_sessions_room ON public.room_sessions USING btree (room_id);


--
-- Name: idx_room_sessions_status; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_room_sessions_status ON public.room_sessions USING btree (status);


--
-- Name: idx_rooms_output_schema; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_rooms_output_schema ON public.rooms USING btree (default_output_schema_id) WHERE (default_output_schema_id IS NOT NULL);


--
-- Name: idx_rooms_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_rooms_user ON public.rooms USING btree (user_id);


--
-- Name: idx_router_requests_session; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_router_requests_session ON public.router_requests USING btree (session_id, status);


--
-- Name: idx_step_documents_step; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_step_documents_step ON public.step_documents USING btree (step_id);


--
-- Name: idx_step_inputs_step; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_step_inputs_step ON public.step_inputs USING btree (workflow_step_id);


--
-- Name: idx_step_outputs_step; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_step_outputs_step ON public.step_outputs USING btree (workflow_step_id);


--
-- Name: idx_step_routing_rules_agent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_step_routing_rules_agent ON public.step_routing_rules USING btree (agent_id);


--
-- Name: idx_step_routing_rules_step; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_step_routing_rules_step ON public.step_routing_rules USING btree (workflow_step_id);


--
-- Name: idx_system_config_key; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_system_config_key ON public.system_config USING btree (config_key);


--
-- Name: idx_system_config_type; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_system_config_type ON public.system_config USING btree (config_type);


--
-- Name: idx_tasks_assigned_agent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tasks_assigned_agent ON public.tasks USING btree (assigned_agent);


--
-- Name: idx_tasks_slice_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tasks_slice_id ON public.tasks USING btree (slice_id);


--
-- Name: idx_tasks_status; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tasks_status ON public.tasks USING btree (status);


--
-- Name: idx_tasks_user_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tasks_user_id ON public.tasks USING btree (user_id);


--
-- Name: idx_token_ledger_agent_exec; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_token_ledger_agent_exec ON public.token_ledger USING btree (agent_execution_id);


--
-- Name: idx_token_ledger_created; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_token_ledger_created ON public.token_ledger USING btree (created_at DESC);


--
-- Name: idx_token_ledger_model; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_token_ledger_model ON public.token_ledger USING btree (model_id);


--
-- Name: idx_token_ledger_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_token_ledger_user ON public.token_ledger USING btree (user_id);


--
-- Name: idx_token_ledger_user_created; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_token_ledger_user_created ON public.token_ledger USING btree (user_id, created_at DESC);


--
-- Name: idx_tool_capabilities_category; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_capabilities_category ON public.tool_capabilities USING btree (category);


--
-- Name: idx_tool_capabilities_safety; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_capabilities_safety ON public.tool_capabilities USING btree (safety_level);


--
-- Name: idx_tool_capability_assignments_capability; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_capability_assignments_capability ON public.tool_capability_assignments USING btree (capability_id);


--
-- Name: idx_tool_router_mode_tools_tool; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_router_mode_tools_tool ON public.tool_router_mode_tools USING btree (tool_id);


--
-- Name: idx_tool_router_modes_order; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_router_modes_order ON public.tool_router_modes USING btree (router_id, display_order);


--
-- Name: idx_tool_router_modes_router; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_router_modes_router ON public.tool_router_modes USING btree (router_id);


--
-- Name: idx_tool_router_tools_tool; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_router_tools_tool ON public.tool_router_tools USING btree (tool_id);


--
-- Name: idx_tool_routers_level; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_routers_level ON public.tool_routers USING btree (level);


--
-- Name: idx_tool_routers_parent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_routers_parent ON public.tool_routers USING btree (parent_router_id);


--
-- Name: idx_tool_routers_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tool_routers_user ON public.tool_routers USING btree (user_id);


--
-- Name: idx_tools_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_tools_user ON public.tools USING btree (user_id);


--
-- Name: idx_users_email; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_users_email ON public.users USING btree (email);


--
-- Name: idx_users_github_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_users_github_id ON public.users USING btree (github_id) WHERE (github_id IS NOT NULL);


--
-- Name: idx_workflow_collections_user_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_collections_user_id ON public.workflow_collections USING btree (user_id);


--
-- Name: idx_workflow_executions_collection_run_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_executions_collection_run_id ON public.workflow_executions USING btree (collection_run_id);


--
-- Name: idx_workflow_executions_status; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_executions_status ON public.workflow_executions USING btree (status);


--
-- Name: idx_workflow_executions_user_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_executions_user_id ON public.workflow_executions USING btree (user_id);


--
-- Name: idx_workflow_executions_workflow_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_executions_workflow_id ON public.workflow_executions USING btree (workflow_id);


--
-- Name: idx_workflow_step_agents_agent_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_step_agents_agent_id ON public.workflow_step_agents USING btree (agent_id);


--
-- Name: idx_workflow_step_agents_step_id; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_step_agents_step_id ON public.workflow_step_agents USING btree (step_id);


--
-- Name: idx_workflow_step_edges_from; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_step_edges_from ON public.workflow_step_edges USING btree (from_step_id);


--
-- Name: idx_workflow_step_edges_ports; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_step_edges_ports ON public.workflow_step_edges USING btree (from_output_port, to_input_port);


--
-- Name: idx_workflow_step_edges_to; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_step_edges_to ON public.workflow_step_edges USING btree (to_step_id);


--
-- Name: idx_workflow_step_edges_workflow; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_step_edges_workflow ON public.workflow_step_edges USING btree (workflow_id);


--
-- Name: idx_workflow_steps_agent; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_steps_agent ON public.workflow_steps USING btree (agent_id);


--
-- Name: idx_workflow_steps_routing; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_steps_routing ON public.workflow_steps USING btree (routing_mode) WHERE (routing_mode IS NOT NULL);


--
-- Name: idx_workflow_steps_workflow; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflow_steps_workflow ON public.workflow_steps USING btree (workflow_id);


--
-- Name: idx_workflows_user; Type: INDEX; Schema: public; Owner: nexor
--

CREATE INDEX idx_workflows_user ON public.workflows USING btree (user_id);


--
-- Name: agent_context agent_context_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_context
    ADD CONSTRAINT agent_context_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: agent_context agent_context_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_context
    ADD CONSTRAINT agent_context_document_id_fkey FOREIGN KEY (document_id) REFERENCES public.documents(id) ON DELETE CASCADE;


--
-- Name: agent_executions agent_executions_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id);


--
-- Name: agent_executions agent_executions_parent_agent_execution_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_parent_agent_execution_id_fkey FOREIGN KEY (parent_agent_execution_id) REFERENCES public.agent_executions(id);


--
-- Name: agent_executions agent_executions_room_session_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_room_session_id_fkey FOREIGN KEY (room_session_id) REFERENCES public.room_sessions(id);


--
-- Name: agent_executions agent_executions_selected_mode_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_selected_mode_id_fkey FOREIGN KEY (selected_mode_id) REFERENCES public.agent_modes(id);


--
-- Name: agent_executions agent_executions_selected_router_mode_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_selected_router_mode_id_fkey FOREIGN KEY (selected_router_mode_id) REFERENCES public.tool_router_modes(id) ON DELETE SET NULL;


--
-- Name: agent_executions agent_executions_selected_routing_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_selected_routing_document_id_fkey FOREIGN KEY (selected_routing_document_id) REFERENCES public.documents(id);


--
-- Name: agent_executions agent_executions_workflow_execution_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_workflow_execution_id_fkey FOREIGN KEY (workflow_execution_id) REFERENCES public.workflow_executions(id) ON DELETE CASCADE;


--
-- Name: agent_executions agent_executions_workflow_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_executions
    ADD CONSTRAINT agent_executions_workflow_step_id_fkey FOREIGN KEY (workflow_step_id) REFERENCES public.workflow_steps(id);


--
-- Name: agent_modes agent_modes_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_modes
    ADD CONSTRAINT agent_modes_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: agent_modes_versions agent_modes_versions_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_modes_versions
    ADD CONSTRAINT agent_modes_versions_id_fkey FOREIGN KEY (id) REFERENCES public.agent_modes(id) ON DELETE CASCADE;


--
-- Name: agent_tools agent_tools_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_tools
    ADD CONSTRAINT agent_tools_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: agent_tools agent_tools_tool_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agent_tools
    ADD CONSTRAINT agent_tools_tool_id_fkey FOREIGN KEY (tool_id) REFERENCES public.tools(id) ON DELETE CASCADE;


--
-- Name: agents agents_current_task_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_current_task_fkey FOREIGN KEY (current_task) REFERENCES public.tasks(id);


--
-- Name: agents agents_output_schema_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_output_schema_id_fkey FOREIGN KEY (output_schema_id) REFERENCES public.output_schemas(id) ON DELETE SET NULL;


--
-- Name: agents agents_router_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_router_id_fkey FOREIGN KEY (router_id) REFERENCES public.tool_routers(id) ON DELETE SET NULL;


--
-- Name: agents agents_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: agents_versions agents_versions_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.agents_versions
    ADD CONSTRAINT agents_versions_id_fkey FOREIGN KEY (id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: chat_messages chat_messages_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.chat_messages
    ADD CONSTRAINT chat_messages_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: chat_sessions chat_sessions_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.chat_sessions
    ADD CONSTRAINT chat_sessions_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id);


--
-- Name: chat_sessions chat_sessions_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.chat_sessions
    ADD CONSTRAINT chat_sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: collection_runs collection_runs_collection_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_runs
    ADD CONSTRAINT collection_runs_collection_id_fkey FOREIGN KEY (collection_id) REFERENCES public.workflow_collections(id) ON DELETE CASCADE;


--
-- Name: collection_runs collection_runs_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_runs
    ADD CONSTRAINT collection_runs_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: collection_workflow_edges collection_workflow_edges_collection_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_workflow_edges
    ADD CONSTRAINT collection_workflow_edges_collection_id_fkey FOREIGN KEY (collection_id) REFERENCES public.workflow_collections(id) ON DELETE CASCADE;


--
-- Name: collection_workflow_edges collection_workflow_edges_from_workflow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_workflow_edges
    ADD CONSTRAINT collection_workflow_edges_from_workflow_id_fkey FOREIGN KEY (from_workflow_id) REFERENCES public.workflows(id) ON DELETE CASCADE;


--
-- Name: collection_workflow_edges collection_workflow_edges_to_workflow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_workflow_edges
    ADD CONSTRAINT collection_workflow_edges_to_workflow_id_fkey FOREIGN KEY (to_workflow_id) REFERENCES public.workflows(id) ON DELETE CASCADE;


--
-- Name: collection_workflows collection_workflows_collection_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_workflows
    ADD CONSTRAINT collection_workflows_collection_id_fkey FOREIGN KEY (collection_id) REFERENCES public.workflow_collections(id) ON DELETE CASCADE;


--
-- Name: collection_workflows collection_workflows_workflow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.collection_workflows
    ADD CONSTRAINT collection_workflows_workflow_id_fkey FOREIGN KEY (workflow_id) REFERENCES public.workflows(id) ON DELETE CASCADE;


--
-- Name: context_store context_store_session_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.context_store
    ADD CONSTRAINT context_store_session_id_fkey FOREIGN KEY (session_id) REFERENCES public.chat_sessions(id) ON DELETE CASCADE;


--
-- Name: documents documents_session_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_session_id_fkey FOREIGN KEY (session_id) REFERENCES public.chat_sessions(id);


--
-- Name: documents documents_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: execution_messages execution_messages_agent_execution_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.execution_messages
    ADD CONSTRAINT execution_messages_agent_execution_id_fkey FOREIGN KEY (agent_execution_id) REFERENCES public.agent_executions(id) ON DELETE CASCADE;


--
-- Name: mode_required_capabilities mode_required_capabilities_capability_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.mode_required_capabilities
    ADD CONSTRAINT mode_required_capabilities_capability_id_fkey FOREIGN KEY (capability_id) REFERENCES public.tool_capabilities(id) ON DELETE CASCADE;


--
-- Name: mode_required_capabilities mode_required_capabilities_mode_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.mode_required_capabilities
    ADD CONSTRAINT mode_required_capabilities_mode_id_fkey FOREIGN KEY (mode_id) REFERENCES public.tool_router_modes(id) ON DELETE CASCADE;


--
-- Name: output_schemas output_schemas_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.output_schemas
    ADD CONSTRAINT output_schemas_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: output_schemas_versions output_schemas_versions_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.output_schemas_versions
    ADD CONSTRAINT output_schemas_versions_id_fkey FOREIGN KEY (id) REFERENCES public.output_schemas(id) ON DELETE CASCADE;


--
-- Name: pr_merge_queue pr_merge_queue_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.pr_merge_queue
    ADD CONSTRAINT pr_merge_queue_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: prompt_templates prompt_templates_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.prompt_templates
    ADD CONSTRAINT prompt_templates_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: prompt_templates_versions prompt_templates_versions_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.prompt_templates_versions
    ADD CONSTRAINT prompt_templates_versions_id_fkey FOREIGN KEY (id) REFERENCES public.prompt_templates(id) ON DELETE CASCADE;


--
-- Name: results results_agent_execution_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.results
    ADD CONSTRAINT results_agent_execution_id_fkey FOREIGN KEY (agent_execution_id) REFERENCES public.agent_executions(id);


--
-- Name: results results_output_schema_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.results
    ADD CONSTRAINT results_output_schema_id_fkey FOREIGN KEY (output_schema_id) REFERENCES public.output_schemas(id);


--
-- Name: results results_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.results
    ADD CONSTRAINT results_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: room_execution_outputs room_execution_outputs_agent_execution_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_execution_outputs
    ADD CONSTRAINT room_execution_outputs_agent_execution_id_fkey FOREIGN KEY (agent_execution_id) REFERENCES public.agent_executions(id) ON DELETE CASCADE;


--
-- Name: room_execution_outputs room_execution_outputs_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_execution_outputs
    ADD CONSTRAINT room_execution_outputs_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id);


--
-- Name: room_execution_outputs room_execution_outputs_room_session_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_execution_outputs
    ADD CONSTRAINT room_execution_outputs_room_session_id_fkey FOREIGN KEY (room_session_id) REFERENCES public.room_sessions(id) ON DELETE CASCADE;


--
-- Name: room_execution_outputs room_execution_outputs_schema_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_execution_outputs
    ADD CONSTRAINT room_execution_outputs_schema_id_fkey FOREIGN KEY (schema_id) REFERENCES public.output_schemas(id);


--
-- Name: room_members room_members_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_members
    ADD CONSTRAINT room_members_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: room_members room_members_input_schema_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_members
    ADD CONSTRAINT room_members_input_schema_id_fkey FOREIGN KEY (input_schema_id) REFERENCES public.output_schemas(id);


--
-- Name: room_members room_members_output_schema_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_members
    ADD CONSTRAINT room_members_output_schema_id_fkey FOREIGN KEY (output_schema_id) REFERENCES public.output_schemas(id);


--
-- Name: room_members room_members_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_members
    ADD CONSTRAINT room_members_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id) ON DELETE CASCADE;


--
-- Name: room_sessions room_sessions_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.room_sessions
    ADD CONSTRAINT room_sessions_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id) ON DELETE CASCADE;


--
-- Name: rooms rooms_collection_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_collection_id_fkey FOREIGN KEY (collection_id) REFERENCES public.workflow_collections(id) ON DELETE SET NULL;


--
-- Name: rooms rooms_default_output_schema_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_default_output_schema_id_fkey FOREIGN KEY (default_output_schema_id) REFERENCES public.output_schemas(id);


--
-- Name: rooms rooms_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: router_requests router_requests_agent_execution_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.router_requests
    ADD CONSTRAINT router_requests_agent_execution_id_fkey FOREIGN KEY (agent_execution_id) REFERENCES public.agent_executions(id);


--
-- Name: router_requests router_requests_session_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.router_requests
    ADD CONSTRAINT router_requests_session_id_fkey FOREIGN KEY (session_id) REFERENCES public.chat_sessions(id) ON DELETE CASCADE;


--
-- Name: step_documents step_documents_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_documents
    ADD CONSTRAINT step_documents_document_id_fkey FOREIGN KEY (document_id) REFERENCES public.documents(id) ON DELETE CASCADE;


--
-- Name: step_documents step_documents_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_documents
    ADD CONSTRAINT step_documents_step_id_fkey FOREIGN KEY (step_id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: step_inputs step_inputs_workflow_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_inputs
    ADD CONSTRAINT step_inputs_workflow_step_id_fkey FOREIGN KEY (workflow_step_id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: step_outputs step_outputs_workflow_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_outputs
    ADD CONSTRAINT step_outputs_workflow_step_id_fkey FOREIGN KEY (workflow_step_id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: step_routing_rules step_routing_rules_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_routing_rules
    ADD CONSTRAINT step_routing_rules_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: step_routing_rules step_routing_rules_workflow_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.step_routing_rules
    ADD CONSTRAINT step_routing_rules_workflow_step_id_fkey FOREIGN KEY (workflow_step_id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: system_config system_config_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.system_config
    ADD CONSTRAINT system_config_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id);


--
-- Name: tasks tasks_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: token_ledger token_ledger_agent_execution_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.token_ledger
    ADD CONSTRAINT token_ledger_agent_execution_id_fkey FOREIGN KEY (agent_execution_id) REFERENCES public.agent_executions(id);


--
-- Name: token_ledger token_ledger_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.token_ledger
    ADD CONSTRAINT token_ledger_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: tool_capability_assignments tool_capability_assignments_capability_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_capability_assignments
    ADD CONSTRAINT tool_capability_assignments_capability_id_fkey FOREIGN KEY (capability_id) REFERENCES public.tool_capabilities(id) ON DELETE CASCADE;


--
-- Name: tool_capability_assignments tool_capability_assignments_tool_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_capability_assignments
    ADD CONSTRAINT tool_capability_assignments_tool_id_fkey FOREIGN KEY (tool_id) REFERENCES public.tools(id) ON DELETE CASCADE;


--
-- Name: tool_router_mode_tools tool_router_mode_tools_mode_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_mode_tools
    ADD CONSTRAINT tool_router_mode_tools_mode_id_fkey FOREIGN KEY (mode_id) REFERENCES public.tool_router_modes(id) ON DELETE CASCADE;


--
-- Name: tool_router_mode_tools tool_router_mode_tools_tool_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_mode_tools
    ADD CONSTRAINT tool_router_mode_tools_tool_id_fkey FOREIGN KEY (tool_id) REFERENCES public.tools(id) ON DELETE CASCADE;


--
-- Name: tool_router_modes tool_router_modes_router_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_modes
    ADD CONSTRAINT tool_router_modes_router_id_fkey FOREIGN KEY (router_id) REFERENCES public.tool_routers(id) ON DELETE CASCADE;


--
-- Name: tool_router_tools tool_router_tools_router_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_tools
    ADD CONSTRAINT tool_router_tools_router_id_fkey FOREIGN KEY (router_id) REFERENCES public.tool_routers(id) ON DELETE CASCADE;


--
-- Name: tool_router_tools tool_router_tools_tool_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_router_tools
    ADD CONSTRAINT tool_router_tools_tool_id_fkey FOREIGN KEY (tool_id) REFERENCES public.tools(id) ON DELETE CASCADE;


--
-- Name: tool_routers tool_routers_parent_router_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_routers
    ADD CONSTRAINT tool_routers_parent_router_id_fkey FOREIGN KEY (parent_router_id) REFERENCES public.tool_routers(id) ON DELETE CASCADE;


--
-- Name: tool_routers tool_routers_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tool_routers
    ADD CONSTRAINT tool_routers_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: tools tools_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tools
    ADD CONSTRAINT tools_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: tools_versions tools_versions_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.tools_versions
    ADD CONSTRAINT tools_versions_id_fkey FOREIGN KEY (id) REFERENCES public.tools(id) ON DELETE CASCADE;


--
-- Name: workflow_collections workflow_collections_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_collections
    ADD CONSTRAINT workflow_collections_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: workflow_executions workflow_executions_collection_run_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_executions
    ADD CONSTRAINT workflow_executions_collection_run_id_fkey FOREIGN KEY (collection_run_id) REFERENCES public.collection_runs(id) ON DELETE CASCADE;


--
-- Name: workflow_executions workflow_executions_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_executions
    ADD CONSTRAINT workflow_executions_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: workflow_executions workflow_executions_workflow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_executions
    ADD CONSTRAINT workflow_executions_workflow_id_fkey FOREIGN KEY (workflow_id) REFERENCES public.workflows(id) ON DELETE CASCADE;


--
-- Name: workflow_step_agents workflow_step_agents_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_agents
    ADD CONSTRAINT workflow_step_agents_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: workflow_step_agents workflow_step_agents_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_agents
    ADD CONSTRAINT workflow_step_agents_step_id_fkey FOREIGN KEY (step_id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: workflow_step_edges workflow_step_edges_from_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_from_step_id_fkey FOREIGN KEY (from_step_id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: workflow_step_edges workflow_step_edges_to_step_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_to_step_id_fkey FOREIGN KEY (to_step_id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: workflow_step_edges workflow_step_edges_workflow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_workflow_id_fkey FOREIGN KEY (workflow_id) REFERENCES public.workflows(id) ON DELETE CASCADE;


--
-- Name: workflow_steps workflow_steps_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id);


--
-- Name: workflow_steps workflow_steps_cavernous_config_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_cavernous_config_document_id_fkey FOREIGN KEY (cavernous_config_document_id) REFERENCES public.documents(id);


--
-- Name: workflow_steps workflow_steps_interactive_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_interactive_agent_id_fkey FOREIGN KEY (interactive_agent_id) REFERENCES public.agents(id);


--
-- Name: workflow_steps workflow_steps_output_schema_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_output_schema_id_fkey FOREIGN KEY (output_schema_id) REFERENCES public.output_schemas(id);


--
-- Name: workflow_steps workflow_steps_prompt_template_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_prompt_template_id_fkey FOREIGN KEY (prompt_template_id) REFERENCES public.prompt_templates(id);


--
-- Name: workflow_steps workflow_steps_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id);


--
-- Name: workflow_steps_versions workflow_steps_versions_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps_versions
    ADD CONSTRAINT workflow_steps_versions_id_fkey FOREIGN KEY (id) REFERENCES public.workflow_steps(id) ON DELETE CASCADE;


--
-- Name: workflow_steps workflow_steps_workflow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_workflow_id_fkey FOREIGN KEY (workflow_id) REFERENCES public.workflows(id) ON DELETE CASCADE;


--
-- Name: workflows workflows_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflows
    ADD CONSTRAINT workflows_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: workflows_versions workflows_versions_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: nexor
--

ALTER TABLE ONLY public.workflows_versions
    ADD CONSTRAINT workflows_versions_id_fkey FOREIGN KEY (id) REFERENCES public.workflows(id) ON DELETE CASCADE;


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: nexor
--

REVOKE USAGE ON SCHEMA public FROM PUBLIC;


--
-- PostgreSQL database dump complete
--


