# v0.11.17

## Features

- Cookie usage breakdown by model family
  - Add session/7‑day/7‑day Opus/lifetime usage buckets with per‑family totals (Sonnet, Opus) alongside overall totals (input/output tokens).
  - Frontend visualization now expands each window to show Sonnet/Opus input/output when present.
  - Storage layer persists the new UsageBreakdown structures when DB persistence is enabled.

## Improvements

- Settings: decouple Vertex from the Config page
  - The Config page payload no longer includes `vertex`; Vertex credentials are managed exclusively under the Gemini tab.
  - Server preserves existing Vertex configuration on config updates regardless of request body contents.
  - GET /api/config no longer returns Vertex fields to the frontend to avoid accidental round‑trips.
- API ergonomics
  - Change Config update to POST semantics and enhance error propagation so the UI surfaces backend error messages.

## Bug Fixes

- Fix 422 on config save when a placeholder `vertex.credential` string was round‑tripped from the UI. The endpoint now ignores Vertex from the Config page and preserves existing values.
