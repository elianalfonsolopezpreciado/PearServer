# Pear Server Architecture

## Table of Contents
1. [Overview](#overview)
2. [System Layers](#system-layers)
3. [Phase 2: Middle Layer Components](#phase-2-middle-layer-components)
4. [Data Flow](#data-flow)
5. [Cage Lifecycle](#cage-lifecycle)
6. [Self-Healing Mechanism](#self-healing-mechanism)
7. [CRDT Synchronization](#crdt-synchronization)
8. [AI Security](#ai-security)

## Overview

Pear Server is a revolutionary next-generation web server that operates as a userspace pseudo-operating system. It uses a self-healing Cage Pool architecture where websites run in redundant WebAssembly micro-VMs with internal traffic routing and CRDT-based state synchronization.

```
┌─────────────────────────────────────────────────────────────┐
│                    Pear Server System                       │
├─────────────────────────────────────────────────────────────┤
│  ┌────────────┐      ┌────────────┐                         │
│  │  HTTP/2    │      │  HTTP/3    │                         │
│  │  (TCP)     │      │  (QUIC)    │                         │
│  └──────┬─────┘      └──────┬─────┘                         │
│         │                   │                               │
│         └──────────┬────────┘                               │
│                    │                                        │
│         ┌──────────▼──────────┐                             │
│         │   Traffic Router    │  ◄─── AI Security           │
│         │  Load Balancing     │       Anomaly Detection     │
│         └──────────┬──────────┘                             │
│                    │                                        │
│         ┌──────────▼──────────┐                             │
│         │    Cage Pool        │  ◄─── Supervisor            │
│         │  (3 Redundant       │       Self-Healing          │
│         │   Wasm Instances)   │                             │
│         └──────────┬──────────┘                             │
│                    │                                        │
│      ┌─────────────┼─────────────┐                          │
│      ▼             ▼             ▼                          │
│   ┌─────┐       ┌─────┐       ┌─────┐                       │
│   │Cage │       │Cage │       │Cage │                       │
│   │  1  │       │  2  │       │  3  │                       │
│   └──┬──┘       └──┬──┘       └──┬──┘                       │
│      └─────────────┼─────────────┘                          │
│                    │                                        │
│         ┌──────────▼──────────┐                             │
│         │   CRDT State Sync   │                             │
│         │  (Automerge)        │                             │
│         └─────────────────────┘                             │
└─────────────────────────────────────────────────────────────┘
```

## System Layers

### Layer 1: Foundation (Phase 1)
- **Networking**: HTTP/2 (Hyper) + HTTP/3 (Quinn)
- **Runtime**: Tokio async with optimized worker threads
- **State Management**: Lock-free concurrent data structures
- **Signal Handling**: Graceful shutdown (SIGTERM/SIGINT)
- **Observability**: Structured JSON logging

### Layer 2: Middle Layer (Phase 2)
- **Cage Architecture**: WebAssembly execution with Wasmtime
- **Traffic Router**: Intelligent request distribution
- **Supervisor**: Self-healing and crash recovery
- **CRDT Sync**: Eventually consistent shared state
- **AI Security**: ML-powered anomaly detection

### Layer 3: Application Layer (Phase 3 - Future)
- **User Configuration**: Web UI for management
- **Deployment**: CI/CD integration
- **Scaling**: Auto-scaling based on load

## Phase 2: Middle Layer Components

### 1. Cage Architecture

**Purpose**: Isolated WebAssembly execution environments with strict resource limits.

**Key Components**:
- `Cage`: Represents a single Wasm instance
- `CageConfig`: Resource limits (memory, CPU, permissions)
- `CageState`: Lifecycle states (Initializing → Running → Crashed → Terminated)

**Resource Limits**:
- Memory: 128MB (default), configurable
- CPU Timeout: 1000ms per request
- Max Concurrent Requests: 100 per Cage

**Code Location**: `src/cage/`

### 2. Cage Pool System

**Purpose**: Manage redundant Cage instances for high availability.

**Features**:
- Configurable redundancy (default: 3 replicas)
- Health monitoring
- Automatic replica maintenance
- Load distribution strategies

**Pool Health Stats**:
- Total Cages
- Healthy Cages
- Crashed Cages
- Initializing Cages

**Code Location**: `src/cage/pool.rs`

### 3. Traffic Router

**Purpose**: Distribute incoming requests across healthy Cages.

**Load Balancing Strategies**:
1. **Round-Robin**: Distributes evenly across all Cages
2. **Least-Connected**: Routes to Cage with fewest active requests
3. **Weighted** (future): Capacity-based distribution

**Request Flow**:
```
HTTP Request
   │
   ▼
Extract Site ID
   │
   ▼
Get CagePool for Site
   │
   ▼
Select Cage (Strategy)
   │
   ▼
Execute in Cage
   │
   ▼
Return Response
```

**Code Location**: `src/router/`

### 4. Self-Healing Supervisor

**Purpose**: Automatic failure detection and recovery.

**Features**:
- Continuous health monitoring (5-second interval)
- Crash detection
- Automatic Cage respawn with exponential backoff
- Zero-downtime recovery

**Exponential Backoff**:
- Base delay: 1 second
- Max delay: 60 seconds
- Max attempts: 5

**Recovery Process**:
1. Detect crashed Cage
2. Calculate backoff delay
3. Remove crashed instance
4. Spawn new Cage
5. Wait for initialization
6. Resume traffic routing

**Code Location**: `src/supervisor/`

### 5. CRDT State Synchronization

**Purpose**: Share state across redundant Cage instances.

**Technology**: Automerge (mature CRDT library)

**Features**:
- Eventually consistent
- Automatic conflict resolution
- Delta-based synchronization
- 100ms sync interval (configurable)

**Data Structures**:
- User sessions
- Shopping carts
- Custom application state

**Synchronization Flow**:
```
Cage 1: Set("key", "value1")
   │
   ▼
Generate Change Delta
   │
   ▼
Broadcast to Other Cages
   │
   ├──▼ Cage 2: Apply Delta
   └──▼ Cage 3: Apply Delta
```

**Code Location**: `src/crdt/`

### 6. AI Security Module

**Purpose**: ML-powered threat detection and anomaly identification.

**Features**:
- Isolation Forest anomaly detection (Linfa)
- Real-time traffic analysis
- Configurable sampling (10% default)
- Threat classification

**Anomaly Detection**:
- Feature extraction from requests
- Statistical scoring
- Threshold-based alerting (80% default)

**Threat Types**:
- Anomalous traffic patterns
- SQL injection
- XSS attempts
- DDoS patterns
- Bot activity

**Code Location**: `src/ai/`

## Data Flow

### Request Processing Flow

```
1. HTTP/2 or HTTP/3 Request Arrives
   │
   ▼
2. AI Security: Analyze Request (Sampled)
   │
   ▼
3. Router: Extract Site ID
   │
   ▼
4. Router: Get CagePool
   │
   ▼
5. Router: Select Healthy Cage (Load Balance)
   │
   ▼
6. Cage: Execute Request in Wasm
   │
   ▼
7. CRDT: Read/Write Shared State
   │
   ▼
8. Cage: Generate Response
   │
   ▼
9. Router: Return Response
   │
   ▼
10. HTTP/2 or HTTP/3 Response
```

## Cage Lifecycle

### State Machine

```
[  START  ]
     │
     ▼
[INITIALIZING]
     │
     ├─(success)──▼
     │      [RUNNING] ◄────┐
     │           │         │
     │           │         │
     │     (crash)▼    (recover)
     │        [CRASHED]────┘
     │           │
     │    (terminate)
     │           │
     ▼           ▼
[TERMINATING]───▼
     │
     ▼
[TERMINATED]
```

### State Descriptions

- **INITIALIZING**: Cage is loading Wasm module and setting up runtime
- **RUNNING**: Cage is healthy and handling requests
- **CRASHED**: Cage has failed (panic, timeout, or resource exhaustion)
- **TERMINATING**: Cage is gracefully shutting down
- **TERMINATED**: Cage has been cleaned up

## Self-Healing Mechanism

### Healing Decision Tree

```
Supervisor Check
   │
   ▼
 Crashed Cages > 0?
   │
   ├─No──▼ Continue Monitoring
   │
   └─Yes──▼
       │
       ▼
   Respawn Attempts < Max?
       │
       ├─No──▼ Give Up (Log Error)
       │
       └─Yes──▼
           │
           ▼
       Calculate Backoff
           │
           ▼
       Enough Time Passed?
           │
           ├─No──▼ Wait Longer
           │
           └─Yes──▼
               │
               ▼
           Remove Crashed Cage
               │
               ▼
           Spawn New Cage
               │
               ▼
           Success?
               │
               ├─Yes──▼ Reset Attempts
               │
               └─No──▼ Increment Attempts
```

## CRDT Synchronization

### Synchronization Protocol

**Interval**: 100ms (configurable)

**Process**:
1. Each Cage maintains a local Automerge document
2. Every 100ms, SyncCoordinator runs:
   - Collect changes from all Cages
   - Generate delta patches
   - Apply patches to all other Cages
3. Conflicts are automatically resolved by Automerge

**Example**:
```rust
// Cage 1
state.set("user.name", "Alice").await?;

// Cage 2 (simultaneously)
state.set("user.email", "alice@example.com").await?;

// After sync (both Cages)
// user.name = "Alice"
// user.email = "alice@example.com"
```

## AI Security

### Anomaly Detection Pipeline

```
Request
   │
   ▼
Extract Features
   │
   ├─ Path length
   ├─ Query params count
   ├─ Headers count
   └─ Body size
   │
   ▼
Isolation Forest Model
   │
   ▼
Anomaly Score (0.0-1.0)
   │
   ▼
Score > Threshold?
   │
   ├─No──▼ Allow Request
   │
   └─Yes──▼ Flag/Block Request
```

### Model Training

- **Algorithm**: Isolation Forest
- **Training**: Auto-trains after 100 samples
- **Features**: Request characteristics
- **Output**: Anomaly score (0.0 = normal, 1.0 = highly anomalous)

## Performance Characteristics

| Metric | Phase 1 | Phase 2 | Target |
|--------|---------|---------|--------|
| Request Latency | ~1ms | ~8ms | <10ms |
| Memory per Site | ~10MB | ~300MB | <500MB |
| Throughput | 100k req/s | 80k req/s | 50k+ req/s |
| Concurrent Connections | 1M | 500k | 100k+ |
|Healing Time | N/A | <5s | <10s |

## Configuration Examples

### Development Configuration

```rust
// Relaxed limits for testing
CageConfig {
    memory_limit_bytes: 256 * 1024 * 1024,  // 256MB
    cpu_timeout_ms: 5000,                    // 5 seconds
    allow_filesystem: true,
    allow_network: true,
}
```

### Production Configuration

```rust
// Strict limits for security
CageConfig {
    memory_limit_bytes: 64 * 1024 * 1024,   // 64MB
    cpu_timeout_ms: 500,                     // 500ms
    allow_filesystem: false,
    allow_network: false,
}
```

## Security Considerations

1. **Wasm Sandboxing**: Each Cage is isolated with no host access
2. **Resource Limits**: Memory and CPU strictly enforced
3. **AI Monitoring**: Real-time anomaly detection
4. **State Isolation**: CRDT sync uses secure channels
5. **No Unsafe Code**: All business logic in safe Rust

## Future Enhancements (Phase 3)

- Zero-RTT HTTP/3 connections
- Advanced AI threat classification
- Dynamic Cage scaling based on load
- Multi-tenant site isolation
- Distributed CRDT across servers
- GPU-accelerated AI inference

---

**Pear Server** - Built with 🦀 Rust for maximum performance, safety, and reliability.
