---
id: FEAT--SYMBOLS-FRAMEWORK-AWARENESS
phase: 2
type: feat
status: superseded
tier: process
source_type: axiomatic
vault_id: default
title: Framework-aware symbol indexing — recognizing API routes, ORM models, and MCP tools
tags:
  - msp
  - symbol-graph
  - framework
  - nextjs
  - app-router
  - server-components
  - prisma
  - orm
  - route
  - mcp
crosslinks: {"implements":["ADR--SYMBOL-GRAPH-PERSISTENCE"],"references":["FRAME--SYMBOL-GRAPH","CONCEPT--SYMBOL-GRAPH","FEAT--MSP-SYMBOL-MCP"],"superseded_by":["CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS","ADR--SYMBOLS-FRAMEWORK-AWARENESS","ALGO--SYMBOLS-FRAMEWORK-RECOGNITION","PROTO--SYMBOLS-FRAMEWORK-INVARIANTS"]}
linked_symbols:
  - {"file":"packages/msp/src/symbols/parser/framework.ts"}
  - {"file":"packages/msp/src/symbols/parser/orm.ts"}
created_at: 2026-05-12T05:48:00.000+07:00
---

# FEAT — Framework-aware symbol indexing

> ⚠️ **Superseded on 2026-05-11.** This FEAT bundled four concerns (motivation, decision, algorithms, invariants) which violates atom-type semantics per `KNOWLEDGE-TYPES.md`. It has been decomposed into:
>
> - `CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS` (motivation, scope)
> - `ADR--SYMBOLS-FRAMEWORK-AWARENESS` (decision + decomposition rationale)
> - `ALGO--SYMBOLS-FRAMEWORK-RECOGNITION` (algorithms per recognizer)
> - `PROTO--SYMBOLS-FRAMEWORK-INVARIANTS` (validator-enforced graph invariants)
>
> See `ADR--SYMBOLS-FRAMEWORK-AWARENESS` for rationale. Implementation work continues per `HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md`.
>
> The original content below is preserved verbatim for historical reference.

---

## User-facing behaviour

ยกระดับความฉลาดของ Symbol Graph ให้เข้าใจ "หน้าที่" ของโค้ดในบริบทของ Framework ต่างๆ ไม่ใช่แค่โครงสร้าง Syntax:

1. **Route Mapping (Stage 6):** ระบุและสร้างโหนดประเภท `Route` สำหรับ API/Web endpoints อัตโนมัติ (เช่น Next.js `app/**/route.ts` หรือ `api/` routes) พร้อมเชื่อมโยง `HANDLES` edge ไปยัง handler function
2. **MCP Tool Discovery (Stage 7):** สแกนหาคำนิยามของ MCP tools (เช่น โค้ดที่ใช้ `@mcp/tool` หรือ handler objects) และสร้างโหนดประเภท `Tool` พร้อมเชื่อมโยงกับฟังก์ชันที่ทำหน้าที่ประมวลผลจริง
3. **ORM Model Resolution (Stage 8):** วิเคราะห์ไฟล์ Schema (เช่น Prisma `.prisma` หรือ Drizzle models) เพื่อสร้างโหนดประเภท `Entity` และเชื่อมโยงความสัมพันธ์ไปยัง Repository layer ในโค้ด ทำให้เห็นภาพ "Data-to-Code" ชัดเจน

เมื่อ Agent ใช้ `msp_symbol_lookup` หรือ `msp_symbol_neighbors` จะเห็นความสัมพันธ์เชิงหน้าที่ (Functional links) ทันที เช่น "Endpoint นี้ถูกจัดการโดยฟังก์ชัน X และเขียนข้อมูลลงใน Entity Y"

### 1b. Next.js App Router refinements (สำคัญสำหรับโปรเจกต์ Next.js สมัยใหม่)

นอกเหนือจาก "Route detection" แบบทั่วไป ต้องเข้าใจโครงสร้างเฉพาะของ Next.js App Router:

**Routing Awareness — แยกประเภท node ตาม file convention:**

| File pattern | Node kind | Edge ที่ควรมี |
|---|---|---|
| `app/**/page.tsx` | `Page` | `RENDERS_AT` → URL path |
| `app/**/layout.tsx` | `Layout` | `WRAPS` → children pages |
| `app/**/loading.tsx` | `LoadingState` | `FALLBACK_FOR` → sibling page |
| `app/**/error.tsx` | `ErrorBoundary` | `CATCHES_FROM` → sibling page |
| `app/**/route.ts` | `Route` (API) | `HANDLES` → exported HTTP verb fn |
| `app/**/template.tsx` | `Template` | `WRAPS_PER_NAVIGATION` |
| `middleware.ts` (root) | `Middleware` | `INTERCEPTS` → all matching routes |

**Server/Client Component Tags — frontmatter on each symbol:**

- Detect top-of-file `'use client'` directive → tag symbol as `runtime: 'client'`
- Detect top-of-file `'use server'` directive → tag symbol as `runtime: 'server'`
- Default (no directive in App Router) → `runtime: 'server'` (Next.js default)
- Surface as `node.attrs.runtime` so `msp_symbol_lookup` returns this hint

**Data Fetching Logic — track dataflow surfaces:**

- Pages App Router style: detect `generateStaticParams`, `generateMetadata`, top-level `fetch()` calls in Server Components
- Pages Router legacy: detect `getServerSideProps`, `getStaticProps`, `getStaticPaths` exports → tag as `DataLoader` edge `LOADS_FOR` → Page node
- Route handlers: identify `GET` / `POST` / `PUT` / `PATCH` / `DELETE` / `OPTIONS` exports as separate edges per HTTP verb

→ When Agent asks "What loads data for this page?" the graph answers directly.

## Why Framework Awareness

เพื่อให้ AI Agent สามารถทำ "Navigation" และ "Impact Analysis" ในระดับสถาปัตยกรรมได้จริง:
- ช่วยให้ Agent เข้าใจว่าการเปลี่ยนโค้ดที่ฟังก์ชันหนึ่ง จะส่งผลกระทบต่อ API Route ไหนบ้าง
- ช่วยให้การสืบค้นโค้ดเป็นไปตาม "ความต้องการทางธุรกิจ" (เช่น "หาโค้ดที่ใช้จัดการข้อมูล Order") แทนที่จะหาตามชื่อไฟล์เพียงอย่างเดียว

## Verification

- **Integration Test:** สแกนโปรเจกต์ Next.js + Prisma ตัวอย่าง แล้วตรวจสอบว่า `GraphStore` มีโหนดประเภท `Route` / `Page` / `Layout` / `Entity` ที่ถูกสร้างขึ้นอัตโนมัติ
- **Link Check:** ตรวจสอบว่ามี Edge ประเภท `HANDLES` / `WRAPS` / `RENDERS_AT` / `ACCESSES` / `LOADS_FOR` เชื่อมโยงระหว่าง Routes/Pages/Entities กับฟังก์ชันในโค้ดต้นฉบับ
- **Runtime Tag Check:** ตรวจสอบว่า symbol ใน `'use client'` file ถูก tag เป็น `runtime: 'client'` และ default Server Component เป็น `runtime: 'server'`
- **Data Fetching Check:** ตรวจสอบว่า fixture page ที่ export `generateStaticParams` มี edge `LOADS_FOR` ไปยัง Page node และ legacy `getServerSideProps` ก็ทำงานเหมือนกัน
- **MCP Verification:** เรียก `msp_symbol_neighbors` บนโหนดที่เป็น Route แล้วตรวจสอบว่าคืนค่า Handler function + linked Page (สำหรับ App Router) + runtime tag กลับมาถูกต้อง

## Out of scope

- การสแกน UI Components เชิงลึก (เน้นที่ API และ Data layer ก่อน)
- การรองรับ Framework ที่หลากหลายเกินไป (ในเฟสแรกจะเน้น Next.js และ Prisma เป็นหลัก)
- การวิเคราะห์สิทธิการเข้าถึง (Auth) ที่ซับซ้อนในระดับ Route

## Source

- `FEAT--SYMBOLS-MULTI-LANG`
- `packages/msp/src/symbols/parser/typescript.ts`
- Next.js File-based routing documentation
- Prisma Schema reference
