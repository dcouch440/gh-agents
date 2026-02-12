## PRD: Real-Time Collaboration for Miro-Style Whiteboard App

### Problem Statement

CanvasBoard is a browser-based whiteboard tool for design teams. Currently it's single-player — one user edits a board at a time. When multiple designers need to brainstorm, they export PNGs and paste them into Slack. Two large agency clients have churned specifically citing the lack of real-time multiplayer as the reason. Competitor Whimsical shipped live cursors last quarter and our trial-to-paid conversion dropped 18%.

### Goals

1. **Live cursors** — see other users' cursor positions and selections on the board in real-time
2. **Concurrent editing** — multiple users can add, move, and resize shapes simultaneously without conflicts
3. **Presence indicators** — see who's on the board and which element they have selected
4. **Change attribution** — every shape edit shows who made it and when (visible on hover)
5. **Conflict resolution** — when two users edit the same shape property simultaneously, last-write-wins with a visible "overwritten" toast to the first user

### Non-Goals

- Voice/video chat (use Zoom/Meet integrations)
- Commenting or annotation threads on shapes (separate Q3 feature)
- Offline editing with sync (always-online assumption for v1)
- Per-shape permissions or locking (all collaborators have full edit access)
- Version history / time-travel (existing snapshot system is sufficient)

### User Stories

**As a lead designer**, I want to see my teammates' cursors on the board so I know which section they're working on during a brainstorm.

**As a design manager**, I want to see who modified a component so I can review changes during our weekly design review.

**As a new team member**, I want to see presence indicators so I don't accidentally edit a frame someone else is actively working in.

**As a freelance contractor**, I want to join a shared board with just a link, see who else is there, and start contributing immediately without setup.

### Technical Constraints

- Frontend is React 18 + Konva.js for canvas rendering. Cursor broadcasting needs to integrate with Konva's stage coordinate system and viewport transforms (zoom, pan).
- Backend is Go with PostgreSQL 15. WebSocket support exists for notification delivery but has not been used for high-frequency state sync.
- Current shape save is optimistic with debounced PUT to REST API. Concurrent edits will need either CRDTs, operational transforms, or a simpler server-reconciled approach for MVP.
- Database: `shapes` table has no `updated_by` or `updated_at` columns. Migration required, and the table has 45M rows across all boards.
- Existing auth is session-based with 30-day cookies. Guest access (link sharing) will need a lightweight token scheme.
- Target: 25 concurrent users per board without degradation. P99 cursor broadcast latency < 80ms. Shape sync latency < 200ms.

### Architecture Considerations

- **WebSocket vs WebTransport** — WebSocket is proven and well-supported. WebTransport offers lower latency but limited browser support. Recommend WS for MVP.
- **State model** — Full CRDT (e.g., Yjs, Automerge) gives offline-first and automatic conflict resolution but adds complexity. Server-reconciled last-write-wins is simpler for always-online v1.
- **Broadcast topology** — Fan-out from server (star) vs peer-to-peer mesh. Star is simpler, gives server authority for conflict resolution, and works behind corporate firewalls. Mesh is lower latency but harder to debug.
- **Cursor throttling** — Raw mouse events at 60fps are too chatty. Throttle to ~15fps client-side, interpolate on receiving clients for smooth rendering.

### Success Metrics

- Recover 2 churned agency accounts within 60 days of launch
- Trial-to-paid conversion back to pre-competitor baseline (+18% relative)
- Collaboration session adoption > 30% of active teams within 6 weeks
- No measurable increase in canvas rendering latency (< 3ms P95 delta)
- Zero data loss incidents from concurrent editing in first 90 days

### Open Questions

1. Should we implement Yjs/CRDT or go with server-reconciled last-write-wins for MVP?
2. Do we need per-user undo/redo stacks or a shared history?
3. Should presence persist across page reloads or only show active browser tabs?
4. What's the guest access model — anonymous with display name, or require email?
5. Do we throttle shape property sync the same as cursor sync, or use different rates?
