# MSP Query Engine — Startup-Grade Infrastructure Blueprint

## 1. System Overview

MSP is a **centralized retrieval and knowledge orchestration service** powering EVA agents.

Flow:
User → EVA → MSP API → Retrieval Pipeline → Context Builder → EVA → Response

---

## 2. High-Level Architecture

### Core Services

1. **API Service (Node.js / Fastify)**
   - /query
   - /index
   - /feedback
   - /health

2. **Worker Service (Queue-based)**
   - Indexing
   - Embedding
   - Graph building

3. **Vector DB**
   - Supabase / Pinecone

4. **Metadata DB**
   - PostgreSQL

5. **Cache Layer**
   - Redis

6. **Storage**
   - GKS (Obsidian Vault / Git-backed)

---

## 3. Data Flow

### Query Flow

1. EVA sends query
2. MSP API validates + normalizes
3. Cache check (Redis)
4. Retrieval Pipeline:
   - Semantic (vector DB)
   - Keyword (BM25)
   - Graph traversal
5. Ranking engine
6. Context builder
7. Return context + nodes

---

### Indexing Flow

1. File change (Git hook / watcher)
2. Push job → Queue
3. Worker:
   - Parse markdown
   - Chunk
   - Embed
   - Update vector DB
   - Extract graph
   - Update graph store

---

## 4. Services Breakdown

### API Service
- Stateless
- Horizontal scalable
- Handles auth, rate limit

### Worker
- Async jobs
- Retry + dead-letter queue

---

## 5. Tech Stack (Recommended)

- API: Node.js (Fastify)
- Queue: BullMQ (Redis)
- DB: PostgreSQL
- Vector: Supabase / pgvector
- Cache: Redis
- Infra: Docker + Fly.io / AWS

---

## 6. Retrieval Pipeline

Hybrid scoring:

score =
  0.5 * semantic +
  0.25 * keyword +
  0.15 * graph +
  0.1 * recency

---

## 7. Context Builder

- Token budget aware
- Deduplicate
- Merge nodes
- Optional summarization layer

---

## 8. Feedback Loop

- Store query + result
- Track clicks / follow-ups
- Adjust ranking weights

---

## 9. Observability

- Logs (query, latency)
- Metrics (p95 latency, hit rate)
- Tracing (OpenTelemetry)

---

## 10. Deployment

- Dockerized services
- CI/CD (GitHub Actions)
- Environments: dev / staging / prod

---

## 11. Scaling Strategy

- Scale API horizontally
- Scale workers independently
- Partition vector index

---

## 12. Future Extensions

- Multi-tenant support
- Agent-specific memory profiles
- Real-time collaboration

---

## 13. Key Principle

MSP is NOT a tool.
It is a **core intelligence layer** controlling how knowledge is retrieved and used.

