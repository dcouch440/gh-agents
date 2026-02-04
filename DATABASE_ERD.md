# DATABASE ENTITY RELATIONSHIP DIAGRAM

## FULL SYSTEM ERD

```
┌─────────────────┐
│      USERS      │  (ROOT - MULTI-TENANT ANCHOR)
│─────────────────│
│ ID (PK)         │
│ EMAIL (UNIQUE)  │
│ PASSWORD_HASH   │
│ GITHUB_ID (UQ)  │
│ GITHUB_LOGIN    │
│ GITHUB_TOKEN_ENC│
│ CREATED_AT      │
│ UPDATED_AT      │
└────────┬────────┘
         │
         │ (ALL TABLES REFERENCE USER_ID FOR TENANT ISOLATION)
         │
         ├──────────────────────────────────────────────────────────────────────────────┐
         │                                                                              │
         │  CORE ENTITIES                                                               │
         │                                                                              │
┌────────▼────────┐         ┌───────────────┐         ┌──────────────────┐              │
│     AGENTS      │◄────────┤  AGENT_TOOLS  ├────────►│      TOOLS       │              │
│─────────────────│         │  (JOIN TABLE) │         │──────────────────│              │
│ ID (PK)         │         │───────────────│         │ ID (PK)          │              │
│ USER_ID (FK)    │         │ AGENT_ID (FK) │         │ USER_ID (FK)     │              │
│ NAME            │         │ TOOL_ID (FK)  │         │ NAME (UNIQUE/USR)│              │
│ SYSTEM_PROMPT   │         └───────────────┘         │ DISPLAY_NAME     │              │
│ PERSONA_STYLE   │                                   │ DESCRIPTION      │              │
│ MODEL_PROVIDER  │         ┌───────────────┐         │ PARAMETERS (JSON)│              │
│ MODEL_ID        │◄────────┤ AGENT_CONTEXT ├──┐      │ VERSION          │              │
│ MODEL_MAX_TOKENS│         │  (JOIN TABLE) │  │      └────────┬─────────┘              │
│ MODEL_TEMP      │         │───────────────│  │               │                        │
│ CURRENT_TASK(FK)├──┐      │ AGENT_ID (FK) │  │               │                        │
│ STATUS          │  │      │ DOCUMENT_ID   │  │      ┌────────▼─────────┐              │
│ ROUTER_MODE     │  │      └───────────────┘  │      │  TOOL_ROUTERS    │              │
│ OUTPUT_SCHEMA_ID│  │                         │      │──────────────────│              │
│ VERSION         │  │                         │      │ ID (PK)          │              │
└────────┬────────┘  │                         │      │ USER_ID (FK)     │              │
         │           │                         │      │ NAME             │              │
         │           │                         │      │ DESCRIPTION      │              │
         │ 1:N       │                         │      │ SYSTEM_PROMPT    │              │
         │           │                         │      │ MODEL_ID         │              │
┌────────▼────────┐  │                         │      │ IS_ACTIVE        │              │
│  AGENT_MODES    │  │                         │      └────────┬─────────┘              │
│─────────────────│  │                         │               │                        │
│ ID (PK)         │  │                         │               │ 1:N                    │
│ AGENT_ID (FK)   │  │                         │               │                        │
│ NAME (UQ/AGENT) │  │                         │      ┌────────▼─────────┐              │
│ SYSTEM_PROMPT_  │  │                         │      │TOOL_ROUTER_TOOLS │              │
│   SUFFIX        │  │                         │      │  (JOIN TABLE)    │              │
│ TEMP_OVERRIDE   │  │                         │      │──────────────────│              │
│ MODEL_OVERRIDE  │  │                         │      │ ROUTER_ID (FK)   │              │
│ TOOL_OVERRIDES[]│  │                         │      │ TOOL_ID (FK)     │              │
│ CLASSIFIER_HINT │  │                         │      └──────────────────┘              │
│ VERSION         │  │                         │                                        │
└─────────────────┘  │                         │                                        │
                     │                         │                                        │
         ┌───────────┘                         │                                        │
         │                                     │                                        │
         │                              ┌──────▼──────────┐                             │
         │                              │   DOCUMENTS     │                             │
         │                              │─────────────────│                             │
         │                              │ ID (PK)         │                             │
         │                              │ USER_ID (FK)    │                             │
         │                              │ SESSION_ID (FK) │                             │
         │                              │ TITLE           │                             │
         │                              │ CONTENT         │                             │
         │                              │ SUMMARY         │                             │
         │                              │ DOC_TYPE        │                             │
         │                              │ REF_TAG         │                             │
         │                              │ TAGS[]          │                             │
         │                              └─────────────────┘                             │
         │                                                                              │
    ┌────▼──────┐                                                                       │
    │  TASKS    │  (LEGACY)           ┌──────────────────┐                              │
    │───────────│                     │    CLUSTERS      │◄─────────────────────────────┤
    │ ID (PK)   │                     │──────────────────│                              │
    │ USER_ID   │                     │ ID (PK)          │                              │
    │ SLICE_ID  │                     │ USER_ID (FK)     │                              │
    │ TITLE     │                     │ NAME             │                              │
    │ STATUS    │                     │ DESCRIPTION      │                              │
    │ PRIORITY  │                     │ CONVENTIONS      │                              │
    │ RETRY_*   │                     │ SHARED_FILES     │                              │
    └─────┬─────┘                     └────────┬─────────┘                              │
          │                                    │                                        │
          │ 1:N                                │ N:M                                    │
          │                                    │                                        │
    ┌─────▼───────────┐               ┌────────▼─────────┐                              │
    │  TASK_EVENTS    │               │ CLUSTER_MEMBERS  │                              │
    │─────────────────│               │  (JOIN TABLE)    │                              │
    │ ID (PK)         │               │──────────────────│                              │
    │ TASK_ID (FK)    │               │ CLUSTER_ID (FK)  │                              │
    │ EVENT_TYPE      │               │ AGENT_ID (FK)    │                              │
    │ AGENT_ID        │               │ ROLE             │                              │
    │ DETAILS         │               │ PERSONA_OVERRIDE │                              │
    └─────────────────┘               └──────────────────┘                              │
          │                                                                             │
    ┌─────▼───────────┐                                                                 │
    │TASK_DEPENDENCIES│                                                                 │
    │  (JOIN TABLE)   │                                                                 │
    │─────────────────│                                                                 │
    │ TASK_ID (FK)    │                                                                 │
    │ DEPENDS_ON (FK) │                                                                 │
    └─────────────────┘                                                                 │
                                                                                        │
         ├──────────────────────────────────────────────────────────────────────────────┘
         │
         │  WORKFLOW & DAG SYSTEM
         │
┌────────▼────────┐                                    ┌──────────────────┐
│    WORKFLOWS    │                                    │ OUTPUT_SCHEMAS   │
│─────────────────│                                    │──────────────────│
│ ID (PK)         │                                    │ ID (PK)          │
│ USER_ID (FK)    │                                    │ USER_ID (FK)     │
│ NAME            │                                    │ NAME (UQ/USER)   │
│ DESCRIPTION     │                                    │ SCHEMA (JSONB)   │
│ EXECUTION_MODE  │                                    │ VERSION          │
│ VERSION         │                                    └────────┬─────────┘
└────────┬────────┘                                             │
         │                                                      │
         │ 1:N                          ┌───────────────────────┘
         │                              │
┌────────▼──────────────┐       ┌───────┘       ┌──────────────────┐
│    WORKFLOW_STEPS     │       │               │ PROMPT_TEMPLATES │
│───────────────────────│       │               │──────────────────│
│ ID (PK)               │       │               │ ID (PK)          │
│ WORKFLOW_ID (FK)      │       │               │ USER_ID (FK)     │
│ AGENT_ID (FK)         │       │               │ NAME (UQ/USER)   │
│ EXECUTION_MODE        │◄──────┘               │ CONTENT          │
│ AGENT_EXECUTION_MODE  │                       │ VERSION          │
│ FOR_EACH_REF          │◄──────────────────────┘                  │
│ FOR_EACH_LABEL_FIELD  │  PROMPT_TEMPLATE_ID(FK)                  │
│ PROMPT_TEMPLATE       │                       └──────────────────┘
│ OUTPUT_SCHEMA_ID (FK) │
│ OUTPUT_VARIABLE_NAME  │
│ INTERACTIVE_AGENT_ID  │
│ ROOM_ID (FK)          │
│ DISPLAY_ORDER         │
│ VERSION               │
└────────┬──────────────┘
         │
         ├──────────────────────────────────────────┐
         │                                          │
         │ N:M (DAG EDGES)                          │ N:M (MULTI-AGENT)
         │                                          │
┌────────▼──────────────┐               ┌───────────▼──────────┐
│ WORKFLOW_STEP_EDGES   │               │ WORKFLOW_STEP_AGENTS │
│───────────────────────│               │──────────────────────│
│ FROM_STEP_ID (FK)     │               │ STEP_ID (FK)         │
│ TO_STEP_ID (FK)       │               │ AGENT_ID (FK)        │
└───────────────────────┘               │ EXECUTION_STRATEGY   │
                                        │ AGENT_ORDER          │
         │                              └──────────────────────┘
         │
         │ N:M (STEP CONTEXT)
         │
┌────────▼──────────────┐
│   STEP_DOCUMENTS      │
│   (JOIN TABLE)        │
│───────────────────────│
│ STEP_ID (FK)          │
│ DOCUMENT_ID (FK)      │────────────────────────►  DOCUMENTS
└───────────────────────┘


         ├──────────────────────────────────────────────────────────────────────────────┐
         │                                                                              │
         │  COLLECTION SYSTEM (DAG OF WORKFLOWS)                                        │
         │                                                                              │
┌────────▼──────────────────┐                                                           │
│  WORKFLOW_COLLECTIONS     │                                                           │
│───────────────────────────│                                                           │
│ ID (PK)                   │                                                           │
│ USER_ID (FK)              │                                                           │
│ NAME                      │                                                           │
│ DESCRIPTION               │                                                           │
│ EXECUTION_MODE            │                                                           │
└────────┬──────────────────┘                                                           │
         │                                                                              │
         ├──────────────────────┬───────────────────────┐                               │
         │                      │                       │                               │
         │ 1:N                  │ 1:N (DAG EDGES)       │ 1:N                           │
         │                      │                       │                               │
┌────────▼──────────────┐ ┌────▼───────────────────┐ ┌──▼──────────────┐               │
│ COLLECTION_WORKFLOWS  │ │COLLECTION_WORKFLOW_EDGES│ │    ROOMS        │               │
│   (JOIN TABLE)        │ │────────────────────────│ │─────────────────│               │
│───────────────────────│ │ FROM_WORKFLOW_ID (FK)  │ │ ID (PK)         │               │
│ COLLECTION_ID (FK)    │ │ TO_WORKFLOW_ID (FK)    │ │ USER_ID (FK)    │               │
│ WORKFLOW_ID (FK)      │ │ COLLECTION_ID (FK)     │ │ COLLECTION_ID   │               │
│ DISPLAY_ORDER         │ └────────────────────────┘ │ NAME            │               │
│ EXECUTION_MODE        │                            │ GATEKEEPER_*    │               │
└───────────────────────┘                            │ MAX_SPEAKERS    │               │
         │                                           │ MAX_TURNS       │               │
         │                                           │ TOOLS_ENABLED   │               │
         └───────────►  WORKFLOWS                    └────────┬────────┘               │
                                                              │                        │
                                                    ┌─────────┴──────────┐             │
                                                    │                    │              │
                                                    │ N:M                │ 1:N          │
                                                    │                    │              │
                                           ┌────────▼────────┐ ┌────────▼────────┐     │
                                           │  ROOM_MEMBERS   │ │ ROOM_SESSIONS   │     │
                                           │  (JOIN TABLE)   │ │─────────────────│     │
                                           │─────────────────│ │ ID (PK)         │     │
                                           │ ROOM_ID (FK)    │ │ ROOM_ID (FK)    │     │
                                           │ AGENT_ID (FK)   │ │ STATUS          │     │
                                           │ DISPLAY_NAME    │ │ CURRENT_TURN    │     │
                                           │ ROLE_DESCRIPTION│ │ TRANSCRIPT_SUM  │     │
                                           │ DISPLAY_ORDER   │ │ STARTED_AT      │     │
                                           └─────────────────┘ │ COMPLETED_AT    │     │
                                                               └────────┬────────┘     │
                                                                        │              │
                                                                        │ 1:N          │
                                                                        ▼              │
                                                              AGENT_EXECUTIONS         │
                                                                                       │
         ├─────────────────────────────────────────────────────────────────────────────┘
         │
         │  EXECUTION RUNTIME
         │
┌────────▼──────────────┐
│    COLLECTION_RUNS    │
│───────────────────────│
│ ID (PK)               │
│ COLLECTION_ID (FK)    │
│ USER_ID (FK)          │
│ STATUS                │
│ STARTED_AT            │
│ COMPLETED_AT          │
│ ERROR                 │
└────────┬──────────────┘
         │
         │ 1:N
         │
┌────────▼──────────────┐
│ WORKFLOW_EXECUTIONS   │
│───────────────────────│
│ ID (PK)               │
│ COLLECTION_RUN_ID (FK)│
│ WORKFLOW_ID (FK)      │
│ USER_ID (FK)          │
│ STATUS                │
│ STARTED_AT            │
│ COMPLETED_AT          │
│ OUTPUTS (JSONB)       │
│ ERROR                 │
└────────┬──────────────┘
         │
         │ 1:N
         │
┌────────▼──────────────────┐
│    AGENT_EXECUTIONS       │
│───────────────────────────│
│ ID (PK)                   │
│ AGENT_ID (FK)             │
│ WORKFLOW_STEP_ID (FK)     │
│ WORKFLOW_EXECUTION_ID (FK)│
│ IS_INTERACTIVE            │
│ PARENT_EXEC_ID (FK) ◄────┼──┐ (SELF-REFERENCING)
│ SELECTED_MODE_ID (FK)     │  │
│ SYSTEM_PROMPT_RENDERED    │  │
│ INPUT                     │  │
│ OUTPUT                    │  │
│ STRUCTURED_OUTPUT (JSONB) │  │
│ ROOM_SESSION_ID (FK)      │  │
│ SPEAKER_ORDER             │  │
│ STATUS                    │  │
│ STARTED_AT                │  │
│ COMPLETED_AT              │  │
└────────┬──────────────────┘  │
         │                     │
         ├─────────────────────┘
         │
         ├──────────────────────┬──────────────────────┬──────────────────┐
         │                      │                      │                  │
         │ 1:N                  │ 1:N                  │ 1:N              │ 1:N
         │                      │                      │                  │
┌────────▼──────────────┐ ┌────▼──────────────┐ ┌─────▼──────────┐ ┌────▼──────────────┐
│ EXECUTION_MESSAGES    │ │  TOKEN_LEDGER     │ │   RESULTS      │ │EXECUTION_VARIABLES│
│───────────────────────│ │───────────────────│ │────────────────│ │───────────────────│
│ ID (PK)               │ │ ID (PK)           │ │ ID (PK)        │ │ ID (PK)           │
│ AGENT_EXEC_ID (FK)    │ │ USER_ID (FK)      │ │ USER_ID (FK)   │ │ COLLECTION_RUN_ID │
│ ROLE                  │ │ AGENT_EXEC_ID (FK)│ │ AGENT_EXEC_ID  │ │ WORKFLOW_EXEC_ID  │
│ CONTENT               │ │ MODEL_ID          │ │ OUTPUT_SCHEMA_ID│ │ STEP_EXEC_ID     │
│ TOOL_CALL_ID          │ │ INPUT_TOKENS      │ │ NAME           │ │ VARIABLE_NAME     │
│ INPUT_TOKENS          │ │ OUTPUT_TOKENS     │ │ DATA (JSONB)   │ │ VARIABLE_PATH     │
│ OUTPUT_TOKENS         │ │ COST_USD          │ └────────────────┘ │ VALUE (JSONB)     │
│ CREATED_AT            │ │ CREATED_AT        │                    │ CREATED_AT        │
└───────────────────────┘ └───────────────────┘                    └───────────────────┘


         ├──────────────────────────────────────────────────────────────────────────────┐
         │                                                                              │
         │  CHAT & SESSION SYSTEM                                                       │
         │                                                                              │
┌────────▼──────────────┐                                                               │
│    CHAT_SESSIONS      │                                                               │
│───────────────────────│                                                               │
│ ID (PK)               │                                                               │
│ USER_ID (FK)          │                                                               │
│ AGENT_ID (FK)         │────────────────────────────────────────────►  AGENTS           │
│ MODE_ID               │                                                               │
│ TITLE                 │                                                               │
│ SUMMARY               │                                                               │
└────────┬──────────────┘                                                               │
         │                                                                              │
         ├──────────────────┬───────────────────┬───────────────────┐                   │
         │                  │                   │                   │                   │
         │ 1:N              │ 1:N               │ 1:N               │ 1:N               │
         │                  │                   │                   │                   │
┌────────▼────────┐ ┌───────▼────────┐ ┌────────▼────────┐ ┌───────▼────────┐          │
│ CHAT_MESSAGES   │ │ CONTEXT_STORE  │ │ROUTER_REQUESTS  │ │  DOCUMENTS     │          │
│─────────────────│ │────────────────│ │─────────────────│ │  (CREATED IN   │          │
│ ID (PK)         │ │ ID (PK)        │ │ ID (PK)         │ │   SESSION)     │          │
│ USER_ID (FK)    │ │ SESSION_ID (FK)│ │ SESSION_ID (FK) │ └────────────────┘          │
│ SESSION_ID (FK) │ │ SOURCE         │ │ AGENT_EXEC_ID   │                             │
│ ROLE            │ │ PRIORITY       │ │ INTENT          │                             │
│ CONTENT         │ │ CONTENT        │ │ PRIORITY        │                             │
│ TIMESTAMP       │ │ METADATA (JSON)│ │ CALLBACK_HINT   │                             │
└─────────────────┘ │ STATUS         │ │ ROUTED_TOOL     │                             │
                    │ EXPIRES_AT     │ │ ROUTED_ARGS     │                             │
                    └────────────────┘ │ IS_ASYNC        │                             │
                                       │ PASSDOWN        │                             │
                                       │ CHAIN (JSONB)   │                             │
                                       │ STATUS          │                             │
                                       │ RESULT          │                             │
                                       └─────────────────┘                             │
                                                                                       │
         ├─────────────────────────────────────────────────────────────────────────────┘
         │
         │  LEGACY TICKET SYSTEM
         │
┌────────▼──────────┐
│     TICKETS       │  (DEPRECATED)
│───────────────────│
│ ID (PK)           │
│ USER_ID (FK)      │
│ SOURCE_TYPE       │
│ SOURCE_OWNER      │
│ SOURCE_REPO       │
│ SOURCE_ISSUE_NUM  │
│ TITLE             │
│ DESCRIPTION       │
│ LABELS (JSONB)    │
│ STATUS            │
└────────┬──────────┘
         │
         │ 1:N
         │
┌────────▼──────────┐
│  VERTICAL_SLICES  │  (DEPRECATED)
│───────────────────│
│ ID (PK)           │
│ USER_ID (FK)      │
│ TICKET_ID (FK)    │
│ TITLE             │
│ DESCRIPTION       │
│ STATUS            │
└───────────────────┘
```

## KEY RELATIONSHIPS SUMMARY

### EXECUTION HIERARCHY
```
WORKFLOW_COLLECTIONS
  └── COLLECTION_RUNS
      └── WORKFLOW_EXECUTIONS
          └── AGENT_EXECUTIONS (CAN HAVE WORKFLOW_STEP_ID)
              ├── EXECUTION_MESSAGES
              ├── TOKEN_LEDGER
              ├── RESULTS (STRUCTURED OUTPUTS)
              └── EXECUTION_VARIABLES
```

### AGENT CONFIGURATION
```
AGENTS
  ├── AGENT_TOOLS ──► TOOLS (WHAT TOOLS CAN AGENT USE)
  ├── AGENT_CONTEXT ──► DOCUMENTS (KNOWLEDGE BASE)
  ├── AGENT_MODES (BEHAVIORAL VARIANTS)
  └── CLUSTER_MEMBERS ──► CLUSTERS (TEAM MEMBERSHIP)
```

### WORKFLOW DAG
```
WORKFLOWS
  └── WORKFLOW_STEPS (NODES)
      ├── WORKFLOW_STEP_EDGES (DEFINES EXECUTION ORDER)
      ├── WORKFLOW_STEP_AGENTS (MULTI-AGENT PER STEP)
      ├── STEP_DOCUMENTS ──► DOCUMENTS (STEP CONTEXT)
      ├── PROMPT_TEMPLATE_ID ──► PROMPT_TEMPLATES
      └── OUTPUT_SCHEMA_ID ──► OUTPUT_SCHEMAS
```

### COLLECTION DAG (DAG OF WORKFLOWS)
```
WORKFLOW_COLLECTIONS
  ├── COLLECTION_WORKFLOWS ──► WORKFLOWS
  ├── COLLECTION_WORKFLOW_EDGES (DEFINES WORKFLOW ORDER)
  ├── COLLECTION_RUNS (EXECUTION RECORDS)
  └── ROOMS (MULTI-AGENT DISCUSSION SPACES)
```

### ROOM COLLABORATION
```
ROOMS
  ├── ROOM_MEMBERS ──► AGENTS
  └── ROOM_SESSIONS
      └── AGENT_EXECUTIONS (SPEAKER_ORDER TRACKS TURN)
```

### CHAT SYSTEM
```
CHAT_SESSIONS
  ├── CHAT_MESSAGES
  ├── CONTEXT_STORE (CONTEXTUAL DATA)
  ├── ROUTER_REQUESTS ──► AGENT_EXECUTIONS
  └── DOCUMENTS (CREATED IN SESSION)
```

## VERSION HISTORY TABLES

THE FOLLOWING TABLES HAVE ASSOCIATED `_VERSIONS` HISTORY TABLES:

```
AGENTS              ──►  AGENTS_VERSIONS
AGENT_MODES         ──►  AGENT_MODES_VERSIONS
TOOLS               ──►  TOOLS_VERSIONS
WORKFLOWS           ──►  WORKFLOWS_VERSIONS
WORKFLOW_STEPS      ──►  WORKFLOW_STEPS_VERSIONS
OUTPUT_SCHEMAS      ──►  OUTPUT_SCHEMAS_VERSIONS
PROMPT_TEMPLATES    ──►  PROMPT_TEMPLATES_VERSIONS
```

EACH VERSION TABLE CONTAINS: `ID`, `VERSION`, ALL ENTITY COLUMNS, `CHANGED_BY` (UUID), `CHANGED_AT` (TIMESTAMPTZ)

## JOIN TABLE INDEX

```
AGENT_TOOLS             ──  AGENTS        ◄──N:M──►  TOOLS
TOOL_ROUTER_TOOLS       ──  TOOL_ROUTERS  ◄──N:M──►  TOOLS
CLUSTER_MEMBERS         ──  CLUSTERS      ◄──N:M──►  AGENTS
WORKFLOW_STEP_AGENTS    ──  WORKFLOW_STEPS◄──N:M──►  AGENTS
WORKFLOW_STEP_EDGES     ──  WORKFLOW_STEPS◄──DAG──►  WORKFLOW_STEPS
STEP_DOCUMENTS          ──  WORKFLOW_STEPS◄──N:M──►  DOCUMENTS
COLLECTION_WORKFLOWS    ──  COLLECTIONS   ◄──N:M──►  WORKFLOWS
COLLECTION_WORKFLOW_EDGES── COLLECTIONS   ◄──DAG──►  WORKFLOWS
ROOM_MEMBERS            ──  ROOMS         ◄──N:M──►  AGENTS
AGENT_CONTEXT           ──  AGENTS        ◄──N:M──►  DOCUMENTS
TASK_DEPENDENCIES       ──  TASKS         ◄──DAG──►  TASKS
```
