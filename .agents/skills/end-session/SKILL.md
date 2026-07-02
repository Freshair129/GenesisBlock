---
name: end-session
description: Wrap up the current working session — write a session summary to .brain/session/, refresh the self-note/TODO in .brain/memory/, optionally commit/push, and confirm a clean working tree. Use when the user says "end session", "wrap up", "ปิด session", "/end-session", or otherwise signals the work is done and should be recorded for the next session.
---

# End Session

Close out the current session by persisting durable memory so the **next** session
(or the next agent) can resume with full context. The `.brain/` directory is the
project's working memory — treat it as the source of truth for "where we are".

Run these steps in order. Skip a step only if it clearly does not apply, and say so.

## 1. Write a session summary → `.brain/session/`

Create `.brain/session/SESSION--<YYYY-MM-DD>[-<B/C…>]-<SHORT-SLUG>.md`.
- If a session file already exists for today, add a suffix (`-B`, `-C`) — do **not**
  overwrite a prior session's record.
- Match the style of the most recent existing `SESSION--*.md`. Include:
  - One-line **entry point** (what the session started from) + final commit range.
  - **Arc**: numbered narrative of what happened and *why* (decisions, corrections,
    dead-ends), not just a changelog.
  - **Commits** made this session (oneline).
  - **Key measured numbers / results** if any benchmarking or perf work was done.
  - **Artifacts** created/changed (files, audit docs, bench result paths).
  - **Open / next** — defer to the self-note for the ranked list; note anything new.

## 2. Refresh the self-note / TODO → `.brain/memory/`

Update (or create) `.brain/memory/SELF-NOTE--<YYYY-MM-DD>.md`:
- Mark finished items **DONE** (strike-through is fine) rather than deleting them —
  the trail of what was decided matters.
- Add any new hard-won facts, gotchas, or "do not repeat" corrections discovered
  this session (e.g. a wrong assumption the user corrected).
- Keep / update a **ranked "highest-leverage next work"** list so the next session
  knows where to start.
- Convert relative dates to absolute. Don't duplicate what the code/git already records.
- If the repo uses a `MEMORY.md` index, add/refresh the one-line pointer there.

## 3. Verify state, then optionally commit & push

- Run `git status` (or equivalent) and confirm the working tree is clean **except**
  for intended leftovers (e.g. `.brain/` if gitignored, generated scratch).
- If the user asked to commit/push (or there is uncommitted session work and the
  user wants it saved): make logically-grouped commits with clear messages, then
  push **only if explicitly asked**. Follow the repo's commit conventions
  (sign-off / co-author trailer if used). Never push without an explicit ask.

## 4. Confirm and close

Give the user a short close-out:
- Where the session summary + self-note were written.
- Commit range pushed (if any) and that the tree is clean.
- The top 1–3 "next session" items from the self-note.

## Notes
- Be honest in the record: failed tests, skipped steps, and unverified claims go in
  as-is. The summary is for resuming work accurately, not for looking good.
- Do not invent a future obligation (cron/schedule) unless the session actually left
  a dated artifact that warrants one.
