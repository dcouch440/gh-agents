# AI Design Assistant - Future Feature

## Vision

An intelligent AI assistant that helps users design and refine agent system prompts through natural conversation.

## Core Philosophy

### Personality & Communication
- **Natural & conversational** - Not robotic or corporate
- **Explains clearly** - Describes prompts in plain language
- **Context-aware** - Understands inputs/outputs and their implications
- **Collaborative** - Works WITH the user, not just executing commands

### Example Tone:
❌ "I have updated the system prompt per your specifications."
✅ "I've added those security checks! Now your agent will catch SQL injection and XSS issues right away."

## Key Capabilities

### 1. Prompt Understanding
- Analyzes current prompt critically
- Explains what the prompt does in plain language
- Identifies gaps or areas for improvement
- Suggests alternatives when appropriate

### 2. Document Integration
- **Read selected documents** - Can reference agent context documents
- **Create new documents** - Can generate short, focused documents
  - **Important:** Keep documents SHORT and specific
  - Examples: "Python Security Checklist", "Code Review Template"
- Uses document content to inform prompt suggestions

### 3. Surgical Editing
- Makes targeted, precise edits (not full rewrites)
- Explains what changed and why
- Can iterate based on feedback

### 4. Input/Output Awareness
- Understands example inputs user provides
- Can test prompt logic against examples
- Explains how the agent would respond to different scenarios

## Features to Build

### Mode Toggle
```
[Design Mode] [Test Mode]
```
- **Design Mode:** AI helps you write the prompt
- **Test Mode:** Chat with the agent you created

### Edit Operations
The AI can call tools to edit the prompt:

1. `append_to_prompt(text)` - Add to end
2. `insert_after(search, text)` - Insert after specific text
3. `replace_in_prompt(search, replace)` - Search and replace
4. `remove_from_prompt(search)` - Delete section

### Document Operations
5. `read_document(doc_id)` - Read content of selected document
6. `create_document(title, content, type)` - Create new short document
7. `suggest_documents()` - Suggest relevant documents to attach

### Analysis Operations
8. `analyze_prompt()` - Provide structured feedback on current prompt
9. `test_against_input(input)` - Explain how agent would handle input

## Example Conversations

### Scenario 1: Building from Scratch
```
👤 User: "I want to build a Python code reviewer"

🤖 AI: "Great! Let's start by defining what makes a good code
       reviewer. Should it focus on style, bugs, security, or
       all of the above?"

👤 User: "Mainly security"

🤖 AI: "Perfect. I'll create a foundation that emphasizes
       security reviews. Let me also pull from your 'Python
       Security Checklist' document..."

       [Creates initial prompt]
       [Reads selected document for context]

       "I've set up the base prompt with security focus. The
       agent will now check for SQL injection, XSS, and the
       other issues from your checklist. Want to add anything
       specific?"
```

### Scenario 2: Refining Existing
```
👤 User: "This prompt feels too generic"

🤖 AI: [Analyzes current prompt]

       "I see what you mean. Right now it just says 'review code'
       without much guidance. Let me make it more specific..."

       [Adds concrete examples and criteria]

       "Better? I added specific things to look for and example
       outputs. Now the agent knows exactly what a good review
       looks like."
```

### Scenario 3: Document Creation
```
👤 User: "The agent needs to know our API conventions"

🤖 AI: "I don't see a document about that yet. Want me to
       create one? I'll keep it short - just the key rules."

👤 User: "Yes, RESTful conventions"

🤖 AI: [Creates document]

       "Done! I created 'API Conventions' with RESTful best
       practices. I've also updated the prompt to reference
       this document. Your agent will now check code against
       these standards."
```

## Technical Implementation (Backend)

### Tool Definitions
```rust
Tool {
    name: "edit_prompt",
    parameters: {
        operation: "append|insert_after|replace|remove",
        search: "text to find (for insert/replace/remove)",
        text: "new content",
    }
}

Tool {
    name: "read_document",
    parameters: {
        document_id: "uuid",
    }
}

Tool {
    name: "create_document",
    parameters: {
        title: "string",
        content: "string (max 2000 chars)",
        doc_type: "reference|checklist|template",
    }
}
```

### Meta-Agent System Prompt
```
You are an expert AI prompt engineer with a friendly, collaborative personality.

CURRENT DRAFT:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
{editor_content}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

SELECTED DOCUMENTS:
{list of attached documents}

Your role:
- Help the user design an effective agent prompt
- Explain things clearly in natural language
- Make surgical edits, not rewrites
- Read and reference selected documents
- Create new SHORT documents when needed
- Be conversational and collaborative

Communication style:
- Natural, not robotic
- Explain the "why" behind suggestions
- Use examples to illustrate points
- Ask questions when unclear

Available tools:
- edit_prompt: Make precise changes
- read_document: Read selected documents
- create_document: Create short reference docs
- analyze_prompt: Provide structured feedback

Remember: You're a collaborative partner, not a corporate assistant.
```

## Frontend Changes (After Rebuild)

### Workshop UI with Material-UI
```
┌─────────────────────────────────────────────────────┐
│ Agent Workshop                                      │
│ ┌─────────────────┐                                 │
│ │ Design    Test  │ ← Material-UI Toggle Button     │
│ └─────────────────┘                                 │
├─────────────────────────────────────────────────────┤
│  Chat                      │  Editor                │
│  (Material-UI Card)        │  (Material-UI Paper)   │
│                            │                        │
│  Natural conversation      │  Live edits            │
│  with AI assistant         │  with highlights       │
└─────────────────────────────────────────────────────┘
```

## Open Questions

1. **Auto-save documents?** When AI creates a document, save immediately or ask first?
2. **Document length limit?** Hard cap at 2000 chars or soft warning?
3. **Edit history?** Track all AI edits for undo/redo?
4. **Proactive suggestions?** Should AI analyze on mode entry and suggest improvements?
5. **Testing integration?** Quick toggle between design and test, or separate flow?

## Why Not Building Yet

**Frontend rebuild planned** - Material-UI from ground up. Will revisit this feature after frontend is stable.

## Related Files
- Backend: `src/server/dag_executor/mod.rs` - Prompt composition
- Frontend: `frontend/src/pages/Agents/AgentWorkshopPage.tsx` - Workshop UI
- Documents: `src/server/api/documents/mod.rs` - Document management
