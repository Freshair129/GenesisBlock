# Tier-C Competitive Field: On-Device + Sync (research date 2026-07-06)

Skeptical-analyst read of what each player **actually ships** as of July 2026, along the axes GenesisBlockDB's Tier-C buyer (on-device flagship agent: local hot store + privacy-curated export + conflict-free sync) cares about. All claims cited; items I could not confirm from a primary source are marked **UNVERIFIED**.

---

## 1. Ditto — P2P CRDT mesh, mobile-first

- **Embedded/on-device**: Yes, in-process embedded document store inside the app. SDKs for iOS, Android, Linux, Windows (also Flutter/RN/C#/C++/Rust surfaces). ([docs.ditto.live/sdk/latest/sync/syncing-data](https://docs.ditto.live/sdk/latest/sync/syncing-data), 2026-07-06)
- **Sync mechanism**: True peer-to-peer mesh (Bluetooth LE, LAN, Wi-Fi Aware, plus cloud "Big Peer") over a CRDT document model — this is the most genuinely CRDT-native shipping product in the field. ([businessmodelcanvastemplate.com Ditto analysis](https://businessmodelcanvastemplate.com/blogs/how-it-works/ditto-how-it-works), 2026; [docs.ditto.live](https://docs.ditto.live/sync/syncing-data))
- **Conflict handling**: CRDT merge (register/map/counter semantics), no server arbiter required.
- **Partial/selective sync**: Strong — sync **subscriptions** are declarative DQL queries registered *on the device*; only matching documents replicate to that peer. This is genuinely device-controlled inbound partial sync. ([docs.ditto.live subscriptions](https://docs.ditto.live/sdk/v4-7/sync/subscriptions-management), 2026-07-06.) Caveat for the privacy axis: subscriptions control what a device *receives*; controlling what *leaves* the device is coarser (collection/document-level, `EvictionPolicy`/local-only collections) — a curated-export pipeline is not a first-class primitive. **Partially UNVERIFIED** (outbound-filtering granularity not confirmed from primary docs this pass).
- **On-device vector**: **No.** No vector index in the SDK as of this research date.
- **Graph**: No. Document model only.
- **Temporal/versioning**: No time-travel/point-in-time query surface; CRDT metadata is internal.
- **License/pricing**: Fully commercial (closed source). Per-device/peer subscription tiers plus consumption-based Big Peer cloud charges; volume breaks reported around 5k/10k devices (secondary source — treat exact numbers as **UNVERIFIED**). ([ditto.live/pricing/cloud-sync](https://ditto.live/pricing/cloud-sync); [AWS Marketplace listing](https://aws.amazon.com/marketplace/pp/prodview-axv2ggb5yy5za), 2026)
- **2026 status**: Active, expanding into vertical modules (retail, aviation). The strongest "conflict-free sync" competitor, but zero vector/graph story.

## 2. ObjectBox — embedded NoSQL + on-device vectors + commercial Sync

- **Embedded/on-device**: Yes, in-process object database; SDKs for Java/Kotlin, Swift, Dart/Flutter, C/C++, Go, Python. ([objectbox.io](https://objectbox.io/), 2026-07-06)
- **On-device vector**: **Yes — the closest analogue to GenesisBlockDB's vector story.** HNSW ANN index on-device since ObjectBox 4.0 ("first on-device vector database"), still actively tuned (2026 releases note HNSW perf work on Linux ARM). ([docs.objectbox.io/on-device-vector-search](https://docs.objectbox.io/on-device-vector-search); [objectbox-java releases](https://github.com/objectbox/objectbox-java/releases), 2026)
- **Sync mechanism**: ObjectBox Sync — client/server, server-authoritative offline sync (Sync protocol v10 in 2026 releases); MongoDB Sync Connector GA Oct 2025 (bi-directional ObjectBox↔MongoDB Atlas). **Not CRDT**; conflict handling is last-write/server-mediated. ([sync.objectbox.io/mongodb-sync-connector](https://sync.objectbox.io/mongodb-sync-connector); [ObjectBox 5.0 announcement](https://objectbox.io/user-specific-data-sync-mongodb-connector-objectbox-5-0-is-here/), 2025-10)
- **Partial/selective sync**: "User-specific data sync" (ObjectBox 5.0) and selective sync of chosen entities — but rules live in **server config**, not device-side curation. ([objectbox.io/sync](https://objectbox.io/sync/), 2026)
- **Graph**: Object relations (to-one/to-many links), not a graph query engine. No traversal language, no bitemporal.
- **Temporal/versioning**: None exposed.
- **License/pricing**: Bindings Apache-2.0; the **native core is closed source** (ObjectBox Binary License, free to use); **Sync is paid** (subscription, quote-based). ([objectbox.io/faq](https://objectbox.io/faq/); [sync pricing page](https://objectbox.io/sync-pricing/), 2026-07-06)
- **2026 status**: Active; positioning itself explicitly as "edge vector database" and as the Realm-refugee landing pad. The most direct Tier-C feature overlap, minus graph/temporal/CRDT.

## 3. PowerSync — SQLite↔Postgres (also MongoDB/MySQL/SQL Server) sync service

- **Embedded/on-device**: Client SDKs maintain a local in-app SQLite DB (Flutter, RN, JS/web, Kotlin, Swift, .NET). In-process via SQLite. ([powersync.com](https://powersync.com/), 2026-07-06)
- **Sync mechanism**: Server-side sync service consuming Postgres logical replication, streaming row changes into client SQLite; client writes go into an upload queue processed by **your backend API** (developer-defined write path). Not CRDT. ([powersync.com/sync-postgres](https://powersync.com/sync-postgres); [v1.0 announcement](https://powersync.com/blog/introducing-powersync-v1-0-postgres-sqlite-sqlite-sync-layer) — canonical: [blog post](https://powersync.com/blog/introducing-powersync-v1-0-postgres-sqlite-sync-layer))
- **Conflict handling**: Effectively server-authoritative/developer-defined at the upload endpoint (LWW by default in your backend logic).
- **Partial/selective sync**: **Sync Rules** — YAML bucket definitions, evaluated **server-side**. Powerful partitioning (per-user buckets etc.), hot-swappable, but the *server* decides what each device gets; the device does not curate what leaves it beyond which tables/queues the app writes. ([docs.powersync.com](https://docs.powersync.com/client-sdks/advanced/pre-seeded-sqlite); [powersync.com](https://powersync.com/), 2026)
- **On-device vector**: Not native; only whatever you bolt onto SQLite yourself (e.g., sqlite-vec) — no supported vector index product.
- **Graph / temporal**: None.
- **License/pricing**: Sync service open source (self-host "Open Edition", Docker `journeyapps/powersync-service`; 2026 added Postgres-backed bucket storage for self-hosting). Cloud: free tier (2 GB synced/mo, 50 connections), Pro $49/mo, Team $599/mo, Enterprise. ([powersync.com pricing per QueryPlane review](https://queryplane.com/blog/powersync-offline-first-sync/); [PowerSync GitHub](https://github.com/powersync-ja); [releases](https://releases.powersync.com/announcements/powersync-service), 2026)
- **2026 status**: Active, shipping steadily (Service v1.22.x line), the pragmatic "keep Postgres authoritative" choice. Biggest 2026 beneficiary of Realm's death alongside ObjectBox/Couchbase.

## 4. Turso / libSQL — embedded replicas → Rust-rewrite offline sync

Two distinct artifacts; conflating them flatters the marketing:

- **libSQL (C fork of SQLite, production)**: embedded replicas — local DB kept in sync with Turso Cloud, page-frame based; historically read-local/write-forwarded. **Native vector search shipped** (vector column types + LM-DiskANN index, FLOAT32/16/BF16/1-bit quantization, no extension) — proven on-device by consumers like Kin (personal-AI app doing all vector search on device). ([turso.tech/vector](https://turso.tech/vector); [AI & embeddings docs](https://docs.turso.tech/features/ai-and-embeddings); [vector announcement](https://turso.tech/blog/turso-brings-native-vector-search-to-sqlite), fetched 2026-07-06)
- **Turso Database (Rust rewrite, beta)**: full rewrite of SQLite in Rust with MVCC and async I/O; **Turso Sync / Offline Sync in public beta** — local writes at file speed, offline, explicit `push()`/`pull()`, logical CDC instead of page frames, partial-sync bootstrap. Beta SDK coverage: TypeScript + Rust. DiskANN vector search in the rewrite is tracked but **not complete** ([tursodatabase/turso issue #832](https://github.com/tursodatabase/turso/issues/832)). ([Offline Sync public beta](https://turso.tech/blog/turso-offline-sync-public-beta); [Turso Sync benchmark post](https://turso.tech/blog/sync-benchmark); [Databases Anywhere](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync), 2026)
- **Conflict handling**: sync is hub-and-spoke to Turso Cloud; CDC-log replay, conflicts effectively LWW/server-arbitrated — **not CRDT**. Fine-grained semantics of concurrent-write merge in the beta: **UNVERIFIED**.
- **Partial/selective sync**: partial sync = faster bootstrap (sync what you touch), not a privacy-curation surface; what leaves the device is "your writes," unfiltered.
- **Mobile SDKs**: libSQL has Swift/Kotlin/RN bindings; the Rust-rewrite mobile story is early. Graph/temporal: none.
- **License/pricing**: MIT open source (both libSQL and the rewrite); Turso Cloud is the paid service. ([github.com/tursodatabase](https://github.com/tursodatabase), 2026)
- **2026 status**: Explicitly repositioning as "the SQLite for the agentic era" — a database-per-agent pitch aimed near our Tier-C. Watch closely; but its sync is beta and centralized, and it has no graph or bitemporal layer.

## 5. ElectricSQL — Postgres read-path sync, "shapes"

- **What it is now**: after the 2024 rewrite, Electric is a **read-path-only** sync engine: it consumes Postgres logical replication and fans rows out into **Shapes** (a SQL query per shape) over HTTP to any client. **It does not do write-path sync at all** — writes go through your backend; patterns (optimistic state, through-the-DB) are documented, not shipped. ([electric-sql.com/docs/guides/writes](https://electric-sql.com/docs/guides/writes); [github.com/electric-sql/electric](https://github.com/electric-sql/electric), fetched 2026-07-06)
- **Embedded/on-device**: Not an embedded DB; clients sync shapes into whatever local store you choose (TanStack DB, PGlite, etc.). Mobile SDK story thin (JS-first; community Dart client exists: [pub.dev electricsql](https://pub.dev/documentation/electricsql/latest/)).
- **Conflict handling**: N/A — no write path, so no conflict semantics of its own.
- **Partial sync**: Shapes are exactly partial sync, but defined by the app/server tier (where clauses on tables), not a device-privacy control.
- **Vector/graph/temporal**: none.
- **License/pricing**: Apache-2.0 OSS; Electric Cloud hosted service. Notably now marketing itself as "the agent platform built on sync" (repo tagline), i.e., agent-state sync for cloud agents rather than on-device. ([github.com/electric-sql/electric](https://github.com/electric-sql/electric), 2026)
- **2026 status**: Active (recent "reliability sprint"). Not a direct Tier-C threat (no device store, no writes), but a Tier-B-adjacent idea competitor.

## 6. cr-sqlite (vlcn) — CRDT SQLite extension

- **Status: effectively dormant.** Last release **v0.16.3, January 2024** — ~2.5 years old at research date; no maintenance notice, 3.7k stars, MIT. Backers (Turso, Fly.io) moved on; the founder's follow-on work went elsewhere. ([github.com/vlcn-io/cr-sqlite](https://github.com/vlcn-io/cr-sqlite), fetched 2026-07-06; [releases page](https://github.com/vlcn-io/cr-sqlite/releases))
- **What it was**: loadable SQLite extension turning tables into CRRs (conflict-free replicated relations: column-level LWW + causal-length delete tracking), multi-master merge via `crsql_changes` virtual table. Technically the closest OSS prior art to "CRDT relational sync," in-process, any-platform SQLite.
- **Takeaway**: proof the demand exists, not a shipping competitor. Anyone building on it in 2026 owns the maintenance. No vector, no graph, no temporal, and the sync *transport* was always DIY.

## 7. Couchbase Lite + Capella App Services

- **Embedded/on-device**: Yes — mature in-process embedded NoSQL (iOS, Android, .NET, C, Java, Kotlin, Flutter/RN via community); also P2P device-to-device sync. ([couchbase.com/products/lite](https://www.couchbase.com/products/lite/), 2026-07-06)
- **Vector search on-device: YES, landed.** Couchbase Lite ships native vector search (beta 2024 → GA in the 3.2 line), queryable via SQL++, explicitly pitched for on-device RAG/privacy ("queries never leave the device"). Shipped as a separate vector-search library and — per docs — an **Enterprise Edition feature**. ([docs.couchbase.com Couchbase Lite vector search](https://docs.couchbase.com/couchbase-lite/current/android/vector-search.html); [couchbase.com blog: vector search at the edge](https://www.couchbase.com/blog/vector-search-at-the-edge-with-couchbase-mobile/); GA-version precise date **UNVERIFIED** beyond "current docs mark it Enterprise")
- **Sync mechanism**: Sync Gateway / Capella App Services — **server-authoritative sync with channels**; plus true P2P replication between Lite instances. Not CRDT: conflict handling = revision trees with default LWW + custom conflict resolvers. Sync Gateway 4.1 (2025/26) added rolling upgrades, distributed resync. ([couchbase.com/blog P2P sync](https://www.couchbase.com/blog/mobile-peer-to-peer-data-sync/); [docs.couchbase.com](https://docs.couchbase.com/couchbase-lite/current/index.html))
- **Partial/selective sync**: **Channels** — documents are assigned to channels, users/devices get access-filtered subsets. Powerful, but access rules are defined at the Sync Gateway (server), not curated by the device. Device-side replication filters exist for push (deprecated in favor of channels in recent versions — **UNVERIFIED** current status).
- **Graph / temporal**: none (document + SQL++; no traversal engine, no time-travel).
- **License/pricing**: Lite CE is free (Couchbase community license — not OSI-open since the BSL-style relicensing); vector search and enterprise sync features require commercial Enterprise/Capella subscription. ([couchbase.com/pricing](https://www.couchbase.com/pricing/))
- **2026 status**: Active and aggressively harvesting Realm refugees ([their EOL-day migration post](https://www.couchbase.com/blog/realm-mongodb-eol-day-2025/)). **The most complete incumbent on the Tier-C checklist** (embedded + mobile SDKs + on-device vector + managed sync) — its gaps are exactly ours to exploit: no CRDT, server-defined partial sync, no graph, no bitemporal, and vector is paywalled EE.

## 8. Realm / MongoDB Atlas Device Sync — dead

- **Sunset confirmed and executed**: deprecation announced Sept 2024; **Atlas Device Sync, Atlas Device SDKs (Realm), Data API, and Edge Server hit end-of-life 2025-09-30**. ([MongoDB forums EOL notice](https://www.mongodb.com/community/forums/t/atlas-device-sync-end-of-life-and-deprecation/296687); [update notice](https://www.mongodb.com/community/forums/t/update-to-end-of-life-and-deprecation-notice/297168), fetched 2026-07-06)
- The client-side Realm DB source remains on GitHub **unmaintained by MongoDB** ([realm-swift discussion #8680](https://github.com/realm/realm-swift/discussions/8680)); community forks exist but none has emerged as a credible maintained successor (**UNVERIFIED** — no dominant fork found this pass).
- **Market consequence (the relevant fact)**: the only mainstream mobile object-DB-with-managed-sync vacated the field in Sept 2025. Ditto, ObjectBox, PowerSync, Couchbase, and RxDB are all running migration-capture campaigns ([ObjectBox](https://objectbox.io/alternative-to-mongodb-sync/), [Couchbase](https://www.couchbase.com/blog/realm-mongodb-eol-day-2025/), [RxDB](https://rxdb.info/articles/alternatives/mongodb-realm-alternative.html)). Tier-C buyers burned by a vendor sunset are now structurally biased toward OSS/self-hostable sync — a real wedge for us.

## 9. RxDB / WatermelonDB (JS-side, brief)

- **RxDB**: reactive NoSQL JS database, storage-pluggable (IndexedDB/SQLite/OPFS), replication adapters (CouchDB, Supabase, Firestore, custom HTTP). Conflicts: LWW default, customizable handler. Core Apache-2.0 + **paid Premium plugins** (e.g., fast Expo/OPFS storage). No native vector index (docs show DIY embedding-in-documents patterns), no graph, no temporal. ([rxdb.info/premium](https://rxdb.info/premium/); [rxdb.info/react-native-database](https://rxdb.info/react-native-database.html), 2026)
- **WatermelonDB**: React-Native-focused SQLite ORM with a sync *protocol*, not a sync service — server decides conflict resolution entirely (adapter pattern; three-way merge possible if your server tracks ancestors). MIT. No vector/graph/temporal. ([PkgPulse 2026 comparison](https://www.pkgpulse.com/blog/tinybase-vs-watermelondb-vs-rxdb-offline-first-2026))
- Relevance: framework-level DX competitors for JS apps, not engine competitors; neither addresses vector, graph, or curated export.

## 10. Apple / Google platform-native (brief)

- **SwiftData + CloudKit**: built-in iCloud private-database sync; still **no public/shared DB support** via SwiftData, sync is opaque (no custom conflict logic; CloudKit server-record-change-tag/LWW under the hood). iOS 26 brought SwiftData to "production parity" (model inheritance, history fetch sortBy) — persistence maturity, **nothing agent-memory-shaped**. ([developer.apple.com SwiftData+CloudKit](https://developer.apple.com/documentation/swiftdata/syncing-model-data-across-a-persons-devices); [techkodainya iOS 26 overview](https://www.techkodainya.com/blogs/scalable-ios-apps), 2026)
- **Apple on-device AI**: Foundation Models framework (iOS 26) gives on-device LLM inference, but Apple ships **no on-device vector store or agent-memory database API** — third parties (incl. Point-Free's SQLite+CloudKit alpha, [pointfree.co](https://www.pointfree.co/blog/posts/179-a-swiftdata-alternative-with-sqlite-cloudkit-private-alpha)) are filling the gap. Any Apple/Google first-party "agent memory" store as of 2026-07: **UNVERIFIED / not found**.
- **Google**: Room/SQLite + Firestore offline remain the defaults; AICore/Gemini Nano provide inference, not memory. Same gap.
- Relevance: platform vendors validate on-device inference but leave the *memory substrate* open — which is precisely the Tier-C opportunity.

---

## Axis table

| Player | In-process embedded | Mobile SDKs | CRDT / conflict-free | Device-controlled partial sync | On-device vector | Graph | Temporal / versioning |
|---|---|---|---|---|---|---|---|
| **Ditto** | Yes | Yes (iOS/Android/Flutter/RN/C++/Rust) | **Yes** (true CRDT, P2P mesh) | **Partial-Yes** — device-declared DQL subscriptions (inbound); outbound curation coarse | No | No | No |
| **ObjectBox** | Yes | Yes (Java/Swift/Dart/C/Go) | No (server-mediated sync) | Partial — selective/user-specific sync, but **server-configured** | **Yes** (HNSW) | Relations only, no traversal engine | No |
| **PowerSync** | Via SQLite | Yes (Flutter/RN/Kotlin/Swift/.NET/JS) | No (upload queue → your backend, LWW) | No — Sync Rules are **server-side YAML** | No (DIY sqlite-vec) | No | No |
| **Turso / libSQL** | Yes (SQLite-class) | libSQL: yes; Rust rewrite: TS+Rust beta | No (CDC to cloud hub, beta) | No (partial sync = bootstrap perf, not curation) | **Yes** (libSQL DiskANN, native); rewrite: in progress | No | No |
| **ElectricSQL** | No (sync engine, not a DB) | Weak (JS-first) | No (read-path only; **no write path at all**) | Shapes = partial sync, but app/server-defined | No | No | No |
| **cr-sqlite** | Yes (SQLite ext.) | Wherever SQLite runs | **Yes** (CRR, column-LWW) | DIY (transport not included) | No | No | No — and **dormant since 2024-01** |
| **Couchbase Lite + App Services** | Yes | Yes (broad) | No (rev-trees, LWW + custom resolvers; has P2P) | Channels = **server-defined** access filtering | **Yes** (SQL++ vector, **Enterprise-only**) | No | Revision history internal, no time-travel API |
| **Realm / Atlas Device Sync** | (was) Yes | (was) Yes | No | No | No | No | **DEAD — EOL 2025-09-30** |
| **RxDB / WatermelonDB** | JS-layer over storage | RN/Expo/web | No (LWW / server-decides) | Adapter-dependent, developer-built | No | No | No |
| **SwiftData+CloudKit / Google** | Yes (platform) | Platform-native only | No (opaque CloudKit LWW) | No (all-or-nothing private DB) | No (Foundation Models = inference only) | No | Limited (SwiftData history fetch) |
| **GenesisBlockDB (reference)** | Yes (Rust core, FFI/NAPI) | iOS/Android FFI (Mark XVI; iOS still stub) | Yes (CRDT/LWW + signed events) | Target axis: privacy-curated export | Yes (per-collection HNSW) | **Yes** | **Yes (bitemporal)** |

## Analyst bottom line

1. **Nobody ships the full Tier-C stack.** The field partitions cleanly: CRDT-without-vector (Ditto), vector-without-CRDT (ObjectBox, Couchbase Lite, libSQL), and sync-plumbing-without-a-database (PowerSync, Electric). Graph and temporal/point-in-time are at **zero across all ten** — GenesisBlockDB's bitemporal + graph axes are uncontested in this field (the contest for those lives in the memory-layer space — Graphiti et al. — not here).
2. **"Device-controlled partial sync" is nearly vacant.** Every partial-sync mechanism shipping today (PowerSync sync rules, Electric shapes, Couchbase channels, ObjectBox user-sync) is *server*-defined. Only Ditto's subscriptions are device-declared, and even those govern what arrives, not a curated export of what leaves. The privacy-curated-export axis is a real differentiator — provided it's an actual shipping primitive, not a slide.
3. **Realm's Sept-2025 death is the market event.** A vendor-sunset-burned installed base is migrating now and is allergic to closed commercial sync; Couchbase (EE-paywalled vector) and ObjectBox (closed core, paid sync) both carry that allergy trigger.
4. **Closest single threats**: Couchbase Lite (most complete incumbent, enterprise motion) and Turso (OSS momentum + explicit "agentic era" positioning + native vector; sync still beta/centralized). Ditto owns the CRDT narrative but has no AI-retrieval story at all.