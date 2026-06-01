# ADR--PHASE-13-WAL-GROUP-COMMIT

## 1. Status
**Draft / Proposed**

## 2. Context
The Phase 12 Scientific Audit revealed that enforcing strict \sync()\ on every individual write operation creates a massive I/O bottleneck, collapsing throughput to ~139 TPS under 12-thread contention on consumer NVMe hardware. To achieve high durable throughput (e.g., matching RocksDB's 50k+ TPS) while maintaining ACID compliance, GenesisDB requires a mechanism to amortize the cost of \sync\ across multiple concurrent transactions.

## 3. Decision
We will implement a **WAL Group Commit** architecture.

## 4. Architectural Design
- **Event Channel:** Concurrent writers will no longer lock the \BufWriter\ directly. Instead, they will serialize their mutations and send them over a high-performance, lock-free channel (e.g., \crossbeam-channel\ or \lume\).
- **Dedicated Flusher Thread:** A single, dedicated background thread (The "WAL Flusher") will consume events from the channel.
- **Batching Strategy:** The Flusher will accumulate events into an in-memory buffer until either:
  a) A predefined batch size is reached (e.g., 4MB).
  b) A predefined time window expires (e.g., 5 milliseconds).
- **Single Fsync:** Once a batch condition is met, the Flusher writes the entire buffer to disk and issues a single \sync_all()\ call.
- **Acknowledgment Mechanisms:** Writers will await an acknowledgment (via a one-shot channel) from the Flusher before returning \Ok(())\ to the caller, ensuring the client only receives confirmation after true physical durability is achieved.

## 5. Consequences
- **Pros:** Massively increases durable write throughput by amortizing NVMe latency. Eliminates thread contention on the file descriptor.
- **Cons:** Increases code complexity. Requires careful channel management to avoid deadlocks or unbound memory growth if the disk subsystem stalls.
