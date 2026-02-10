# STATUS.md — Private Dashboard

## Current State

**Phase:** Deployed to staging with full API docs  
**Tests:** 22 passing  
**Last Updated:** 2026-02-10 18:55 UTC

## What's Done

- ✅ Project scaffold (Rust/Rocket + React/Tailwind)
- ✅ SQLite database with stats table + config table
- ✅ POST /api/v1/stats — batch submit with auth (max 100, key validation)
- ✅ GET /api/v1/stats — all metrics with trend data (24h/7d/30d/90d)
- ✅ GET /api/v1/stats/:key — single stat history with period filter
- ✅ GET /api/v1/health — health check with stats count
- ✅ GET /llms.txt — AI-readable API summary
- ✅ GET /openapi.json — full OpenAPI 3.0.3 spec
- ✅ Auto-generated manage key on first run
- ✅ Frontend: dark theme dashboard with stat cards, sparklines, trend badges
- ✅ Frontend: responsive grid, auto-refresh 60s, empty state
- ✅ 22 HTTP tests (auth, submit, query, validation, trends, batch limits, key labels, llms.txt, openapi)
- ✅ Dockerfile (multi-stage: frontend + backend)
- ✅ docker-compose.yml (port 3008)
- ✅ Deployed to staging (192.168.0.79:3008)
- ✅ GitHub repo created, pushed to main
- ✅ DESIGN.md with full API spec
- ✅ DB backup script already includes private-dashboard

## What's Next

1. **GitHub Actions CI** — test + build + push to ghcr.io (needs workflow scope — blocked)
2. **Collector cron** — Playbook that reads state files and POSTs to dashboard
3. **Staging domain** — Add dashboard.hnrstage.xyz to Cloudflare wildcard
4. **More edge case tests** — concurrent writes, negative values, special characters in keys

## ⚠️ Gotchas

- CI workflow push requires `workflow` scope on GitHub token (blocked on all HNR repos)
- Manage key is auto-generated on first run and printed to stdout — save it
- Current manage key on staging: `dash_0e54dee0985e417b8ecb78b3607ad816`
- Frontend requires `bun` for Docker build (same pattern as other HNR projects)
- openapi.json must be COPY'd in Dockerfile backend stage (include_str! needs it at compile time)
- Currently using local Docker build on staging (no CI/ghcr.io yet)
