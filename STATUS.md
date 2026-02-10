# STATUS.md — Private Dashboard

## Current State

**Phase:** Deployed to staging, collector live, data retention active  
**Tests:** 47 passing  
**Last Updated:** 2026-02-10 19:55 UTC

## What's Done

- ✅ Project scaffold (Rust/Rocket + React/Tailwind)
- ✅ SQLite database with stats table + config table
- ✅ POST /api/v1/stats — batch submit with auth (max 100, key validation)
- ✅ GET /api/v1/stats — all metrics with trend data (24h/7d/30d/90d)
- ✅ GET /api/v1/stats/:key — single stat history with period filter
- ✅ GET /api/v1/health — health check with stats count, retention info, oldest stat
- ✅ POST /api/v1/stats/prune — manual data retention trigger (auth required)
- ✅ Auto-prune on startup (90-day retention, deletes old stats automatically)
- ✅ GET /llms.txt — AI-readable API summary (updated with prune docs)
- ✅ GET /openapi.json — full OpenAPI 3.0.3 spec (updated with prune endpoint)
- ✅ Auto-generated manage key on first run
- ✅ Frontend: dark theme dashboard with stat cards, sparklines, trend badges
- ✅ Frontend: responsive grid, auto-refresh 60s, empty state
- ✅ 47 HTTP tests (auth, submit, query, validation, trends, batch limits, key labels, llms.txt, openapi, edge cases, prune)
- ✅ Dockerfile (multi-stage: frontend + backend)
- ✅ docker-compose.yml (port 3008)
- ✅ Deployed to staging (192.168.0.79:3008)
- ✅ GitHub repo created, pushed to main
- ✅ DESIGN.md with full API spec
- ✅ DB backup script already includes private-dashboard
- ✅ Collector cron (every 30 min) — gathers 10 metrics from workspace state files
- ✅ Collector script: scripts/dashboard-collector.py (in workspace)
- ✅ Metric labels for all collector keys (siblings_active, moltbook_health, moltbook_my_posts, twitter_accounts)
- ✅ Edge case tests: negative/zero/large/fractional values, special chars, invalid JSON, missing fields, large metadata, all periods, rapid writes, 50-key batch
- ✅ Data retention: 90-day auto-prune on startup + manual prune endpoint + 9 prune tests

## What's Next

1. **GitHub Actions CI** — test + build + push to ghcr.io (needs workflow scope — blocked)
2. **Staging domain** — Add dashboard.hnrstage.xyz to Cloudflare wildcard (Jordan action)
3. **Frontend polish** — Better formatting for metric labels, add unit suffixes

## ⚠️ Gotchas

- CI workflow push requires `workflow` scope on GitHub token (blocked on all HNR repos)
- Manage key is auto-generated on first run and printed to stdout — save it
- Current manage key on staging: `dash_0e54dee0985e417b8ecb78b3607ad816`
- Frontend requires `bun` for Docker build (same pattern as other HNR projects)
- openapi.json must be COPY'd in Dockerfile backend stage (include_str! needs it at compile time)
- Currently using local Docker build on staging (no CI/ghcr.io yet)
- Collector cron ID: 31f1ab2e-c191-4872-a9ce-f746e5d74928
- moltbook_health reads 0 when platformHealth.status != "healthy" (check moltbook-state.json)
- Data retention: 90 days default, runs on startup, manual trigger via POST /api/v1/stats/prune
