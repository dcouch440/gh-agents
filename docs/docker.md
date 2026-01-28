# Docker Deployment

Run nexor in a Docker container without installing Rust.

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+ (optional, for easier management)

## Quick Start

### Using Docker directly

```bash
# Build the image
docker build -t nexor -f docker/Dockerfile .

# Run with TUI (interactive)
docker run -it \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -v $(pwd):/project \
  nexor

# Run headless
docker run \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -v $(pwd):/project \
  nexor --headless --task "implement feature X"
```

### Using Docker Compose

```bash
# Copy example env file and add your keys
cp .env.example .env
# Edit .env with your API keys

# Build and run
docker-compose -f docker/docker-compose.yml run nexor

# Run headless batch job
docker-compose -f docker/docker-compose.yml run nexor --headless --task "your task"
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `ANTHROPIC_API_KEY` | Anthropic API key | Yes |
| `GITHUB_TOKEN` | GitHub personal access token | For GitHub features |
| `RUST_LOG` | Log level (info, debug, trace) | No (default: info) |
| `PROJECT_DIR` | Project directory to mount | No (default: current dir) |

### Volume Mounts

| Mount | Purpose |
|-------|---------|
| `/project` | Your project directory (code access) |
| `/data/.nexor` | Persistent nexor data (database, logs) |

## Use Cases

### Interactive TUI

```bash
docker-compose -f docker/docker-compose.yml run nexor
```

Requires TTY. Works in terminal, not in CI.

### Headless for CI/CD

```bash
docker run \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -v $(pwd):/project \
  nexor --headless --task "run tests and fix failures"
```

### Batch Processing

```bash
# Create tasks file
echo "Add unit tests" > tasks.txt
echo "Update documentation" >> tasks.txt

# Run batch
docker run \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -v $(pwd):/project \
  -v $(pwd)/tasks.txt:/input.txt:ro \
  nexor --headless --input /input.txt
```

### GitHub Issue Sync

```bash
docker run \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -e GITHUB_TOKEN=$GITHUB_TOKEN \
  -v $(pwd):/project \
  nexor --headless --sync "https://github.com/owner/repo/issues/123"
```

## Building

### Development Build

```bash
# Build without cache (useful for debugging)
docker build --no-cache -t nexor -f docker/Dockerfile .
```

### Optimized Build

```bash
# Build with BuildKit for better caching
DOCKER_BUILDKIT=1 docker build -t nexor -f docker/Dockerfile .
```

## Troubleshooting

### "API key not found"

Ensure environment variables are passed to container:
```bash
docker run -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY ...
```

### "Permission denied" on mounted volumes

The container runs as UID 1000. Ensure your files are accessible:
```bash
chmod -R a+rw /path/to/project
```

Or run as root (not recommended for production):
```bash
docker run --user root ...
```

### "Database locked"

Only one nexor instance can access the database. Stop other containers:
```bash
docker-compose -f docker/docker-compose.yml down
```

### Can't see TUI output

TUI requires interactive terminal. Use `-it` flags:
```bash
docker run -it ...  # Note: -it not just -i
```

### Container exits immediately

Check if you're in headless mode without a task:
```bash
# This will exit immediately
docker run nexor --headless

# This will work
docker run nexor --headless --task "your task"
```

## Image Details

- **Base Image**: `debian:bookworm-slim`
- **Runtime User**: `nexor` (UID 1000)
- **Working Directory**: `/project`
- **Data Directory**: `/data/.nexor`
- **Expected Size**: ~50-100MB

## Security Notes

1. **Non-root User**: The container runs as a non-root user by default
2. **Volume Permissions**: Mounted volumes should have appropriate permissions
3. **API Keys**: Never bake API keys into the image; always pass via environment
4. **Network**: The container doesn't expose any ports (TUI app)
