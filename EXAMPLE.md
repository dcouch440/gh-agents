Research the top 5 trending AI papers this week
completed
Configure this new workflow node. <user_text> Research the top 5 trending AI papers this week </user_text>
Configured a single specialized agent to research and rank the top 5 trending AI papers this week using web and X searches. No additional capabilities or dependencies needed as browsing is native.
▾ System Prompt
<role>
You are the workforce builder for "{{node_name}}", a node on a visual
workflow canvas. Users draw boxes with text, connect them with arrows,
and submit. You receive the user's canvas input and configure the team
of agents inside this node using your tools.
</role>

<board_state>
  <node ref="workforce-1" id="8fecd7b0-df94-4231-9d19-ccaf8f9c0bc6" status="idle">
    Not configured
    <node_text>Research the top 5 trending AI papers this week</node_text>
  </node>
</board_state>


<dispatch_status>
  <dispatch id="ba134c73" instruction="Configure this new workflow node.

&lt;user_text&gt;
Research the top 5 trending AI papers this week
&lt;/user_text&gt;" status="in_progress" started="0s ago" />
</dispatch_status>


<context>
Your configuration feeds into an agent designer that generates each
agent's runtime prompts. The designer reads the roster you configure
(names, roles, capabilities, dependencies) along with the node's board
text and upstream topology directly.

Available capabilities: file_read, file_write, content_search, shell,
document_read, database_query. All agents can browse the web and
search X/Twitter natively — this does not need to be assigned.

Every agent has implicit store_read_file and store_write_file — these
are the primary communication tools between agents. Do NOT assign them
as capabilities. store_write_file is always available. store_read_file
is available when upstream files exist. Only assign explicit capabilities
when the task requires project file access or specialized tools.

If an <upstream_topology> block is present in your instruction, use it to
understand what data flows into this node and what downstream expects.
When upstream already produces the core artifact, this node should consume
it — not recreate it.

The user may have drawn pen strokes on the canvas. You cannot see these
drawings — they are sent directly to the workforce agents as images at
runtime. Do not attempt to describe or interpret visual content. Focus
on team structure and agent roles. The agents will see the
image themselves.

A <prior_work> block in your instruction shows summaries of what you
previously configured. The board_state is the source of truth for
current configuration.

If the change affects the team configuration (task, agents, roles,
capabilities, or dependencies), call configure_team then complete_task.
If the change does NOT affect configuration (comments, compliments,
formatting, whitespace), skip configure_team and call complete_task
directly.
</context>

<guide>
Role descriptions: 1-2 sentences defining WHO the agent is — domain
expertise, scope boundary, and output type.

Example: "Security scanner who greps for vulnerability patterns and
confirms findings. Outputs a raw findings list with file paths, line
numbers, and vulnerability type."

Match team size to task complexity. A focused task needs 1 agent.
Add agents only when the work decomposes into distinct specialties
with different inputs and outputs. Most tasks are 1-agent tasks.

If a tool call fails, read the error, adjust, and retry.
</guide>

<examples>
<example name="simple_task">
<turn>
instruction: "Read the handwriting from the image."

<tool_call name="configure_team">
{"task": "Read handwriting from the image and transcribe it.",
 "agents": [
   {"name": "Reader", "role_description": "OCR specialist who reads handwritten text from images and produces a clean transcription."}
 ],
 "dependencies": []}
</tool_call>
<tool_call name="complete_task">
{"summary": "Configured single agent to read and transcribe handwriting from the image."}
</tool_call>
</turn>
</example>

<example name="multi_agent_pipeline">
<turn>
instruction: "Build a team to scan a codebase for security vulnerabilities and produce a remediation report with prioritized fixes."

<tool_call name="think">
{"thought": "Linear pipeline: Scanner finds issues, Analyzer prioritizes, Reporter writes the document. Scanner needs file access, Analyzer reads Scanner's output plus files for context, Reporter synthesizes. Dependencies: Scanner then Analyzer then Reporter."}
</tool_call>
<tool_call name="configure_team">
{"task": "Scan codebase for security vulnerabilities, prioritize findings by severity, and produce a remediation report with actionable fix recommendations.",
 "agents": [
   {"name": "Scanner", "role_description": "Security scanner who greps for vulnerability patterns and confirms findings. Outputs a raw findings list with file paths, line numbers, and vulnerability type.", "capabilities": ["file_read", "content_search"]},
   {"name": "Analyzer", "role_description": "Security analyst who verifies findings, assesses severity, and filters false positives. Outputs a prioritized vulnerability list.", "capabilities": ["file_read", "content_search"]},
   {"name": "Reporter", "role_description": "Technical writer who synthesizes prioritized findings into a remediation report with fix examples.", "capabilities": ["file_read"]}
 ],
 "dependencies": [
   {"from": "Scanner", "to": "Analyzer"},
   {"from": "Analyzer", "to": "Reporter"}
 ]}
</tool_call>
<tool_call name="complete_task">
{"summary": "Configured 3-agent pipeline: Scanner → Analyzer → Reporter for security vulnerability scanning with OWASP Top 10 focus."}
</tool_call>
</turn>
</example>

<example name="incremental_change">
<turn>
instruction: "Add a fact-checker after the researcher but before the writer. They should verify all claims the researcher makes."

<tool_call name="configure_team">
{"task": "Research competitors, verify claims, and write a summary report.",
 "agents": [
   {"name": "Researcher", "role_description": "Competitive intelligence analyst who finds pricing, positioning, and strategy data.", "capabilities": ["content_search"]},
   {"name": "FactChecker", "role_description": "Fact verification specialist who checks claims against authoritative sources. Outputs an annotated version with verification status.", "capabilities": []},
   {"name": "Writer", "role_description": "Report writer who synthesizes verified research into a structured summary.", "capabilities": []}
 ],
 "dependencies": [
   {"from": "Researcher", "to": "FactChecker"},
   {"from": "FactChecker", "to": "Writer"}
 ]}
</tool_call>
<tool_call name="complete_task">
{"summary": "Added FactChecker between Researcher and Writer. Pipeline is now Researcher → FactChecker → Writer."}
</tool_call>
</turn>
</example>
<example name="no_config_change">
<turn>
instruction: "The user updated this node on the canvas.

<change>
Before: \"Research competitor pricing.\"
After: \"Research competitor pricing.

Great job so far!\"
</change>"

<tool_call name="complete_task">
{"summary": "No configuration change needed — user added a comment, team and task unchanged."}
</tool_call>
</turn>
</example>
</examples>

<completion>
When done configuring, call complete_task with:

- **summary** — what you configured and key decisions (1-3 sentences).
- **question** — only if you cannot proceed without input. Make reasonable
  defaults rather than asking about preferences.
</completion>
▾ User Message
Configure this new workflow node.

<user_text>
Research the top 5 trending AI papers this week
</user_text>
configure team
agents=[{"name":"Trend Researcher","role_description":"AI trends specialist who identifies the top 5 trending AI papers from the past week by searching academic repositories like arXiv, Papers with Code, and social platforms like X/Twitter and Reddit for discussions, citations, and engagement. Outputs a ranked list with title, authors, publication date, link, key summary, and evidence of trending (e.g., tweet volume, upvotes)."}], task="Research and compile the top 5 trending AI research papers from the past week, ranking them by buzz and relevance, with full details and trend justification."
set node name
name="Top 5 AI Papers"
complete task
summary="Configured a single specialized agent to research and rank the top 5 trending AI papers this week using web and X searches. No additional capabilities or dependencies needed as browsing is native."
▾ System Prompt
<role>
You are the agent designer for "step:8fecd7b0-df94-4231-9d19-ccaf8f9c0bc6". You write
runtime prompts for each agent in the roster. Keep prompts short and direct.
Tell agents what to do, not how to think.
</role>

<board_state>
  <node ref="workforce-1" id="8fecd7b0-df94-4231-9d19-ccaf8f9c0bc6" status="configured" task="Research and compile the top 5 trending AI research papers from the past week, ranking them by buzz and relevance, with full details and trend justification.">
    1 agent, task set
    <node_text>Research the top 5 trending AI papers this week</node_text>
    <agent_roster>
      <agent name="Trend Researcher" id="936f5020-d6cf-43eb-8a38-2e99d949d876" design_status="pending">
        <role>AI trends specialist who identifies the top 5 trending AI papers from the past week by searching academic repositories like arXiv, Papers with Code, and social platforms like X/Twitter and Reddit for discussions, citations, and engagement. Outputs a ranked list with title, authors, publication date, link, key summary, and evidence of trending (e.g., tweet volume, upvotes).</role>
      </agent>
    </agent_roster>
  </node>
</board_state>


<tools>
write_file(path, content)
  Write agent config to design/agents/{slug}.json
  Content must be valid JSON with four fields:
  { "tools": [...], "system_prompt": "...", "assignment": "...", "expected_output": "..." }

read_file(path)
  Read a config from the store. Use this to verify existing
  configs are still consistent with the current board state.

complete_design(summary)
  Signal completion. Summary: what was written, what was verified,
  key decisions. No tools after this.
</tools>

<guidelines>
The board_state shows each agent's design_status:
- "pending" → write a new config
- "designed (vN)" → read the existing config, verify it matches the
  current node_text and upstream topology. Update if stale, skip if valid.

<dispatch_instruction> and <upstream_topology> in your user message
describe what triggered this run and what data flows in/out. These are
YOUR context — agents never see them at runtime.

At runtime, agents see:
- <context> block with the mission description
- <assignment> block (what you write)
- <previous_agent_outputs> (text from prior agents)
- <upstream_artifacts> (store files from prior steps)
- <upstream_step_outputs> (outputs from upstream DAG steps)
Never reference <node_text>, <dispatch_instruction>, or
<upstream_topology> in agent prompts — they don't exist at runtime.

Each config has four fields:
- tools: capabilities beyond baseline. store_read_file and
  store_write_file are always implicit — do not list them.
- system_prompt: who they are and what they do. Short and direct.
  No step-by-step cognitive processes. No numbered reasoning
  frameworks. Just: role, task, output format.
- assignment: the specific task. Reference <previous_agent_outputs>
  for upstream text, <upstream_artifacts> for store files.
- expected_output: 1-2 sentences describing the output shape.
  NOT a full schema. NOT example values. Just the shape.

Prompt proportionality — match size to task complexity:
- Pass-through (save, forward, rename): 30-60 tokens.
- Simple task (search, format, filter): 60-120 tokens.
- Complex task (analyze, compare, multi-step): 120-250 tokens.
Do not exceed 250 tokens for any system prompt.

Data flow:
- Substantial artifacts go to store via store_write_file
- Response text is lean — summary or key output only
- Downstream reads upstream via <previous_agent_outputs>

Web and X search: every agent can natively browse the web and
search X/Twitter. Do NOT reference any tool names for search —
just use natural language in the assignment: "Search the web
for..." When a task involves current data, say so in the
assignment, not as a multi-step process in the system prompt.
</guidelines>

<examples>
<example name="single_agent">
<turn>
Board state: 1 agent "Summarizer" (pending), no upstream.

<tool_call name="write_file">
{"path": "design/agents/summarizer.json",
 "content": "{\"tools\": [], \"system_prompt\": \"You summarize text into concise bullet points.\", \"assignment\": \"Read the input and produce a 5-bullet summary of key findings.\", \"expected_output\": \"Bulleted summary, 5 items max.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Wrote Summarizer config. Single agent, no upstream. Produces bullet summary."}
</tool_call>
</turn>
</example>

<example name="pipeline">
<turn>
Board state: 3 agents Scanner → Analyzer → Reporter (all pending), file_read + content_search capabilities.

<tool_call name="write_file">
{"path": "design/agents/scanner.json",
 "content": "{\"tools\": [\"file_read\", \"content_search\"], \"system_prompt\": \"Security scanner. Find vulnerability patterns in source code.\", \"assignment\": \"Grep the codebase for OWASP Top 10 patterns. Write raw findings to store via store_write_file.\", \"expected_output\": \"JSON array of findings with file path, line, type.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/analyzer.json",
 "content": "{\"tools\": [\"file_read\"], \"system_prompt\": \"Security analyst. Prioritize and verify findings.\", \"assignment\": \"Read <previous_agent_outputs> from Scanner. Verify each, rate severity, filter false positives.\", \"expected_output\": \"Prioritized vulnerability list with severity ratings.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/reporter.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Technical writer. Produce remediation reports.\", \"assignment\": \"Read <previous_agent_outputs> from Analyzer. Write remediation report with fix examples.\", \"expected_output\": \"Markdown report with sections per vulnerability.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "3-agent pipeline: Scanner → Analyzer → Reporter. Format chain: raw findings → prioritized list → markdown report."}
</tool_call>
</turn>
</example>

<example name="update_with_verify">
<turn>
Board state: 2 agents. Researcher (designed v2), Writer (pending).
User changed node text to add "include citations".

<tool_call name="read_file">
{"path": "design/agents/researcher.json"}
</tool_call>
(existing config says "find sources" — doesn't mention citations, stale)
<tool_call name="write_file">
{"path": "design/agents/researcher.json",
 "content": "{\"tools\": [\"content_search\"], \"system_prompt\": \"Research specialist. Find and cite sources.\", \"assignment\": \"Search the web for relevant sources. Include full citations. Write findings with citations to store.\", \"expected_output\": \"Research findings with source citations.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/writer.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Technical writer. Produce reports with citations.\", \"assignment\": \"Read <previous_agent_outputs> from Researcher. Write a report preserving all citations.\", \"expected_output\": \"Markdown report with inline citations.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Updated Researcher (stale — missing citations). Wrote Writer. Chain: cited research → cited report."}
</tool_call>
</turn>
</example>

<example name="verify_no_change">
<turn>
Board state: 2 agents. Scanner (designed v1), Reporter (pending).
Scanner was not changed by the builder.

<tool_call name="read_file">
{"path": "design/agents/scanner.json"}
</tool_call>
(config matches current node_text and topology — still valid)
<tool_call name="write_file">
{"path": "design/agents/reporter.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Report writer.\", \"assignment\": \"Read <previous_agent_outputs> from Scanner. Produce a summary report.\", \"expected_output\": \"Summary report of scan findings.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Verified Scanner (v1, still valid). Wrote Reporter. Chain: scan → report."}
</tool_call>
</turn>
</example>
</examples>
▾ User Message


<dispatch_instruction>
Configure this new workflow node.

<user_text>
Research the top 5 trending AI papers this week
</user_text>
</dispatch_instruction>

<upstream_topology>
Downstream nodes consuming this step's output:
  -> "Write a blog post combining the research and public sentiment": Write a blog post combining the research and public sentiment
</upstream_topology>

Review the board_state. For each agent:
- If design_status="pending", write a new config to design/agents/{slug}.json.
- If design_status="designed", read the existing config via read_file and
  verify it is consistent with the current node_text and upstream topology.
  Update if stale. Skip if correct.
Then call complete_design.
write file
content="{"tools": [], "system_prompt": "AI trends specialist. Identify top 5 trending AI research papers from the past week by buzz on social platforms and relevance.", "assignment": "Search arXiv recent submissions, Papers with Code trending, X/Twitter (recent posts on new AI papers), Reddit r/MachineLearning for discussions from the past 7 days. Rank top 5 by engagement (tweets, upvotes, citations). For each: title, authors, pub date, link, 2-3 sentence summary, trend evidence (e.g., tweet volume, top links). Output structured markdown ranked list 1-5.", "expected_output": "Markdown list ranked 1-5, each paper with details and justification."}", path="design/agents/trend-researcher.json"
complete design
summary="Wrote Trend Researcher config for single-agent node. Searches sources for top 5 trending AI papers past week, outputs ranked markdown with details and buzz evidence. Suitable for downstream blog post on research and sentiment."
Search X/Twitter for public reactions and hot takes on AI this week
completed
Configure this new workflow node. <user_text> Search X/Twitter for public reactions and hot takes on AI this week </user_text>
Configured single agent 'Searcher' to query X/Twitter for recent AI reactions/hot takes using native search tools, focusing on high-engagement posts from the past week. Outputs curated list ready for downstream use.
▾ System Prompt
<role>
You are the workforce builder for "{{node_name}}", a node on a visual
workflow canvas. Users draw boxes with text, connect them with arrows,
and submit. You receive the user's canvas input and configure the team
of agents inside this node using your tools.
</role>

<board_state>
  <node ref="workforce-2" id="9d57ed0a-bc9f-4d30-97a3-a015e1bd018c" status="idle">
    Not configured
    <node_text>Search X/Twitter for public reactions and hot takes on AI this week</node_text>
  </node>
</board_state>


<dispatch_status>
  <dispatch id="d854f324" instruction="Configure this new workflow node.

&lt;user_text&gt;
Search X/Twitter for public reactions and hot takes on AI this week
&lt;/use" status="in_progress" started="0s ago" />
</dispatch_status>


<context>
Your configuration feeds into an agent designer that generates each
agent's runtime prompts. The designer reads the roster you configure
(names, roles, capabilities, dependencies) along with the node's board
text and upstream topology directly.

Available capabilities: file_read, file_write, content_search, shell,
document_read, database_query. All agents can browse the web and
search X/Twitter natively — this does not need to be assigned.

Every agent has implicit store_read_file and store_write_file — these
are the primary communication tools between agents. Do NOT assign them
as capabilities. store_write_file is always available. store_read_file
is available when upstream files exist. Only assign explicit capabilities
when the task requires project file access or specialized tools.

If an <upstream_topology> block is present in your instruction, use it to
understand what data flows into this node and what downstream expects.
When upstream already produces the core artifact, this node should consume
it — not recreate it.

The user may have drawn pen strokes on the canvas. You cannot see these
drawings — they are sent directly to the workforce agents as images at
runtime. Do not attempt to describe or interpret visual content. Focus
on team structure and agent roles. The agents will see the
image themselves.

A <prior_work> block in your instruction shows summaries of what you
previously configured. The board_state is the source of truth for
current configuration.

If the change affects the team configuration (task, agents, roles,
capabilities, or dependencies), call configure_team then complete_task.
If the change does NOT affect configuration (comments, compliments,
formatting, whitespace), skip configure_team and call complete_task
directly.
</context>

<guide>
Role descriptions: 1-2 sentences defining WHO the agent is — domain
expertise, scope boundary, and output type.

Example: "Security scanner who greps for vulnerability patterns and
confirms findings. Outputs a raw findings list with file paths, line
numbers, and vulnerability type."

Match team size to task complexity. A focused task needs 1 agent.
Add agents only when the work decomposes into distinct specialties
with different inputs and outputs. Most tasks are 1-agent tasks.

If a tool call fails, read the error, adjust, and retry.
</guide>

<examples>
<example name="simple_task">
<turn>
instruction: "Read the handwriting from the image."

<tool_call name="configure_team">
{"task": "Read handwriting from the image and transcribe it.",
 "agents": [
   {"name": "Reader", "role_description": "OCR specialist who reads handwritten text from images and produces a clean transcription."}
 ],
 "dependencies": []}
</tool_call>
<tool_call name="complete_task">
{"summary": "Configured single agent to read and transcribe handwriting from the image."}
</tool_call>
</turn>
</example>

<example name="multi_agent_pipeline">
<turn>
instruction: "Build a team to scan a codebase for security vulnerabilities and produce a remediation report with prioritized fixes."

<tool_call name="think">
{"thought": "Linear pipeline: Scanner finds issues, Analyzer prioritizes, Reporter writes the document. Scanner needs file access, Analyzer reads Scanner's output plus files for context, Reporter synthesizes. Dependencies: Scanner then Analyzer then Reporter."}
</tool_call>
<tool_call name="configure_team">
{"task": "Scan codebase for security vulnerabilities, prioritize findings by severity, and produce a remediation report with actionable fix recommendations.",
 "agents": [
   {"name": "Scanner", "role_description": "Security scanner who greps for vulnerability patterns and confirms findings. Outputs a raw findings list with file paths, line numbers, and vulnerability type.", "capabilities": ["file_read", "content_search"]},
   {"name": "Analyzer", "role_description": "Security analyst who verifies findings, assesses severity, and filters false positives. Outputs a prioritized vulnerability list.", "capabilities": ["file_read", "content_search"]},
   {"name": "Reporter", "role_description": "Technical writer who synthesizes prioritized findings into a remediation report with fix examples.", "capabilities": ["file_read"]}
 ],
 "dependencies": [
   {"from": "Scanner", "to": "Analyzer"},
   {"from": "Analyzer", "to": "Reporter"}
 ]}
</tool_call>
<tool_call name="complete_task">
{"summary": "Configured 3-agent pipeline: Scanner → Analyzer → Reporter for security vulnerability scanning with OWASP Top 10 focus."}
</tool_call>
</turn>
</example>

<example name="incremental_change">
<turn>
instruction: "Add a fact-checker after the researcher but before the writer. They should verify all claims the researcher makes."

<tool_call name="configure_team">
{"task": "Research competitors, verify claims, and write a summary report.",
 "agents": [
   {"name": "Researcher", "role_description": "Competitive intelligence analyst who finds pricing, positioning, and strategy data.", "capabilities": ["content_search"]},
   {"name": "FactChecker", "role_description": "Fact verification specialist who checks claims against authoritative sources. Outputs an annotated version with verification status.", "capabilities": []},
   {"name": "Writer", "role_description": "Report writer who synthesizes verified research into a structured summary.", "capabilities": []}
 ],
 "dependencies": [
   {"from": "Researcher", "to": "FactChecker"},
   {"from": "FactChecker", "to": "Writer"}
 ]}
</tool_call>
<tool_call name="complete_task">
{"summary": "Added FactChecker between Researcher and Writer. Pipeline is now Researcher → FactChecker → Writer."}
</tool_call>
</turn>
</example>
<example name="no_config_change">
<turn>
instruction: "The user updated this node on the canvas.

<change>
Before: \"Research competitor pricing.\"
After: \"Research competitor pricing.

Great job so far!\"
</change>"

<tool_call name="complete_task">
{"summary": "No configuration change needed — user added a comment, team and task unchanged."}
</tool_call>
</turn>
</example>
</examples>

<completion>
When done configuring, call complete_task with:

- **summary** — what you configured and key decisions (1-3 sentences).
- **question** — only if you cannot proceed without input. Make reasonable
  defaults rather than asking about preferences.
</completion>
▾ User Message
Configure this new workflow node.

<user_text>
Search X/Twitter for public reactions and hot takes on AI this week
</user_text>
set node name
name="AI X Reactions"
configure team
agents=[{"name":"Searcher","role_description":"Social media scout who semantically and keyword searches X/Twitter for public reactions and hot takes on AI from the past week. Curates the top 10-20 most viral, opinionated, or representative posts with engagement metrics, quotes, and links."}], task="Search X/Twitter for public reactions and hot takes on AI this week, curating a list of notable posts with summaries and links."
complete task
summary="Configured single agent 'Searcher' to query X/Twitter for recent AI reactions/hot takes using native search tools, focusing on high-engagement posts from the past week. Outputs curated list ready for downstream use."
▾ System Prompt
<role>
You are the agent designer for "step:9d57ed0a-bc9f-4d30-97a3-a015e1bd018c". You write
runtime prompts for each agent in the roster. Keep prompts short and direct.
Tell agents what to do, not how to think.
</role>

<board_state>
  <node ref="workforce-2" id="9d57ed0a-bc9f-4d30-97a3-a015e1bd018c" status="configured" task="Search X/Twitter for public reactions and hot takes on AI this week, curating a list of notable posts with summaries and links.">
    1 agent, task set
    <node_text>Search X/Twitter for public reactions and hot takes on AI this week</node_text>
    <agent_roster>
      <agent name="Searcher" id="7b1be561-37dc-4ac1-a339-7570467e1ccb" design_status="pending">
        <role>Social media scout who semantically and keyword searches X/Twitter for public reactions and hot takes on AI from the past week. Curates the top 10-20 most viral, opinionated, or representative posts with engagement metrics, quotes, and links.</role>
      </agent>
    </agent_roster>
  </node>
</board_state>


<tools>
write_file(path, content)
  Write agent config to design/agents/{slug}.json
  Content must be valid JSON with four fields:
  { "tools": [...], "system_prompt": "...", "assignment": "...", "expected_output": "..." }

read_file(path)
  Read a config from the store. Use this to verify existing
  configs are still consistent with the current board state.

complete_design(summary)
  Signal completion. Summary: what was written, what was verified,
  key decisions. No tools after this.
</tools>

<guidelines>
The board_state shows each agent's design_status:
- "pending" → write a new config
- "designed (vN)" → read the existing config, verify it matches the
  current node_text and upstream topology. Update if stale, skip if valid.

<dispatch_instruction> and <upstream_topology> in your user message
describe what triggered this run and what data flows in/out. These are
YOUR context — agents never see them at runtime.

At runtime, agents see:
- <context> block with the mission description
- <assignment> block (what you write)
- <previous_agent_outputs> (text from prior agents)
- <upstream_artifacts> (store files from prior steps)
- <upstream_step_outputs> (outputs from upstream DAG steps)
Never reference <node_text>, <dispatch_instruction>, or
<upstream_topology> in agent prompts — they don't exist at runtime.

Each config has four fields:
- tools: capabilities beyond baseline. store_read_file and
  store_write_file are always implicit — do not list them.
- system_prompt: who they are and what they do. Short and direct.
  No step-by-step cognitive processes. No numbered reasoning
  frameworks. Just: role, task, output format.
- assignment: the specific task. Reference <previous_agent_outputs>
  for upstream text, <upstream_artifacts> for store files.
- expected_output: 1-2 sentences describing the output shape.
  NOT a full schema. NOT example values. Just the shape.

Prompt proportionality — match size to task complexity:
- Pass-through (save, forward, rename): 30-60 tokens.
- Simple task (search, format, filter): 60-120 tokens.
- Complex task (analyze, compare, multi-step): 120-250 tokens.
Do not exceed 250 tokens for any system prompt.

Data flow:
- Substantial artifacts go to store via store_write_file
- Response text is lean — summary or key output only
- Downstream reads upstream via <previous_agent_outputs>

Web and X search: every agent can natively browse the web and
search X/Twitter. Do NOT reference any tool names for search —
just use natural language in the assignment: "Search the web
for..." When a task involves current data, say so in the
assignment, not as a multi-step process in the system prompt.
</guidelines>

<examples>
<example name="single_agent">
<turn>
Board state: 1 agent "Summarizer" (pending), no upstream.

<tool_call name="write_file">
{"path": "design/agents/summarizer.json",
 "content": "{\"tools\": [], \"system_prompt\": \"You summarize text into concise bullet points.\", \"assignment\": \"Read the input and produce a 5-bullet summary of key findings.\", \"expected_output\": \"Bulleted summary, 5 items max.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Wrote Summarizer config. Single agent, no upstream. Produces bullet summary."}
</tool_call>
</turn>
</example>

<example name="pipeline">
<turn>
Board state: 3 agents Scanner → Analyzer → Reporter (all pending), file_read + content_search capabilities.

<tool_call name="write_file">
{"path": "design/agents/scanner.json",
 "content": "{\"tools\": [\"file_read\", \"content_search\"], \"system_prompt\": \"Security scanner. Find vulnerability patterns in source code.\", \"assignment\": \"Grep the codebase for OWASP Top 10 patterns. Write raw findings to store via store_write_file.\", \"expected_output\": \"JSON array of findings with file path, line, type.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/analyzer.json",
 "content": "{\"tools\": [\"file_read\"], \"system_prompt\": \"Security analyst. Prioritize and verify findings.\", \"assignment\": \"Read <previous_agent_outputs> from Scanner. Verify each, rate severity, filter false positives.\", \"expected_output\": \"Prioritized vulnerability list with severity ratings.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/reporter.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Technical writer. Produce remediation reports.\", \"assignment\": \"Read <previous_agent_outputs> from Analyzer. Write remediation report with fix examples.\", \"expected_output\": \"Markdown report with sections per vulnerability.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "3-agent pipeline: Scanner → Analyzer → Reporter. Format chain: raw findings → prioritized list → markdown report."}
</tool_call>
</turn>
</example>

<example name="update_with_verify">
<turn>
Board state: 2 agents. Researcher (designed v2), Writer (pending).
User changed node text to add "include citations".

<tool_call name="read_file">
{"path": "design/agents/researcher.json"}
</tool_call>
(existing config says "find sources" — doesn't mention citations, stale)
<tool_call name="write_file">
{"path": "design/agents/researcher.json",
 "content": "{\"tools\": [\"content_search\"], \"system_prompt\": \"Research specialist. Find and cite sources.\", \"assignment\": \"Search the web for relevant sources. Include full citations. Write findings with citations to store.\", \"expected_output\": \"Research findings with source citations.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/writer.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Technical writer. Produce reports with citations.\", \"assignment\": \"Read <previous_agent_outputs> from Researcher. Write a report preserving all citations.\", \"expected_output\": \"Markdown report with inline citations.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Updated Researcher (stale — missing citations). Wrote Writer. Chain: cited research → cited report."}
</tool_call>
</turn>
</example>

<example name="verify_no_change">
<turn>
Board state: 2 agents. Scanner (designed v1), Reporter (pending).
Scanner was not changed by the builder.

<tool_call name="read_file">
{"path": "design/agents/scanner.json"}
</tool_call>
(config matches current node_text and topology — still valid)
<tool_call name="write_file">
{"path": "design/agents/reporter.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Report writer.\", \"assignment\": \"Read <previous_agent_outputs> from Scanner. Produce a summary report.\", \"expected_output\": \"Summary report of scan findings.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Verified Scanner (v1, still valid). Wrote Reporter. Chain: scan → report."}
</tool_call>
</turn>
</example>
</examples>
▾ User Message


<dispatch_instruction>
Configure this new workflow node.

<user_text>
Search X/Twitter for public reactions and hot takes on AI this week
</user_text>
</dispatch_instruction>

<upstream_topology>
Downstream nodes consuming this step's output:
  -> "Write a blog post combining the research and public sentiment": Write a blog post combining the research and public sentiment
</upstream_topology>

Review the board_state. For each agent:
- If design_status="pending", write a new config to design/agents/{slug}.json.
- If design_status="designed", read the existing config via read_file and
  verify it is consistent with the current node_text and upstream topology.
  Update if stale. Skip if correct.
Then call complete_design.
write file
content="{"tools": [], "system_prompt": "Social media scout. Search X/Twitter for public reactions and hot takes on AI from the past week. Curate top 10-20 viral, opinionated posts with summaries, quotes, metrics, and links.", "assignment": "Search X/Twitter this week for AI reactions and hot takes using semantic/keyword searches. Curate 10-20 notable posts: summary, quote, engagement (likes/RTs/views), author, link. Write JSON list to store_write_file(path='ai_x_reactions.json'). Respond with top 5 bullets incl. links.", "expected_output": "JSON artifact of curated posts; response: 5 bullet summaries with links."}", path="design/agents/searcher.json"
complete design
summary="Wrote Searcher config for single-agent node. Curates 10-20 X/Twitter posts on recent AI reactions into JSON artifact (for downstream blog post) and top-5 summary bullets. Matches node_text; no upstream."
AI Trends Blog
completed
Configure this new workflow node. <user_text> Write a blog post combining the research and public sentiment </user_text>
Configured single Writer agent to consume upstream research and sentiment files, synthesizing them into a cohesive AI trends blog post. Set display name to 'AI Trends Blog' and added description for workflow clarity.
▾ System Prompt
<role>
You are the workforce builder for "{{node_name}}", a node on a visual
workflow canvas. Users draw boxes with text, connect them with arrows,
and submit. You receive the user's canvas input and configure the team
of agents inside this node using your tools.
</role>

<board_state>
  <node ref="workforce-3" id="1c6eb818-68ea-4d69-b4cc-7687f9685d6d" status="idle" receives="Search X/Twitter for public reactions and hot takes on AI this week, Research the top 5 trending AI papers this week">
    Not configured
    <node_text>Write a blog post combining the research and public sentiment</node_text>
  </node>
</board_state>


<dispatch_status>
  <dispatch id="11c3c89e" instruction="Configure this new workflow node.

&lt;user_text&gt;
Write a blog post combining the research and public sentiment
&lt;/user_text" status="in_progress" started="0s ago" />
</dispatch_status>


<context>
Your configuration feeds into an agent designer that generates each
agent's runtime prompts. The designer reads the roster you configure
(names, roles, capabilities, dependencies) along with the node's board
text and upstream topology directly.

Available capabilities: file_read, file_write, content_search, shell,
document_read, database_query. All agents can browse the web and
search X/Twitter natively — this does not need to be assigned.

Every agent has implicit store_read_file and store_write_file — these
are the primary communication tools between agents. Do NOT assign them
as capabilities. store_write_file is always available. store_read_file
is available when upstream files exist. Only assign explicit capabilities
when the task requires project file access or specialized tools.

If an <upstream_topology> block is present in your instruction, use it to
understand what data flows into this node and what downstream expects.
When upstream already produces the core artifact, this node should consume
it — not recreate it.

The user may have drawn pen strokes on the canvas. You cannot see these
drawings — they are sent directly to the workforce agents as images at
runtime. Do not attempt to describe or interpret visual content. Focus
on team structure and agent roles. The agents will see the
image themselves.

A <prior_work> block in your instruction shows summaries of what you
previously configured. The board_state is the source of truth for
current configuration.

If the change affects the team configuration (task, agents, roles,
capabilities, or dependencies), call configure_team then complete_task.
If the change does NOT affect configuration (comments, compliments,
formatting, whitespace), skip configure_team and call complete_task
directly.
</context>

<guide>
Role descriptions: 1-2 sentences defining WHO the agent is — domain
expertise, scope boundary, and output type.

Example: "Security scanner who greps for vulnerability patterns and
confirms findings. Outputs a raw findings list with file paths, line
numbers, and vulnerability type."

Match team size to task complexity. A focused task needs 1 agent.
Add agents only when the work decomposes into distinct specialties
with different inputs and outputs. Most tasks are 1-agent tasks.

If a tool call fails, read the error, adjust, and retry.
</guide>

<examples>
<example name="simple_task">
<turn>
instruction: "Read the handwriting from the image."

<tool_call name="configure_team">
{"task": "Read handwriting from the image and transcribe it.",
 "agents": [
   {"name": "Reader", "role_description": "OCR specialist who reads handwritten text from images and produces a clean transcription."}
 ],
 "dependencies": []}
</tool_call>
<tool_call name="complete_task">
{"summary": "Configured single agent to read and transcribe handwriting from the image."}
</tool_call>
</turn>
</example>

<example name="multi_agent_pipeline">
<turn>
instruction: "Build a team to scan a codebase for security vulnerabilities and produce a remediation report with prioritized fixes."

<tool_call name="think">
{"thought": "Linear pipeline: Scanner finds issues, Analyzer prioritizes, Reporter writes the document. Scanner needs file access, Analyzer reads Scanner's output plus files for context, Reporter synthesizes. Dependencies: Scanner then Analyzer then Reporter."}
</tool_call>
<tool_call name="configure_team">
{"task": "Scan codebase for security vulnerabilities, prioritize findings by severity, and produce a remediation report with actionable fix recommendations.",
 "agents": [
   {"name": "Scanner", "role_description": "Security scanner who greps for vulnerability patterns and confirms findings. Outputs a raw findings list with file paths, line numbers, and vulnerability type.", "capabilities": ["file_read", "content_search"]},
   {"name": "Analyzer", "role_description": "Security analyst who verifies findings, assesses severity, and filters false positives. Outputs a prioritized vulnerability list.", "capabilities": ["file_read", "content_search"]},
   {"name": "Reporter", "role_description": "Technical writer who synthesizes prioritized findings into a remediation report with fix examples.", "capabilities": ["file_read"]}
 ],
 "dependencies": [
   {"from": "Scanner", "to": "Analyzer"},
   {"from": "Analyzer", "to": "Reporter"}
 ]}
</tool_call>
<tool_call name="complete_task">
{"summary": "Configured 3-agent pipeline: Scanner → Analyzer → Reporter for security vulnerability scanning with OWASP Top 10 focus."}
</tool_call>
</turn>
</example>

<example name="incremental_change">
<turn>
instruction: "Add a fact-checker after the researcher but before the writer. They should verify all claims the researcher makes."

<tool_call name="configure_team">
{"task": "Research competitors, verify claims, and write a summary report.",
 "agents": [
   {"name": "Researcher", "role_description": "Competitive intelligence analyst who finds pricing, positioning, and strategy data.", "capabilities": ["content_search"]},
   {"name": "FactChecker", "role_description": "Fact verification specialist who checks claims against authoritative sources. Outputs an annotated version with verification status.", "capabilities": []},
   {"name": "Writer", "role_description": "Report writer who synthesizes verified research into a structured summary.", "capabilities": []}
 ],
 "dependencies": [
   {"from": "Researcher", "to": "FactChecker"},
   {"from": "FactChecker", "to": "Writer"}
 ]}
</tool_call>
<tool_call name="complete_task">
{"summary": "Added FactChecker between Researcher and Writer. Pipeline is now Researcher → FactChecker → Writer."}
</tool_call>
</turn>
</example>
<example name="no_config_change">
<turn>
instruction: "The user updated this node on the canvas.

<change>
Before: \"Research competitor pricing.\"
After: \"Research competitor pricing.

Great job so far!\"
</change>"

<tool_call name="complete_task">
{"summary": "No configuration change needed — user added a comment, team and task unchanged."}
</tool_call>
</turn>
</example>
</examples>

<completion>
When done configuring, call complete_task with:

- **summary** — what you configured and key decisions (1-3 sentences).
- **question** — only if you cannot proceed without input. Make reasonable
  defaults rather than asking about preferences.
</completion>
▾ User Message
Configure this new workflow node.

<user_text>
Write a blog post combining the research and public sentiment
</user_text>
set node name
name="AI Trends Blog"
set node description
description="Consumes upstream AI research papers and X/Twitter sentiment data to produce an engaging blog post synthesizing key insights and public reactions."
configure team
agents=[{"name":"Writer","role_description":"Blog post author who reads upstream files on top trending AI papers and X/Twitter reactions, then combines them into an engaging, informative post highlighting trends, breakthroughs, public buzz, and analysis."}], task="Write a blog post synthesizing the top 5 trending AI papers this week with public reactions and hot takes from X/Twitter, creating an engaging narrative that connects academic advances to real-world sentiment."
complete task
summary="Configured single Writer agent to consume upstream research and sentiment files, synthesizing them into a cohesive AI trends blog post. Set display name to 'AI Trends Blog' and added description for workflow clarity."
▾ System Prompt
<role>
You are the agent designer for "step:1c6eb818-68ea-4d69-b4cc-7687f9685d6d". You write
runtime prompts for each agent in the roster. Keep prompts short and direct.
Tell agents what to do, not how to think.
</role>

<board_state>
  <node ref="workforce-3" id="1c6eb818-68ea-4d69-b4cc-7687f9685d6d" status="configured" task="Write a blog post synthesizing the top 5 trending AI papers this week with public reactions and hot takes from X/Twitter, creating an engaging narrative that connects academic advances to real-world sentiment." receives="AI X Reactions, Top 5 AI Papers">
    1 agent, task set
    <node_text>Write a blog post combining the research and public sentiment</node_text>
    <agent_roster>
      <agent name="Writer" id="ba4d57d3-19a4-42eb-bc17-3ca8877e9d8e" design_status="pending">
        <role>Blog post author who reads upstream files on top trending AI papers and X/Twitter reactions, then combines them into an engaging, informative post highlighting trends, breakthroughs, public buzz, and analysis.</role>
      </agent>
    </agent_roster>
  </node>
</board_state>


<tools>
write_file(path, content)
  Write agent config to design/agents/{slug}.json
  Content must be valid JSON with four fields:
  { "tools": [...], "system_prompt": "...", "assignment": "...", "expected_output": "..." }

read_file(path)
  Read a config from the store. Use this to verify existing
  configs are still consistent with the current board state.

complete_design(summary)
  Signal completion. Summary: what was written, what was verified,
  key decisions. No tools after this.
</tools>

<guidelines>
The board_state shows each agent's design_status:
- "pending" → write a new config
- "designed (vN)" → read the existing config, verify it matches the
  current node_text and upstream topology. Update if stale, skip if valid.

<dispatch_instruction> and <upstream_topology> in your user message
describe what triggered this run and what data flows in/out. These are
YOUR context — agents never see them at runtime.

At runtime, agents see:
- <context> block with the mission description
- <assignment> block (what you write)
- <previous_agent_outputs> (text from prior agents)
- <upstream_artifacts> (store files from prior steps)
- <upstream_step_outputs> (outputs from upstream DAG steps)
Never reference <node_text>, <dispatch_instruction>, or
<upstream_topology> in agent prompts — they don't exist at runtime.

Each config has four fields:
- tools: capabilities beyond baseline. store_read_file and
  store_write_file are always implicit — do not list them.
- system_prompt: who they are and what they do. Short and direct.
  No step-by-step cognitive processes. No numbered reasoning
  frameworks. Just: role, task, output format.
- assignment: the specific task. Reference <previous_agent_outputs>
  for upstream text, <upstream_artifacts> for store files.
- expected_output: 1-2 sentences describing the output shape.
  NOT a full schema. NOT example values. Just the shape.

Prompt proportionality — match size to task complexity:
- Pass-through (save, forward, rename): 30-60 tokens.
- Simple task (search, format, filter): 60-120 tokens.
- Complex task (analyze, compare, multi-step): 120-250 tokens.
Do not exceed 250 tokens for any system prompt.

Data flow:
- Substantial artifacts go to store via store_write_file
- Response text is lean — summary or key output only
- Downstream reads upstream via <previous_agent_outputs>

Web and X search: every agent can natively browse the web and
search X/Twitter. Do NOT reference any tool names for search —
just use natural language in the assignment: "Search the web
for..." When a task involves current data, say so in the
assignment, not as a multi-step process in the system prompt.
</guidelines>

<examples>
<example name="single_agent">
<turn>
Board state: 1 agent "Summarizer" (pending), no upstream.

<tool_call name="write_file">
{"path": "design/agents/summarizer.json",
 "content": "{\"tools\": [], \"system_prompt\": \"You summarize text into concise bullet points.\", \"assignment\": \"Read the input and produce a 5-bullet summary of key findings.\", \"expected_output\": \"Bulleted summary, 5 items max.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Wrote Summarizer config. Single agent, no upstream. Produces bullet summary."}
</tool_call>
</turn>
</example>

<example name="pipeline">
<turn>
Board state: 3 agents Scanner → Analyzer → Reporter (all pending), file_read + content_search capabilities.

<tool_call name="write_file">
{"path": "design/agents/scanner.json",
 "content": "{\"tools\": [\"file_read\", \"content_search\"], \"system_prompt\": \"Security scanner. Find vulnerability patterns in source code.\", \"assignment\": \"Grep the codebase for OWASP Top 10 patterns. Write raw findings to store via store_write_file.\", \"expected_output\": \"JSON array of findings with file path, line, type.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/analyzer.json",
 "content": "{\"tools\": [\"file_read\"], \"system_prompt\": \"Security analyst. Prioritize and verify findings.\", \"assignment\": \"Read <previous_agent_outputs> from Scanner. Verify each, rate severity, filter false positives.\", \"expected_output\": \"Prioritized vulnerability list with severity ratings.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/reporter.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Technical writer. Produce remediation reports.\", \"assignment\": \"Read <previous_agent_outputs> from Analyzer. Write remediation report with fix examples.\", \"expected_output\": \"Markdown report with sections per vulnerability.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "3-agent pipeline: Scanner → Analyzer → Reporter. Format chain: raw findings → prioritized list → markdown report."}
</tool_call>
</turn>
</example>

<example name="update_with_verify">
<turn>
Board state: 2 agents. Researcher (designed v2), Writer (pending).
User changed node text to add "include citations".

<tool_call name="read_file">
{"path": "design/agents/researcher.json"}
</tool_call>
(existing config says "find sources" — doesn't mention citations, stale)
<tool_call name="write_file">
{"path": "design/agents/researcher.json",
 "content": "{\"tools\": [\"content_search\"], \"system_prompt\": \"Research specialist. Find and cite sources.\", \"assignment\": \"Search the web for relevant sources. Include full citations. Write findings with citations to store.\", \"expected_output\": \"Research findings with source citations.\"}"}
</tool_call>
<tool_call name="write_file">
{"path": "design/agents/writer.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Technical writer. Produce reports with citations.\", \"assignment\": \"Read <previous_agent_outputs> from Researcher. Write a report preserving all citations.\", \"expected_output\": \"Markdown report with inline citations.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Updated Researcher (stale — missing citations). Wrote Writer. Chain: cited research → cited report."}
</tool_call>
</turn>
</example>

<example name="verify_no_change">
<turn>
Board state: 2 agents. Scanner (designed v1), Reporter (pending).
Scanner was not changed by the builder.

<tool_call name="read_file">
{"path": "design/agents/scanner.json"}
</tool_call>
(config matches current node_text and topology — still valid)
<tool_call name="write_file">
{"path": "design/agents/reporter.json",
 "content": "{\"tools\": [], \"system_prompt\": \"Report writer.\", \"assignment\": \"Read <previous_agent_outputs> from Scanner. Produce a summary report.\", \"expected_output\": \"Summary report of scan findings.\"}"}
</tool_call>
<tool_call name="complete_design">
{"summary": "Verified Scanner (v1, still valid). Wrote Reporter. Chain: scan → report."}
</tool_call>
</turn>
</example>
</examples>
▾ User Message


<dispatch_instruction>
Configure this new workflow node.

<user_text>
Write a blog post combining the research and public sentiment
</user_text>
</dispatch_instruction>

<upstream_topology>
Upstream nodes feeding into this step:
  <- "AI X Reactions": Search X/Twitter for public reactions and hot takes on AI this week
  <- "Top 5 AI Papers": Research the top 5 trending AI papers this week
</upstream_topology>

Review the board_state. For each agent:
- If design_status="pending", write a new config to design/agents/{slug}.json.
- If design_status="designed", read the existing config via read_file and
  verify it is consistent with the current node_text and upstream topology.
  Update if stale. Skip if correct.
Then call complete_design.
write file
content="{"tools": [], "system_prompt": "Blog post author who combines top trending AI papers with X/Twitter reactions into an engaging, informative post highlighting trends, breakthroughs, public buzz, and analysis.", "assignment": "Read &lt;previous_agent_outputs&gt; from 'Top 5 AI Papers' and 'AI X Reactions'. Synthesize into a blog post: intro on weekly AI trends, detail each of top 5 papers with key advances, weave in relevant public reactions/hot takes from X, conclude with analysis connecting academia to sentiment. Use markdown formatting, engaging narrative voice.", "expected_output": "Markdown blog post (800-1500 words) with headings, citations from upstream where applicable."}", path="design/agents/writer.json"
complete design
summary="Wrote Writer agent config (pending). Single agent reads upstream Top 5 AI Papers and AI X Reactions outputs, synthesizes into engaging markdown blog post connecting research to public sentiment. No upstream verification needed."