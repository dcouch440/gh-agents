# Multi-Modal Agent Capabilities — Vision

## What It Is

Multi-modal tools that let agents generate, view, and compose images and video. Built on top of the system store — agents write generated media to the store, downstream agents discover it through the artifact manifest, and vision-capable agents see actual pixels when they `read_file` an image.

## Depends On

- **System Store** (`vision-system-store.md`) — the filesystem, artifact manifest, and implicit `read_file`/`write_file` tools must exist first. Multi-modal tools are store tools — `generate_image` writes to the store, `read_file` returns vision content blocks for images.

## Agents That See Images

Agents can view images through vision content blocks. When `read_file` returns an image, it's sent as `ContentBlock::Image` — the agent sees the actual pixels.

### Media-Aware File Reading

```rust
match meta.media_type.as_str() {
    "image/png" | "image/jpeg" | "image/webp" => {
        ContentBlock::Image { source: ImageSource {
            source_type: "base64",
            media_type: meta.media_type,
            data: base64::encode(&bytes),
        }}
    }
    "text/markdown" | "text/plain" | "application/json" => {
        ContentBlock::Text { text: String::from_utf8_lossy(&bytes) }
    }
    "audio/mp3" | "video/mp4" => {
        // Can't "see" these — return description
        ContentBlock::Text {
            text: format!("[{}: {} — {}]", meta.media_type, path, meta.description)
        }
    }
}
```

This enables self-correcting pipelines — a vision agent can generate an image, read it back, evaluate quality, and regenerate if needed.

## Multi-Modal Capabilities

### Image Generation

xAI Grok Imagine — same API already integrated:

```
POST https://api.x.ai/v1/images/generations
  model: "grok-imagine-image"       $0.02/image
  model: "grok-imagine-image-pro"   $0.07/image

POST https://api.x.ai/v1/images/edits
  Accepts up to 3 source images for editing/composition
```

### Video Generation

```
POST https://api.x.ai/v1/videos/generations
  model: "grok-imagine-video"       $0.05/second
  Supports text-to-video and image-to-video
  Async: submit → poll → retrieve
  Duration: 1-15 seconds per clip
```

### New Tools

```
generate_image:
  prompt: string
  style_ref: optional path
  aspect_ratio: "16:9" | "1:1" | "9:16"
  → generates image via Grok Imagine
  → stores in system store with prompt as description

generate_video:
  prompt: string
  image_url: optional path (for image-to-video)
  duration: 1-15 seconds
  → submits to Grok Imagine Video (async poll)
  → stores in system store with metadata
```

Both tools write to the system store automatically. Generated media appears in the `<upstream_artifacts>` manifest for downstream agents. The designer tells agents what to generate — the store and manifest handle storage and discovery.

## Input Hash Caching

Hash each step's inputs (upstream artifact hashes + prompt + config). If the hash matches a previous execution, skip the step and reuse cached output. Critical when image gen is $0.02/image and video gen is $0.05/second.

```rust
let input_hash = blake3::hash(&serialize(&step.prompt, &step.config, &upstream_hashes));
if let Some(cached) = store.get_by_input_hash(input_hash) {
    return cached;  // skip execution entirely
}
```

This matters most for multi-modal pipelines where regenerating unchanged steps wastes real money. A 6-scene video pipeline that re-runs the story step shouldn't regenerate all images and video clips.

## The Demo Workflow

```
[Write Story] → [Split Scenes] → [Generate Images] → [Generate Video] → [Assemble]
                                        ↑
                                  [Style Guide]
```

1. **Write Story** — workforce writes `.system/artifacts/docs/story.md` and `.system/artifacts/docs/characters.md`
2. **Style Guide** — agent writes `.system/refs/style.md` with visual rules
3. **Split Scenes** — reads story from `<upstream_artifacts>` manifest, writes `.system/artifacts/data/scenes.json`
4. **Generate Images** — parallel fan-out, one image per scene via Grok Imagine, each stored with description
5. **Generate Video** — image-to-video for each scene via Grok Imagine Video
6. **Assemble** — ffmpeg in Docker container, stitches clips into final video

Each step discovers upstream artifacts through the runtime manifest. The designer tells agents what to produce and what to expect — the manifest delivers the paths automatically.

**Cost for a 6-scene short film**: ~$0.12 images + ~$1.80 video + ~$2 LLM calls. Under $5.

## Implementation Slices

Depends on system store slices 1 + 4 (Store Foundation + Implicit Agent Tools).

1. **Media-Aware Reading** — `read_file` returns `ContentBlock::Image` for image types, text for text types, description placeholder for audio/video. Extends the store's implicit `read_file` tool.
2. **Image Generation** — `generate_image` tool wrapping xAI Grok Imagine API. Writes to store with prompt as description. Available as an agent capability.
3. **Video Generation** — `generate_video` tool wrapping xAI Grok Imagine Video API. Async poll pattern. Writes to store with metadata.
4. **Input Hash Caching** — Blake3 hash of step inputs. Cache lookup before execution. Skip unchanged steps in expensive pipelines.

## What This Enables

- Multi-modal workflows (text + image + video generation in one pipeline)
- Self-correcting pipelines (vision agents QA their own output, regenerate on failure)
- Story-to-movie pipelines under $5
- Image editing workflows (Grok Imagine edits accept up to 3 source images)
- Cached re-runs that skip unchanged expensive steps
