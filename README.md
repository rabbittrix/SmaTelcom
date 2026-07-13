# SmaTelcom

**Local-first Desktop AI Orchestrator for Telecommunications Network Management**  
*(Autonomous Networks Level 4)*

| | |
|---|---|
| **Author** | Roberto de Souza |
| **Email** | [rabbittrix@hotmail.com](mailto:rabbittrix@hotmail.com) |
| **Stack** | Tauri v2 · Rust · React · TypeScript · Tailwind CSS · Ollama |
| **Privacy** | Localhost-only inference & data plane |

---

## Vision

SmaTelcom is a mission-critical desktop orchestrator that analyzes **network intents** with a multi-agent SLM pipeline, grounds answers in local manuals (RAG), validates every proposed command with a **deterministic Rust Safety Linter**, and escalates risky actions through a **Human-in-the-Loop (HITL)** workflow with graduated autonomy.

> **Architectural note:** The UI is **Vite + React + TypeScript** (not Next.js). Tauri desktop apps ship a static SPA into a native WebView; Next.js SSR/App Router adds complexity without benefit for a local-first offline desktop product. Tailwind, Lucide, and Framer Motion are used as specified.

---

## Core Features

### 1. Multi-Agent Decision Pipeline
- **Performance Agent** — latency, throughput, QoS, capacity  
- **Security Agent** — threats, ACL integrity, blast radius  
- **Topology Agent** — path diversity, site roles, failover  
- **Judge Agent** — synthesizes a single recommendation + operational command  
- Inference via **Ollama** at `http://127.0.0.1:11434` (Phi-3 / Mistral preferred)

### 2. Safety Linter (Rust)
Deterministic regex blacklist — **no LLM** — runs **before** HITL:
- Hard-block: `shutdown core_router`, `delete config`, factory reset, disable firewall, etc.
- Graduated risk: Low → auto-approve · Medium/High → HITL · Critical blacklist → blocked

### 3. Human-in-the-Loop
Critical/complex actions surface a high-visibility modal with:
- Decision logic  
- Risk assessment (Low / Medium / High / Critical)  
- **Approve** / **Reject**

### 4. Dashboard UX
- Dark / light theme (Linear / Vercel–inspired technical aesthetic)  
- Sidebar navigation  
- Live **Network Health** from Rust telemetry simulator  
- **Activity Log** of agent reasoning  
- Framer Motion transitions  

### 5. Telemetry Simulation
Rust thread emits mock JSON network events every **5 seconds** for the AI loop.

### 6. Local RAG
Reads `knowledge_base/` (`.txt`, `.md`, `.pdf`), chunks content, and injects top passages into agent prompts.

---

## Repository Structure

```
SmaTelcom/
├── assets/                      # Brand: logo.png, favicon.png
├── knowledge_base/              # Local manuals for RAG
│   ├── ran_congestion_playbook.txt
│   ├── core_safety_policy.txt
│   └── topology_reference.txt
├── public/                      # Static web assets (favicon, logo)
├── src/                         # React frontend
│   ├── components/
│   │   ├── dashboard/           # NetworkHealth, ActivityLog
│   │   ├── hitl/                # CriticalAlert modal
│   │   └── layout/              # Sidebar
│   ├── hooks/                   # Theme provider
│   ├── lib/                     # Tauri IPC + shared types
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css                # Tailwind v4 + design tokens
├── src-tauri/                   # Rust / Tauri backend
│   ├── icons/                   # Native app icons
│   ├── src/
│   │   ├── agents.rs            # Multi-agent + Judge pipeline
│   │   ├── commands.rs          # Tauri IPC commands
│   │   ├── guardrails.rs        # Deterministic Safety Linter
│   │   ├── ollama.rs            # Localhost Ollama client
│   │   ├── rag.rs               # Knowledge-base loader + retrieval
│   │   ├── telemetry.rs         # 5s mock network simulator
│   │   ├── error.rs
│   │   ├── lib.rs
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
├── vite.config.ts
├── index.html
├── .gitignore
└── README.md
```

---

## Prerequisites

1. **Node.js** 20+ and npm  
2. **Rust** stable (1.77+) via [rustup](https://rustup.rs)  
3. **Tauri system deps** — [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (Windows: WebView2)  
4. **Ollama** — [https://ollama.com](https://ollama.com)

### Pull a local model

```bash
ollama pull phi3
# or
ollama pull mistral
```

Confirm the API:

```bash
curl http://127.0.0.1:11434/api/tags
```

---

## Quick Start

```bash
# 1. Install JS dependencies
npm install

# 2. Ensure Ollama is running with a model (separate terminal)
ollama serve
ollama pull phi3

# 3. Launch desktop app (Vite + Rust)
npm run tauri:dev
```

Frontend-only (no Rust IPC — limited):

```bash
npm run dev
```

Production build:

```bash
npm run tauri:build
```

---

## Tauri Commands (IPC)

| Command | Purpose |
|---|---|
| `check_ollama` | Health-check localhost Ollama |
| `list_models` | List installed models |
| `get_telemetry_snapshot` | Latest simulated health + events |
| `analyze_network_intent` | Full multi-agent pipeline + safety lint |
| `lint_command` | Run Safety Linter alone |
| `approve_action` / `reject_action` | HITL outcomes |
| `reload_knowledge_base` / `search_knowledge` | RAG management |

---

## Safety & Autonomy Model

```
Intent → RAG context → 3 Specialist Agents → Judge Agent
       → Rust Safety Linter (deterministic)
       → Auto-approve (Low) | HITL modal (Medium+) | Hard block (Critical blacklist)
```

Memory safety and isolation are enforced by Rust ownership, `rustls` HTTPS/TLS stack for HTTP client, and **localhost-only** CSP / Ollama base URL (`127.0.0.1:11434`).

---

## Brand Assets

| File | Use |
|---|---|
| `assets/logo.png` / `public/logo.png` | App logo (sidebar, marketing) |
| `assets/favicon.png` / `public/favicon.png` | Browser / window favicon |
| `src-tauri/icons/*` | Native installer & window icons |

To regenerate Tauri icons from the logo:

```bash
npm run tauri icon ./assets/logo.png
```

---

## Configuration Notes

- Window defaults: 1440×900, title `SmaTelcom — AI Network Orchestrator`
- CSP allows only `self` + Ollama localhost
- Telemetry interval: **5 seconds** (`telemetry.rs`)
- Knowledge root: `knowledge_base/` (resolved relative to app / `src-tauri`)

---

## Development Roadmap (post-MVP)

- [ ] Persist HITL decisions to encrypted local store  
- [ ] Streaming Ollama tokens into Activity Log  
- [ ] Embeddings-based RAG (local vector index)  
- [ ] Northbound adapters (NETCONF / gNMI) behind safety gate  
- [ ] Signed audit trail for AN Level-4 compliance  

---

## License

**Private · Apache-style · Author Authorization required** — not MIT / not public open source.

© 2026 Roberto de Souza — `rabbittrix@hotmail.com`

- Full terms: [`LICENSE`](./LICENSE)
- Authorization & payment rules (SEPA QR): [`LICENSES/LICENSE_RULES.md`](./LICENSES/LICENSE_RULES.md)

| Field | Value |
|---|---|
| IBAN | `PT50 3560 0001 9001 8573 6595 0` |
| BIC/SWIFT | `REVOPTP2` |
| Beneficiary | Roberto de Souza |

Unauthorized use, redistribution, or deployment is prohibited.

---

## Disclaimer

SmaTelcom is an **MVP research / lab orchestrator**. Do not connect to production network elements without a formal safety case, change-management process, operator certification, and **Author Authorization**. The Safety Linter reduces risk; it does not eliminate it.
