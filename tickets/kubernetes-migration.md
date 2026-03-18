# Kubernetes Migration — Replace Docker Compose with Local K8s

## Summary

Migrate the dev environment from Docker Compose + FUSE mount propagation to Kubernetes (Docker Desktop's built-in single-node cluster) with the JuiceFS CSI driver. This eliminates FUSE propagation hacks, privileged containers for JuiceFS, and the Docker 26+ version requirement for `volume-subpath`. The CSI driver handles JuiceFS mounts natively at the node level.

## Why

The current Docker Compose setup has friction:

1. **FUSE mount sharing** — JuiceFS runs in a privileged container with `SYS_ADMIN` + `/dev/fuse`. Sharing the mount between the server and agent containers requires `rshared` propagation, which [doesn't work reliably on Docker Desktop macOS](https://forums.docker.com/t/make-mount-point-accesible-from-container-to-host-rshared-not-working/108759). We work around this with named volumes, but it's fragile.

2. **Docker version dependency** — Agent workspace mounts use `volume-subpath` which requires Docker 26+. This is a hard dependency that can break on older Docker installations.

3. **Privileged containers** — The server container needs `SYS_ADMIN` for OverlayFS, the JuiceFS container needs `SYS_ADMIN` + `/dev/fuse`. In Kubernetes, the CSI driver runs at the node level — application pods don't need privileges for storage access.

4. **Sibling container management** — The server creates agent containers via Docker socket (`/var/run/docker.sock`). This works but is a hack — paths must resolve from the Docker daemon's perspective, not the server container's. In Kubernetes, the server would create agent Pods via the Kubernetes API, and volume mounts are declarative.

5. **Prod parity** — Production will run on Kubernetes. Building on Compose means rewriting the orchestration layer later. Building on Kubernetes from the start means the dev and prod environments are structurally identical.

## Architecture

```
Docker Desktop Kubernetes (single-node)
├── JuiceFS CSI Driver (DaemonSet + Controller)
│   └── manages JuiceFS mounts at the node level
├── nexor-server (Deployment)
│   └── mounts JuiceFS PVC at /mnt/jfs
│   └── creates agent Pods via Kubernetes API
├── agent Pods (created dynamically by server)
│   └── each gets a JuiceFS volume with subPath for its run directory
├── postgres (StatefulSet or Deployment)
│   └── PVC for pgdata
└── minio (Deployment)
    └── PVC for minio-data
```

### JuiceFS CSI Driver

The [JuiceFS CSI driver](https://github.com/juicedata/juicefs-csi-driver) implements the Kubernetes Container Storage Interface. It runs as a DaemonSet on each node and handles mounting JuiceFS volumes for any Pod that requests them. No FUSE setup in application containers, no privileged mode, no mount propagation.

**Installation:**
```bash
# Enable Kubernetes in Docker Desktop settings
helm repo add juicefs https://juicedata.github.io/charts/
helm upgrade --install juicefs-csi-driver juicefs/juicefs-csi-driver -n kube-system
```

**JuiceFS Secret (credentials):**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: juicefs-secret
type: Opaque
stringData:
  name: nexor-workspace
  metaurl: "postgres://nexor:nexor@postgres:5432/juicefs?sslmode=disable"
  storage: minio
  bucket: "http://minio:9000/juicefs-data"
  access-key: minioadmin
  secret-key: minioadmin
```

**StorageClass:**
```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: juicefs
provisioner: csi.juicefs.com
parameters:
  csi.storage.k8s.io/provisioner-secret-name: juicefs-secret
  csi.storage.k8s.io/provisioner-secret-namespace: default
  csi.storage.k8s.io/node-publish-secret-name: juicefs-secret
  csi.storage.k8s.io/node-publish-secret-namespace: default
reclaimPolicy: Retain
```

**PVC for workspace:**
```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: nexor-workspace
spec:
  accessModes:
    - ReadWriteMany
  storageClassName: juicefs
  resources:
    requests:
      storage: 100Gi
```

### Agent Pods with subPath

The server currently creates agent containers via Docker socket with `--mount type=volume,...,volume-subpath=...`. In Kubernetes, agent containers become Pods with `volumeMounts.subPath`:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: nexor-agent-{uuid}
spec:
  containers:
    - name: agent
      image: nexor-agent:latest
      workingDir: /workspace
      command: ["sleep", "infinity"]
      volumeMounts:
        - name: workspace
          mountPath: /workspace
          subPath: workflows/{workflow_id}/runs/{run_id}
      resources:
        limits:
          memory: "2Gi"
          cpu: "2"
      securityContext:
        capabilities:
          drop: ["ALL"]
  volumes:
    - name: workspace
      persistentVolumeClaim:
        claimName: nexor-workspace
```

`subPath` is a native Kubernetes feature — works on all versions, no special Docker version required.

### Server Changes

The biggest code change: replace `ContainerManager` (Docker CLI calls) with a Kubernetes client that creates/manages Pods.

| Current (Docker) | Future (Kubernetes) |
|--|--|
| `docker create` + `docker start` | `kubectl create pod` (via k8s API) |
| `docker exec` | `kubectl exec` (via k8s API) |
| `docker rm -f` | `kubectl delete pod` |
| `-v volume:/workspace` | `volumeMounts` with `subPath` |
| `--cap-drop=ALL` | `securityContext.capabilities.drop` |
| Docker socket (`/var/run/docker.sock`) | Kubernetes API (in-cluster ServiceAccount) |

The `kube-rs` crate provides a native Rust Kubernetes client. The `ContainerHandle` abstraction can be adapted — `exec()` becomes `kubectl exec`, `read_file`/`write_file` become exec'd `cat`/`sh -c` commands (same as today).

### OverlayFS in Kubernetes

For B3 (OverlayFS), the server Pod still needs `SYS_ADMIN` to run `mount -t overlay`. This doesn't change with Kubernetes — but only the server Pod needs it, not agent Pods. The server creates the overlay on the JuiceFS volume before spawning the agent Pod.

Alternatively, OverlayFS setup could become an init container on the agent Pod — but that requires privileged init containers, which is worse.

## Implementation Phases

### Phase 1: Kubernetes manifests for infrastructure

- Postgres Deployment + PVC (migrate from Compose)
- MinIO Deployment + PVC (migrate from Compose)
- JuiceFS CSI driver (Helm install)
- JuiceFS Secret + StorageClass + PVC
- Namespace: `nexor`

### Phase 2: Server Deployment

- `docker/Dockerfile.dev` → works as-is for the Pod image
- Kubernetes Deployment for nexor-server
- ServiceAccount with permissions to create/delete/exec Pods
- Mount JuiceFS PVC at `/mnt/jfs`
- Expose port 3000 via Service

### Phase 3: Replace ContainerManager with KubernetesManager

- New `KubernetesManager` in `src/execution/` (parallel to `ContainerManager`)
- Uses `kube-rs` crate for Pod CRUD + exec
- `KubernetesHandle` implements same interface as `ContainerHandle`
- Volume mounts use PVC + subPath instead of Docker named volumes
- Feature flag or config to switch between Docker and Kubernetes backends

### Phase 4: Agent Pod lifecycle

- Agent Pods created with JuiceFS subPath mount
- Exec commands via Kubernetes exec API
- Pod cleanup (reaper equivalent: label-based Pod garbage collection)
- Timeout handling (Pod deadline / active deadline seconds)

### Phase 5: Dev workflow

- `make server` → `kubectl apply -k k8s/dev/`
- `make server-down` → `kubectl delete -k k8s/dev/`
- `make server-logs` → `kubectl logs -f deployment/nexor-server`
- Kustomize overlays for dev vs prod

## What Doesn't Change

- All Rust application code (handlers, services, DAG executor, designer, etc.)
- WorkspaceManager (still creates dirs on JuiceFS, just mounted differently)
- Agent Dockerfile (same image, just runs as a Pod instead of a container)
- Frontend
- OverlayFS logic (B3-B5) — same `mount -t overlay` commands

## Dependencies

- Docker Desktop with Kubernetes enabled (single checkbox)
- `helm` CLI for CSI driver installation
- `kube-rs` crate added to Cargo.toml
- `kubectl` CLI for dev workflow

## Risks

1. **kube-rs complexity** — Kubernetes API is more complex than Docker CLI. Pod creation, exec, and lifecycle management require more code than `docker create/exec/rm`.

2. **Exec latency** — Kubernetes exec goes through the API server → kubelet → container runtime. May be slower than `docker exec` for high-frequency tool calls.

3. **Docker Desktop K8s stability** — Docker Desktop's Kubernetes has occasional issues. Need to validate it's reliable enough for daily dev.

4. **Migration effort** — Rewriting ContainerManager is a significant change that touches the execution layer. Needs thorough testing.

## References

- [JuiceFS CSI Driver](https://github.com/juicedata/juicefs-csi-driver)
- [JuiceFS CSI Installation](https://juicefs.com/docs/csi/getting_started/)
- [JuiceFS CSI Configurations](https://juicefs.com/docs/csi/guide/configurations/)
- [JuiceFS subPath mounting](https://juicefs.com/docs/csi/examples/subpath/)
- [kube-rs — Rust Kubernetes client](https://github.com/kube-rs/kube)
- [Docker Desktop Kubernetes](https://docs.docker.com/desktop/kubernetes/)
