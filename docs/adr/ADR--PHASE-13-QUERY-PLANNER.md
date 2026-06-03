# ADR--PHASE-13-QUERY-PLANNER

## 1. Status
**Proposed**

## 2. Context
Currently, HQL (Hybrid Query Language) strings are handled by a primitive regex dispatcher. This approach is fragile, doesn't support complex logical nesting (AND/OR), and lacks optimization (e.g., deciding whether to perform a vector search before a graph traversal). To become a true "Knowledge Engine", GenesisDB requires a formal query pipeline.

## 3. Decision
We will implement a structured Query Engine consisting of a **Pest-based Parser**, an **Abstract Syntax Tree (AST)**, and a **Logical/Physical Planner**.

## 4. Architectural Components

### 4.1 Lexer & Parser (Pest)
- Use PEG (Parsing Expression Grammar) via the \pest\ crate.
- Formally define \grammar.pest\ covering \MATCH\, \WHERE\, \RETURN\ clauses.

### 4.2 AST (Abstract Syntax Tree)
- Represent queries as a tree of enums (e.g., \Query\, \Pattern\, \Constraint\).
- This AST allows for static validation of the query before execution.

### 4.3 Logical Planner & Optimizer
- **Task:** Translate AST into a list of execution steps (the Plan).
- **Optimization:** Use a basic heuristic. For example:
  - If a \SIMILAR TO\ constraint is present with a small \K\, perform the Vector Search first, then filter results via graph edges.
  - If a \TRAVERSE\ constraint is more specific (single root ID), walk the graph first.

### 4.4 Execution Engine (Physical Plan)
- Execute the plan using the sharded \DashMap\ and \HNSW\ index.

## 5. Consequences
- **Pros:** Robust query handling. Better performance for complex hybrid searches. Provides "Reproducible Code" for architectural audit.
- **Cons:** Significantly increases codebase size and complexity.
