# Workspace Filesystem Infrastructure — Research

Research into shared filesystem solutions for AI agent containers with real POSIX access, covering single-workflow workspace sharing and cross-workflow composition.

## The Problem

AI agents run in containers. They need a real filesystem — `pip install`, `python main.py`, `ls`, `cat` all need to work. Steps in a workflow share a workspace where files accumulate. Sequential steps must see previous step's files. Parallel steps may overlap. The workspace may contain code, images, data (mixed, up to several GB).

Current system store is S3-backed with Postgres metadata. Files are text blobs accessed via `store_read_file` / `store_write_file` tools — not a real filesystem. Agents can't execute programs they produce.

---

## Part 1: POSIX Filesystems Over Object Storage

### JuiceFS — Recommended

**What it is:** POSIX-compatible filesystem with metadata in Postgres/Redis and data in S3. Clients mount via FUSE. Full random read/write support.

**Consistency:** Close-to-open. Once a file is written and closed, all other clients see the update on next open. Same model as NFS — strong enough for pipeline workflows.

**Multi-writer:** Fully supported. Thousands of clients can mount simultaneously. Kubernetes CSI driver handles multi-pod mounts natively.

**Performance:**
- Redis metadata: sub-millisecond metadata ops, 2-4x faster than SQL engines
- Postgres metadata: 2-4x slower than Redis, ~13x slower for small file I/O (~100KB)
- Data throughput: scales with S3 bandwidth, 1.2 TB/s aggregate demonstrated
- Local SSD cache (`--cache-dir`) dramatically reduces repeat-read latency
- `--writeback` mode absorbs small-file burst during package installation

**Can agents run dev tools?** Yes with Redis metadata + local SSD cache. `pip install` creates thousands of small files — each inode operation roundtrips to metadata engine. With Redis, this is sluggish but functional. With Postgres, noticeably slow. Recommendation: use `--writeback` mode and local SSD cache, or install packages locally then copy to JuiceFS.

**Production readiness:** Heavily deployed for AI training workloads. 265 issues resolved in 2024, 305 issues and 601 merged PRs in 2025. Apache-2.0.

**Limitation:** Postgres metadata is single-instance ceiling. Fine for our use case (agent containers sharing a workspace, not thousands of concurrent writers).

### LakeFS — Compelling for Branching, Not for Execution

**What it is:** Git-like version control layer over S3. Branches, commits, merges, diffs — but for object keys, not POSIX inodes.

**Merge mechanics:** Three-way merge using nearest common ancestor. File-level (not line-level). Conflict strategies: `source-wins`, `dest-wins`. Per-file resolution on roadmap.

**FUSE mount (Everest):** Available but limited. No POSIX locks, no chmod/chown, no concurrent write semantics. Write mode uploads to temporary branch, then commits.

**Can agents run dev tools?** No. The mount lacks POSIX locks, permission support, and concurrent write semantics needed for `pip install` or general development toolchains.

**Verdict:** Architecturally compelling for "branch per agent, merge results" pattern. But not a general-purpose POSIX filesystem. Best used as a versioning layer ABOVE the working filesystem, not as the filesystem itself.

**Hybrid approach:** JuiceFS as working filesystem (agents install, execute, write) + LakeFS-style branching semantics at the orchestration layer for parallel step merge.

### S3 FUSE Mounts — Not Suitable

| Tool | Random Writes | Dev Tools | Status |
|------|--------------|-----------|--------|
| **Mountpoint for S3** | Sequential only, no random writes | No — no symlinks, locks, appends | Production, AWS-backed |
| **s3fs-fuse** | Via full-file re-upload (download, modify, re-upload on close) | Technically yes, extremely slow | Maintained, v1.90+ |
| **goofys** | Fails outright | No | Effectively unmaintained |

None suitable for development toolchain workloads. Designed for bulk data access (read/write large files sequentially), not metadata-heavy random-write workloads.

### SeaweedFS — Viable but More Overhead

**What it is:** Distributed blob store (master + volume servers) with FUSE mount. Own storage stack, not delegating to S3.

**Performance:** 4K random writes ~4,138 IOPS via FUSE (~9% of native NVMe). Small file creation ~11.6k files/s (acknowledged weakness).

**vs JuiceFS:** Less POSIX-compliant, no client-side caching, more infrastructure to manage (must run master + volume servers vs JuiceFS using existing S3). JuiceFS wins for our use case.

---

## Part 2: Container Workspace Patterns

### How ML Pipelines Handle Shared State

**Universal pattern: object store + references.** No major framework provides live shared filesystem between container steps.

- **Flyte:** `FlyteFile`/`FlyteDirectory` abstractions. Auto-uploads to S3, downstream gets reference, downloads on demand.
- **Metaflow (Netflix):** Auto-persists any `self.variable` to S3. Steps don't share filesystem.
- **Kubeflow:** `dsl.Artifact` subclasses serialized to artifact store. No shared filesystem.
- **Airflow:** XCom for small data, S3 + path for large data. No shared filesystem.
- **Tekton:** Exception — steps within same Task share `emptyDir` volume (same Pod). Cross-Task requires PVC.

**Takeaway:** We're going beyond what any ML framework currently offers. A live shared POSIX workspace across steps is novel infrastructure.

### OverlayFS for Parallel Branching

**How it works:** Read-only `lowerdir` (base workspace) + writable `upperdir` per parallel container. All writes go to upper layer via copy-on-write. Near-native performance.

**Merge:** No built-in merge. `overlayfs-tools` provides `diff`, `merge`, `vacuum` utilities. Merging two parallel uppers is manual — merge upper-A into lower, overlay upper-B on top, resolve conflicts at file level.

**Assessment:** Works well for parallel steps writing to non-overlapping files. For agents that might edit the same file, still need conflict resolution — OverlayFS gives file-level granularity (not line-level), so any file touched by two uppers is a conflict.

### Container Volume Patterns

| Pattern | Speed | Sharing | Use Case |
|---------|-------|---------|----------|
| **Same-Pod emptyDir** | Fastest (local disk or tmpfs) | Containers in same Pod | Sequential steps as init/sidecar containers |
| **PVC ReadWriteMany** | Network-dependent | Multiple Pods | Parallel steps needing shared access |
| **JuiceFS CSI** | Good with cache | Multiple Pods, strong consistency | Best RWX option for POSIX workloads |
| **Cloud NFS (EFS/Filestore)** | Variable | Multiple Pods | Easiest to operate |
| **virtio-fs** | Near-native | Host-guest (Kata/Firecracker) | VM-based container runtimes |

**Recommendation for sequential steps:** Same-Pod emptyDir or JuiceFS. Full POSIX, local speed.

**Recommendation for parallel steps:** JuiceFS with CSI driver. Multiple Pods mount the same filesystem with strong consistency.

### Copy-on-Write Filesystems (ZFS / Btrfs)

**ZFS clones:** Near-instant regardless of dataset size (metadata-only operation). Each clone is writable, shares blocks with parent. `zfs diff` shows changes between clone and base.

**Pattern:**
1. `zfs snapshot pool/workspace@base`
2. `zfs clone pool/workspace@base pool/workspace-agent-A` (per parallel step)
3. Agents work with full POSIX on their clone
4. `zfs diff` each clone against base
5. Apply non-conflicting changes, flag conflicts

**Merge limitation:** No built-in merge for diverged clones. Must resolve at application layer.

### Git Worktrees — Production-Proven for AI Agents

**What it is:** `git worktree add` creates a new working directory linked to same `.git` repo. Each worktree checks out a different branch. Shared object database (space-efficient).

**Production evidence:**
- **incident.io:** 4-5 parallel Claude agents routinely, completing 2-hour tasks in 10 minutes
- **Cursor 2.0:** Parallel agents (up to 8) powered by git worktrees (October 2025)
- **OpenAI Codex:** Each task in isolated container preloaded with repo, creates PR with changes

**Merge:** Standard `git merge`. Line-level conflict resolution. Well-understood tooling. AI-assisted merge available.

**Challenges:**
- Agents touching same files guarantee conflicts — no coordination layer prevents this
- Disk space: 9.82 GB for ~2GB codebase in 20 minutes with worktree creation
- Binary files: not ideal (use LFS or separate storage)

**Assessment:** Best for code-heavy workflows. The parallel AI agent workspace problem is exactly what the industry converged on git worktrees for in 2025-2026. Doesn't handle images/data/binary well — hybrid approach needed.

---

## Part 3: Cross-Workflow Workspace Sharing

### Patterns from Industry

**1. Artifact Registries (MLflow, W&B, Neptune)**
- Artifacts are versioned directories, not individual files
- Lineage is a bipartite DAG of runs and artifacts
- Aliases replace stages: `champion`, `challenger`, `latest` instead of fixed states
- Cross-workspace registries with RBAC (Azure ML pattern)
- **Applicable:** Workflow B mounts "latest" output from workflow A via alias, not specific run ID

**2. Monorepo Build Systems (Bazel, Nx, Turborepo)**
- Dependency graph is source of truth
- Content-addressable remote cache: same inputs = cached output, skip rebuild
- **Applicable:** Hash workflow inputs. If unchanged and deterministic, skip re-execution

**3. Data Mesh / Data Products**
- Each domain publishes data products with defined output ports, SLA, schema contract
- Data contracts: versioned interface between producer and consumer
- Self-describing outputs with metadata
- **Applicable:** Each workflow's store is already a data product. Add manifest/contract at mount boundary

**4. Kubernetes Namespace Sharing**
- Isolation by default, sharing opt-in and explicit
- PVCs namespace-scoped, cross-namespace requires explicit mechanisms
- Temporal's Nexus: contract-based API for cross-namespace communication
- **Validates:** Our `system_mounts` with `access: read/read_write` mirrors K8s model

**5. Composable Pipelines (Nextflow, Dagster)**
- **Nextflow:** Workflows declare `take:` (inputs) and `emit:` (outputs). Composition: `WORKFLOW_A.out.channel_name` feeds `WORKFLOW_B`
- **Dagster:** Assets are first-class — named, typed data objects with declared dependencies. Cross-job lineage in unified catalog
- **Applicable:** Typed workflow-level ports. Collection DAG wires them. Mount is implementation, port is interface

### Recommended Architecture for Cross-Workflow

1. **Typed workflow ports** — each workflow declares input/output directories (Nextflow pattern)
2. **Alias-based references** — mount "latest" or "approved" output, not specific run (MLflow pattern)
3. **Output manifests** — machine-readable description of output artifacts
4. **Content-addressable caching** — input hash enables skip-on-unchanged (Bazel pattern)
5. **Isolation by default** — each workflow has own namespace, sharing via explicit mounts (K8s pattern)

---

## Part 4: Recommended Architecture

### Single Workflow (Priority)

**Working filesystem: JuiceFS (Redis metadata + S3 storage)**

Every agent container mounts the same JuiceFS filesystem at `/workspace/`. Close-to-open consistency ensures step B sees step A's files after A completes. Local SSD cache + writeback mode handles the small-file burden of dev toolchains.

**Sequential steps:** Straightforward. Step A writes files, closes, syncs. Step B mounts, sees everything. No merge needed.

**Parallel steps — two strategies:**

*Strategy A: Directory isolation (simple, no merge)*
- Parallel steps write to different directories
- Builder ensures parallel steps don't share write targets
- All writes land on same JuiceFS filesystem — no merge
- Works for genuinely independent parallel work

*Strategy B: OverlayFS branching (when parallel steps must share directories)*
- Snapshot base workspace via OverlayFS lowerdir
- Each parallel container gets an OverlayFS upper layer
- Agents work with full POSIX on their overlay
- After completion, merge uppers:
  - New files from either: take both
  - Modified by one: take that version
  - Modified by both: LLM merge agent resolves (reads both versions, produces combined)

**Within a workforce (same step):** Agents share one container, one filesystem. Sequential execution. No merge needed.

### Container Lifecycle

```
Step starts:
  → Container launches with JuiceFS mounted at /workspace/
  → All previous steps' files visible immediately (close-to-open)
  → Agent runs with full POSIX — pip install, python main.py, etc.
  → Agent writes output files to /workspace/
  → Step completes, container torn down

Parallel steps:
  → Multiple containers mount same JuiceFS
  → Strategy A: write to different dirs (no conflict)
  → Strategy B: OverlayFS branching (merge after)
```

### Cross-Workflow (Future)

Each workflow gets its own JuiceFS namespace (prefix). Cross-workflow sharing via mounts:

```
Workflow A: /workspace-a/
Workflow B: /workspace-b/
            /workspace-b/imports/workflow-a/  ← read-only mount of A's output
```

Typed ports declare what's shared. Alias resolution (`latest`, `v2`) determines which run's output is mounted. Collection DAG wires the ports.

---

## Part 5: Three-Way Merge Strategy for Parallel Steps

The research found no filesystem-level merge solution. Every option — OverlayFS, ZFS, LakeFS, JuiceFS — stops at file-level conflict detection. None do line-level merge. Git does line-level merge but can't handle binary files and has size limits.

The answer: don't solve it at the filesystem level. Solve it at the application level using standard three-way diff, and only invoke an LLM for the rare cases where two agents touched the same lines.

### How Three-Way Merge Works

Three versions of every file:
- **Base** — the file before the parallel batch started (snapshot)
- **Version A** — what agent A's container has after execution
- **Version B** — what agent B's container has after execution

`diff3` (standard Unix tool) compares all three and categorizes every section:

```
Unchanged by both     → keep base           (automatic)
Changed by A only     → take A's version    (automatic)
Changed by B only     → take B's version    (automatic)
Changed by both       → CONFLICT            (needs resolution)
New file from A only  → take it             (automatic)
New file from B only  → take it             (automatic)
New file from both    → CONFLICT            (needs resolution)
```

Most of the file merges automatically. Only the conflict hunks — sections where both agents modified the same lines — need intelligence.

### Conflict Resolution via LLM

When a conflict hunk is detected, send just that hunk (plus surrounding context) to Haiku:

```
File: /workspace/my_app/main.py
Lines 36-42 conflict.

Context (lines 30-35, unchanged):
  from flask import Flask
  app = Flask(__name__)

Agent A wrote (lines 36-42):
  from auth import auth_middleware
  from rate_limit import limiter
  app.use(auth_middleware)
  app.use(limiter)

Agent B wrote (lines 36-42):
  from db import init_database
  from models import Base
  init_database(app)
  Base.metadata.create_all()

Context (lines 43-48, unchanged):
  @app.route('/')
  def index():

Combine both agents' changes into a single coherent version.
```

Haiku returns:

```python
from auth import auth_middleware
from rate_limit import limiter
from db import init_database
from models import Base
app.use(auth_middleware)
app.use(limiter)
init_database(app)
Base.metadata.create_all()
```

10-20 lines of context. One Haiku call. Fractions of a cent.

### Binary File Conflicts

Binary files (images, data, compiled artifacts) can't be diffed at the line level. Policy options:
- **Last-write-wins** — take the version from whichever step completed last
- **Both-keep** — rename one (`image.png` → `image_step_a.png`, `image_step_b.png`)
- **Flag for user** — pause the DAG and ask the user to choose

For most workflows, binary conflicts shouldn't happen — parallel steps producing images are usually producing different images to different paths.

### The Full Merge Flow

```
1. Before parallel batch:
   → Snapshot workspace (file paths + checksums from system_files)

2. Parallel steps execute:
   → Each container mounts JuiceFS at /workspace/
   → Each writes to the shared filesystem
   → JuiceFS close-to-open consistency handles isolation

3. All parallel steps complete:
   → Collect changed files from each step (checksum diff against snapshot)
   → Categorize:
     - Changed by one step only → accept (no conflict)
     - New files → accept (no conflict)
     - Changed by multiple steps → run diff3

4. For each conflicted text file:
   → diff3 base vs version_A vs version_B
   → Auto-merge clean sections (programmatic, no LLM)
   → Send conflict hunks to Haiku with context
   → Apply resolved hunks back into merged file

5. For each conflicted binary file:
   → Apply policy (last-write-wins / both-keep / flag)

6. Write merged workspace back
   → Next batch of steps sees the clean merged result
```

### Why This Works

- **diff3 is battle-tested** — standard Unix tool, used by git internally, handles edge cases
- **Most merges are automatic** — parallel steps doing different work rarely touch the same lines
- **Conflicts are small** — a 500-line file with a 5-line conflict means the LLM sees ~20 lines, not 500
- **Haiku is cheap** — resolving a merge conflict is easier than writing code from scratch
- **No new infrastructure** — diff3 is available everywhere, checksum tracking already exists in system_files

### Expected Conflict Rate

If the builder designs the DAG well (parallel steps doing genuinely different work):
- **Different files modified** — most common case, auto-merge, zero cost
- **Same file, different sections** — auto-merge, zero cost
- **Same file, overlapping sections** — rare, one Haiku call per hunk
- **Same file, same lines** — very rare, one Haiku call

The merge system exists as a safety net for the cases the builder couldn't predict, not as the primary execution path.

---

## Part 6: Workspace File Organization

Research into how AI agents, CI/CD systems, and sandboxed environments organize working files vs output files in shared workspaces.

### Key Finding: Nobody Separates Working Files from Output

**No production AI agent system creates a separate "output" directory.** The workspace IS the working directory. Agents modify files in-place. The output is the diff/branch/PR, not a special directory.

- **Codex:** works directly in the repo clone. The diff IS the output.
- **Cursor:** clones to isolated VM, works on branch. Output is the branch/PR.
- **SWE-Agent:** works in cloned repo inside container. Metadata (trajectories) stored externally on host.
- **OpenHands:** mounts workspace at `/opt/workspace_base`. No source/output split.
- **Manus AI:** writes working notes and outputs to the same filesystem.

### CI/CD: Artifacts Are Selection, Not Location

CI/CD systems don't use separate output directories during execution. Artifact separation happens AFTER the build.

- **GitHub Actions:** `actions/upload-artifact` selects which workspace files to preserve. No special output dir.
- **Jenkins:** `archiveArtifacts` copies from workspace to build archive post-build.
- **GitLab CI:** `artifacts: paths:` in YAML declares which files to keep. Relative to repo root.

### Build Systems: The Exception

Build systems DO separate source from output — but AI agents aren't build systems.

- **Bazel:** source tree is never written to. All output to `~/.cache/bazel/`.
- **Nix:** hermetic sandbox, all output to `/nix/store/<hash>-<name>/`.
- **Gradle/Maven:** `build/` or `target/` subdirectories.

### Emerging: folder.md Convention

A `FOLDER.md` file in the workspace root declares directory purposes:
- `drafts/` — scratch space
- `final/` — locked until finalized
- `prompt/` — read-only
- `notes/` — append-only

Closest thing to formal working/output separation for AI agents. Not yet widely adopted.

### Decision: Don't Separate

The workspace is the workspace. Agents create directories as they see fit. The handoff text (shaped by `expected_output`) tells the next step what's important and where to look. Everything else is noise the next agent can ignore. Fresh workspace per run means nothing accumulates across runs.

Sources:
- [OpenAI Codex CLI](https://developers.openai.com/codex/cli/reference/)
- [Cursor Background Agents](https://docs.cursor.com/en/background-agent)
- [SWE-Agent Trajectories](https://swe-agent.com/latest/usage/trajectories/)
- [GitHub Actions Artifacts](https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts)
- [Bazel Output Directory Layout](https://bazel.build/remote/output-directories)
- [folder.md Convention](https://www.folder.md/docs)
- [Anthropic Multi-Agent Research](https://www.anthropic.com/engineering/multi-agent-research-system)

---

## Decision Matrix

| Concern | Solution | Status |
|---------|----------|--------|
| Real POSIX filesystem for agents | JuiceFS (Redis + S3) | Available, production-ready |
| Sequential step file sharing | JuiceFS close-to-open consistency | Built-in |
| Parallel step isolation | Directory isolation OR OverlayFS branching | Strategy choice |
| Parallel step merge (text files) | Three-way diff3 + LLM conflict resolver | To build (standard tools) |
| Parallel step merge (binary files) | Last-write-wins or flag for user | Policy choice |
| Dev toolchain support (pip, npm, cargo) | JuiceFS with Redis metadata + local SSD cache + writeback | Configuration |
| Cross-workflow composition | Typed ports + alias mounts + manifests | Future |
| Versioning / rollback | Application-layer snapshots (already have `run_snapshots`) | Existing |

## Key Sources

- [JuiceFS Docs](https://juicefs.com/docs/community/introduction/) — Architecture, benchmarks, K8s CSI
- [LakeFS Merge Docs](https://docs.lakefs.io/v1.73/understand/how/merge/) — Three-way merge semantics
- [overlayfs-tools](https://github.com/kmxz/overlayfs-tools) — Diff/merge for OverlayFS layers
- [Git Worktrees for AI Agents](https://nx.dev/blog/git-worktrees-ai-agents) — Production patterns (incident.io, Cursor)
- [Nextflow Workflow Composition](https://training.nextflow.io/2.1.2/side_quests/workflows_of_workflows/) — `take:`/`emit:` pattern
- [Dagster Software-Defined Assets](https://dagster.io/blog/software-defined-assets) — Asset-based composition
- [Temporal Nexus](https://community.temporal.io/t/cross-cluster-namespace-workflow-chaining/7347) — Cross-namespace contracts
- [Flyte Data Management](https://docs.flyte.org/en/latest/user_guide/concepts/main_concepts/data_management.html) — FlyteFile/FlyteDirectory
- [ZFS Snapshots & Clones](https://www.illumos.org/books/zfs-admin/snapshots.html) — COW workspace cloning
