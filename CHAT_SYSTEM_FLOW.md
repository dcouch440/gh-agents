# Chat System Flow Guide

This document provides ASCII flow diagrams for the Nexor chat system architecture.

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              FRONTEND (React)                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  ChatInput  │  │  ChatPanel  │  │ ChatMessage │  │ WebSocketContext    │ │
│  │  (User UI)  │  │  (Display)  │  │  (Render)   │  │ (Real-time events)  │ │
│  └──────┬──────┘  └──────▲──────┘  └──────▲──────┘  └──────────▲──────────┘ │
│         │                │                │                    │            │
│         │         ┌──────┴────────────────┴────────┐           │            │
│         │         │       ChatContext              │           │            │
│         │         │  (State: messages, loading)    │           │            │
│         │         └──────▲─────────────────────────┘           │            │
│         │                │                                     │            │
│         ▼                │                                     │            │
│  ┌──────────────────────────────────────┐                      │            │
│  │       useChatMutations (Hook)        │                      │            │
│  │  - useSendMessage()                  │                      │            │
│  │  - useSendSessionMessage()           │                      │            │
│  └──────┬───────────────────────────────┘                      │            │
│         │                                                      │            │
└─────────┼──────────────────────────────────────────────────────┼────────────┘
          │                                                      │
          │  HTTP POST + SSE Stream                    WebSocket │
          ▼                                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              BACKEND (Rust/Axum)                            │
│  ┌─────────────────────┐     ┌─────────────────────┐     ┌────────────────┐ │
│  │   /api/chat         │     │   /api/chat/{id}/   │     │    /ws         │ │
│  │   (POST endpoint)   │     │   stream (SSE)      │     │  (WebSocket)   │ │
│  └──────────┬──────────┘     └──────────▲──────────┘     └────────▲───────┘ │
│             │                           │                         │         │
│             ▼                           │                         │         │
│  ┌──────────────────────┐    ┌─────────┴─────────┐    ┌──────────┴───────┐ │
│  │   Chat Consumer      │    │   BufferedStream  │    │  Broadcast       │ │
│  │   (Background Task)  │───▶│   (Token Buffer)  │    │  Channels        │ │
│  └──────────┬───────────┘    └───────────────────┘    └──────────────────┘ │
│             │                                                               │
│             ▼                                                               │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                        Execution Engine                                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐ │ │
│  │  │ChatStrategy │  │ModeResolver │  │ToolExecutor │  │   SseSink     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └───────────────┘ │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│             │                                                               │
└─────────────┼───────────────────────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           EXTERNAL SERVICES                                 │
│  ┌─────────────────────┐                    ┌─────────────────────────────┐ │
│  │   Anthropic API     │                    │       PostgreSQL            │ │
│  │   (Claude LLM)      │                    │   - chat_messages           │ │
│  │                     │                    │   - session_messages        │ │
│  │   Streaming tokens  │                    │   - sessions                │ │
│  └─────────────────────┘                    └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Step-by-Step Message Flow

### Step 1: User Sends Message

```
┌──────────────────────────────────────────────────────────────────┐
│                     USER TYPES MESSAGE                           │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  ChatInput Component                                             │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  [Hello, can you help me with this code?      ] [Send]     │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  - User types message                                            │
│  - Shift+Enter = new line                                        │
│  - Enter = send message                                          │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ onSubmit()
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  useSendMessage Hook                                             │
│                                                                  │
│  1. Add optimistic user message to ChatContext                   │
│  2. POST /api/chat { message: "..." }                            │
│  3. Receive { message_id, status: "queued" }                     │
│  4. Open SSE stream: GET /api/chat/{message_id}/stream           │
└──────────────────────────────────────────────────────────────────┘
```

---

### Step 2: Backend Receives & Queues Message

```
┌──────────────────────────────────────────────────────────────────┐
│                  POST /api/chat Handler                          │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Validation                                                      │
│  ├─ Check JWT token (extract user_id)                            │
│  ├─ Validate message length < MAX_CHAT_MESSAGE_LENGTH            │
│  └─ Trim whitespace                                              │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Pre-create Response Stream                                      │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  BufferedStream {                                          │  │
│  │    tx: broadcast::Sender<StreamChunk>,                     │  │
│  │    buffer: Vec<StreamChunk>,  // Captures all tokens       │  │
│  │    done: false                                             │  │
│  │  }                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
│  Stored in: AppState.response_streams[message_id]                │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Store User Message                                              │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  INSERT INTO chat_messages                                 │  │
│  │    (id, user_id, role, content, timestamp)                 │  │
│  │  VALUES                                                    │  │
│  │    ($uuid, $user_id, 'user', $message, NOW())              │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Queue to Chat Consumer                                          │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  state.chat_tx.send(ChatRequest {                          │  │
│  │    message_id,                                             │  │
│  │    user_id,                                                │  │
│  │    message,                                                │  │
│  │    session_id: None  // or Some(id) for session chat       │  │
│  │  })                                                        │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Return Response                                                 │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  HTTP 202 Accepted                                         │  │
│  │  { "message_id": "abc-123", "status": "queued" }           │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

### Step 3: Chat Consumer Processes Message

```
┌──────────────────────────────────────────────────────────────────┐
│                  Chat Consumer (Background Task)                 │
│                  Runs: tokio::spawn(chat_consumer_task())        │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ Receives from chat_rx channel
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Load Context                                                    │
│  ├─ Get user's default agent from DB                             │
│  ├─ Get session (if session_id provided)                         │
│  └─ Load session history (up to 50 messages)                     │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Create SseSink                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  SseSink {                                                 │  │
│  │    message_id,                                             │  │
│  │    state: AppState,  // For sending chunks                 │  │
│  │  }                                                         │  │
│  │                                                            │  │
│  │  impl Sink for SseSink:                                    │  │
│  │    fn send_token(&self, text)                              │  │
│  │    fn send_tool_start(&self, name, id)                     │  │
│  │    fn send_tool_end(&self, name, id)                       │  │
│  │    fn send_done(&self)                                     │  │
│  │    fn send_error(&self, err)                               │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Call Execution Engine                                           │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  hub::run_chat(                                            │  │
│  │    agent,                                                  │  │
│  │    message,                                                │  │
│  │    session,                                                │  │
│  │    sink: SseSink,                                          │  │
│  │    tools,                                                  │  │
│  │  )                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

### Step 4: ChatStrategy Builds LLM Request

```
┌──────────────────────────────────────────────────────────────────┐
│                    ChatStrategy::build_messages()                │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Load Prior Context                                              │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  1. Load session summary (if exists)                       │  │
│  │  2. Run "context distiller" (Haiku LLM call)               │  │
│  │     - Extracts relevant prior context for current query    │  │
│  │  3. Inject as system message                               │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Build Messages Array                                            │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  [                                                         │  │
│  │    { role: "system", content: $system_prompt },            │  │
│  │    { role: "system", content: $prior_context },            │  │
│  │    { role: "user", content: $history[0] },                 │  │
│  │    { role: "assistant", content: $history[1] },            │  │
│  │    ...                                                     │  │
│  │    { role: "user", content: $current_message }             │  │
│  │  ]                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Resolve Tools                                                   │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  ModeResolver::resolve_tools()                             │  │
│  │  ├─ Filter by agent config                                 │  │
│  │  ├─ Filter by mode settings                                │  │
│  │  └─ Return: Vec<Tool> with JSON schemas                    │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

### Step 5: LLM Call & Token Streaming

```
┌──────────────────────────────────────────────────────────────────┐
│                    Call Anthropic API                            │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ POST https://api.anthropic.com/v1/messages
                              │ stream: true
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Anthropic Claude LLM                                            │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Model: claude-3-5-sonnet (or configured)                  │  │
│  │                                                            │  │
│  │  Streaming Response:                                       │  │
│  │  ─────────────────────────────────────────────────────     │  │
│  │  event: content_block_delta                                │  │
│  │  data: {"delta": {"text": "Hello"}}                        │  │
│  │                                                            │  │
│  │  event: content_block_delta                                │  │
│  │  data: {"delta": {"text": "! I"}}                          │  │
│  │                                                            │  │
│  │  event: content_block_delta                                │  │
│  │  data: {"delta": {"text": " can"}}                         │  │
│  │  ...                                                       │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ For each token delta
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  SseSink::send_token()                                           │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  state.send_stream_chunk(message_id, StreamChunk::Token(   │  │
│  │    "Hello"                                                 │  │
│  │  ))                                                        │  │
│  │                                                            │  │
│  │  BufferedStream:                                           │  │
│  │  ├─ buffer.push(chunk)     // Store for late clients       │  │
│  │  └─ tx.send(chunk)         // Broadcast to SSE listeners   │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

### Step 6: Tool Execution (If Needed)

```
┌──────────────────────────────────────────────────────────────────┐
│            LLM Requests Tool Use                                 │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  {                                                         │  │
│  │    "type": "tool_use",                                     │  │
│  │    "name": "create_document",                              │  │
│  │    "input": { "title": "...", "content": "..." }           │  │
│  │  }                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Notify Frontend: Tool Start                                     │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  sink.send_tool_start("create_document", tool_id)          │  │
│  │                                                            │  │
│  │  SSE Event:                                                │  │
│  │  event: tool_start                                         │  │
│  │  data: {"name": "create_document", "tool_id": "xyz"}       │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Execute Tool                                                    │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  ToolExecutor::execute_tool()                              │  │
│  │  ├─ Parse tool input                                       │  │
│  │  ├─ Run tool logic (DB insert, API call, etc.)             │  │
│  │  └─ Return ToolResult                                      │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Notify Frontend: Tool End                                       │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  sink.send_tool_end("create_document", tool_id)            │  │
│  │                                                            │  │
│  │  SSE Event:                                                │  │
│  │  event: tool_end                                           │  │
│  │  data: {"name": "create_document", "tool_id": "xyz"}       │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ Tool result fed back to LLM
                              │ LLM continues generating response
                              ▼
                           [Back to Step 5]
```

---

### Step 7: Frontend Receives SSE Stream

```
┌──────────────────────────────────────────────────────────────────┐
│               Frontend: SSE Stream Handler                       │
│               GET /api/chat/{message_id}/stream                  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  createSSEStream() - frontend/src/api/sse.ts                     │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  const response = await fetch(url, {                       │  │
│  │    headers: { Accept: "text/event-stream" }                │  │
│  │  });                                                       │  │
│  │                                                            │  │
│  │  const reader = response.body.getReader();                 │  │
│  │  const decoder = new TextDecoder();                        │  │
│  │                                                            │  │
│  │  while (true) {                                            │  │
│  │    const { done, value } = await reader.read();            │  │
│  │    if (done) break;                                        │  │
│  │    parseSSE(decoder.decode(value));                        │  │
│  │  }                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ Parse SSE events
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  SSE Event Types                                                 │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                                                            │  │
│  │  event: token          ──▶  Append to message content      │  │
│  │  data: "Hello"                                             │  │
│  │                                                            │  │
│  │  event: tool_start     ──▶  Show tool indicator            │  │
│  │  data: {...}                                               │  │
│  │                                                            │  │
│  │  event: tool_end       ──▶  Hide tool indicator            │  │
│  │  data: {...}                                               │  │
│  │                                                            │  │
│  │  event: doc_update     ──▶  Update document reference      │  │
│  │  data: {...}                                               │  │
│  │                                                            │  │
│  │  event: error          ──▶  Display error message          │  │
│  │  data: "Error text"                                        │  │
│  │                                                            │  │
│  │  event: done           ──▶  Mark streaming complete        │  │
│  │  data: ""                                                  │  │
│  │                                                            │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Update ChatContext                                              │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  dispatch({ type: 'APPEND', payload: {                     │  │
│  │    id: message_id,                                         │  │
│  │    role: 'assistant',                                      │  │
│  │    content: accumulated_tokens,                            │  │
│  │    isStreaming: true                                       │  │
│  │  }});                                                      │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

### Step 8: Post-Completion Processing

```
┌──────────────────────────────────────────────────────────────────┐
│                  LLM Response Complete                           │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  1. Send Done Event                                              │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  sink.send_done()                                          │  │
│  │  ──▶ SSE: event: done, data: ""                            │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  2. Save Assistant Response                                      │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  INSERT INTO chat_messages                                 │  │
│  │    (id, user_id, role, content, timestamp)                 │  │
│  │  VALUES                                                    │  │
│  │    ($uuid, $user_id, 'assistant', $full_response, NOW())   │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  3. Record Token Usage                                           │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  TokenLedger::record_usage(                                │  │
│  │    user_id,                                                │  │
│  │    input_tokens: 1234,                                     │  │
│  │    output_tokens: 567,                                     │  │
│  │    model: "claude-3-5-sonnet",                             │  │
│  │  )                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  4. Background Tasks (tokio::spawn)                              │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                                                            │  │
│  │  ┌─ Auto-naming (if session title == "New Session")        │  │
│  │  │  └─ Call Haiku to generate title from conversation      │  │
│  │  │                                                         │  │
│  │  └─ Compaction (if message_count > 30)                     │  │
│  │     └─ Summarize older messages                            │  │
│  │     └─ Update session.summary                              │  │
│  │                                                            │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  5. Broadcast Session Update (WebSocket)                         │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  state.broadcast(Channel::Sessions, SessionUpdate {        │  │
│  │    session_id,                                             │  │
│  │    title: "New title",                                     │  │
│  │    updated_at: now()                                       │  │
│  │  })                                                        │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

## WebSocket Real-Time Updates

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        WebSocket Connection Flow                            │
└─────────────────────────────────────────────────────────────────────────────┘

Frontend                                              Backend
────────                                              ───────
    │                                                     │
    │  ws://localhost:3000/ws?token=JWT                   │
    │ ──────────────────────────────────────────────────▶ │
    │                                                     │
    │                     Connection Established          │
    │ ◀────────────────────────────────────────────────── │
    │                                                     │
    │  { "type": "Subscribe",                             │
    │    "channels": ["sessions", "feed", "tasks"] }      │
    │ ──────────────────────────────────────────────────▶ │
    │                                                     │
    │  { "type": "Subscribed",                            │
    │    "channels": ["sessions", "feed", "tasks"] }      │
    │ ◀────────────────────────────────────────────────── │
    │                                                     │
    │                                                     │
    │         ... time passes, events occur ...           │
    │                                                     │
    │                                                     │
    │  { "type": "SessionUpdate",                         │
    │    "data": { "id": "...", "title": "..." } }        │
    │ ◀────────────────────────────────────────────────── │
    │                                                     │
    │  { "type": "TaskUpdate",                            │
    │    "data": { "id": "...", "status": "..." } }       │
    │ ◀────────────────────────────────────────────────── │
    │                                                     │
    │                                                     │
    │  Ping (every 30s)                                   │
    │ ◀────────────────────────────────────────────────── │
    │                                                     │
    │  Pong                                               │
    │ ──────────────────────────────────────────────────▶ │
    │                                                     │


WebSocket Channels:
┌──────────────┬────────────────────────────────────────────────┐
│ Channel      │ Events                                         │
├──────────────┼────────────────────────────────────────────────┤
│ feed         │ Activity feed updates (new items)              │
│ tasks        │ Task status changes (created, completed, etc.) │
│ agents       │ Agent status updates (online, busy, etc.)      │
│ sessions     │ Session changes (renamed, updated)             │
│ pipelines    │ Pipeline execution updates                     │
│ routing      │ Tool routing updates                           │
└──────────────┴────────────────────────────────────────────────┘
```

---

## Buffered Stream Mechanism

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   Why Buffered Streams?                                     │
│                                                                             │
│   Problem: Client may connect to SSE AFTER tokens have been generated       │
│   Solution: Buffer all chunks, replay on connection                         │
└─────────────────────────────────────────────────────────────────────────────┘

Timeline:
─────────────────────────────────────────────────────────────────────────────▶

 T0          T1          T2          T3          T4          T5
 │           │           │           │           │           │
 ▼           ▼           ▼           ▼           ▼           ▼

 POST        Token1      Token2      Client      Token3      Done
 /chat       generated   generated   connects    generated
 │           │           │           │           │           │
 │           │           │           │           │           │
 ▼           ▼           ▼           ▼           ▼           ▼
┌─────────────────────────────────────────────────────────────────┐
│  BufferedStream                                                 │
│  buffer: [ ]  [ T1 ]  [ T1,T2 ]  [ T1,T2 ]  [ T1,T2,T3 ]       │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Client connects at T3
                                    │
                                    ▼
                          ┌─────────────────────┐
                          │  Replay buffer:     │
                          │  ├─ Token1          │
                          │  ├─ Token2          │
                          │  └─ (then live)     │
                          │      └─ Token3      │
                          │      └─ Done        │
                          └─────────────────────┘


BufferedStream Structure:
┌──────────────────────────────────────────────────────────────────┐
│  struct BufferedStream {                                         │
│      tx: broadcast::Sender<StreamChunk>,  // Live broadcast      │
│      buffer: Vec<StreamChunk>,            // Historical chunks   │
│      done: bool,                          // Completion flag     │
│  }                                                               │
│                                                                  │
│  enum StreamChunk {                                              │
│      Token(String),                       // LLM token           │
│      ToolStart { name, tool_id },         // Tool started        │
│      ToolEnd { name, tool_id },           // Tool completed      │
│      DocUpdate { doc_id, title },         // Document created    │
│      Done,                                // Stream complete     │
│      Error(String),                       // Error occurred      │
│  }                                                               │
└──────────────────────────────────────────────────────────────────┘
```

---

## Error Handling Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                    Error Scenarios                               │
└──────────────────────────────────────────────────────────────────┘

1. LLM API Error:
   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
   │   LLM API   │────▶│   SseSink   │────▶│   Frontend  │
   │   Error     │     │   .error()  │     │   Display   │
   └─────────────┘     └─────────────┘     └─────────────┘
                             │
                             ▼
                       SSE: event: error
                       data: "LLM request failed: ..."

2. Tool Execution Error:
   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
   │   Tool      │────▶│  Return to  │────▶│ LLM handles │
   │   Fails     │     │    LLM      │     │   gracefully│
   └─────────────┘     └─────────────┘     └─────────────┘
                                                 │
                                                 ▼
                                          Continues chat,
                                          explains error

3. Network/Timeout Error:
   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
   │   Timeout   │────▶│   Retry     │────▶│   Error     │
   │   Occurs    │     │   Logic     │     │   Response  │
   └─────────────┘     └─────────────┘     └─────────────┘
                                                 │
                                                 ▼
                                          Frontend shows
                                          retry option


Frontend Error States (ChatContext):
┌──────────────────────────────────────────────────────────────────┐
│  type State = {                                                  │
│    messages: ChatMessage[];                                      │
│    isLoading: boolean;                                           │
│    error: string | null;  ◀── Set on error, cleared on retry    │
│  }                                                               │
└──────────────────────────────────────────────────────────────────┘
```

---

## Database Schema

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Database Tables                                   │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────┐
│           chat_messages             │
├─────────────────────────────────────┤
│ id          UUID PRIMARY KEY        │
│ user_id     UUID → users            │
│ role        TEXT ('user'|'assistant')│
│ content     TEXT                    │
│ timestamp   TIMESTAMP               │
└─────────────────────────────────────┘
              │
              │ Global chat (no session)
              │
              ▼
┌─────────────────────────────────────┐
│         session_messages            │
├─────────────────────────────────────┤
│ id          UUID PRIMARY KEY        │
│ session_id  UUID → sessions         │
│ user_id     UUID → users            │
│ role        TEXT                    │
│ content     TEXT                    │
│ timestamp   TIMESTAMP               │
└─────────────────────────────────────┘
              │
              │ Belongs to
              ▼
┌─────────────────────────────────────┐
│            sessions                 │
├─────────────────────────────────────┤
│ id          UUID PRIMARY KEY        │
│ user_id     UUID → users            │
│ mode_id     UUID                    │
│ agent_id    UUID → persisted_agents │
│ title       TEXT DEFAULT 'New...'   │
│ summary     TEXT DEFAULT ''         │
│ created_at  TIMESTAMP               │
│ updated_at  TIMESTAMP               │
└─────────────────────────────────────┘
```

---

## Key Files Reference

| Layer | Component | File Path |
|-------|-----------|-----------|
| **Frontend** | WebSocket Context | `frontend/src/contexts/WebSocketContext.tsx` |
| | Chat Context | `frontend/src/contexts/ChatContext.tsx` |
| | Chat Mutations | `frontend/src/hooks/useChatMutations.ts` |
| | SSE Handler | `frontend/src/api/sse.ts` |
| | Chat Components | `frontend/src/components/chat/` |
| **Backend** | WebSocket Server | `src/server/ws/mod.rs` |
| | Chat API | `src/server/api/chat/mod.rs` |
| | Chat Consumer | `src/server/chat_consumer/mod.rs` |
| | Chat Strategy | `src/server/hub/strategies/chat/mod.rs` |
| | Streaming/Sink | `src/server/hub/streaming/mod.rs` |
| | App State | `src/server/state/mod.rs` |
| | DB Queries | `src/db/queries/mod.rs` |
| | Hub Executor | `src/server/hub/mod.rs` |

---

## Summary

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Chat System Summary                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. User sends message    ──▶  POST /api/chat                               │
│  2. Message queued        ──▶  mpsc channel to consumer                     │
│  3. Consumer processes    ──▶  ChatStrategy builds LLM request              │
│  4. LLM streams tokens    ──▶  Anthropic API with streaming                 │
│  5. Tokens buffered       ──▶  BufferedStream for late clients              │
│  6. Frontend receives     ──▶  SSE stream updates UI in real-time           │
│  7. Tools executed        ──▶  Mid-stream tool calls if needed              │
│  8. Response saved        ──▶  PostgreSQL for history                       │
│  9. Post-processing       ──▶  Auto-naming, compaction (background)         │
│ 10. Real-time updates     ──▶  WebSocket broadcasts session changes         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
