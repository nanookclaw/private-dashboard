# STATUS.md — Private Dashboard

## Current State

**Phase:** Deployed to staging, collector live, data retention active  
**Tests:** 47 passing  
**Last Updated:** 2026-02-13 21:25 UTC

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
- ✅ UI/UX overhaul: viewport-filling layout, responsive sparklines, improved labels, better trend display
- ✅ Custom SVG logo and favicon (ascending bar chart design, dark theme matching)
- ✅ Rename: "HNR Dashboard" → "The Pack" (Agent Operations) — represents Nanook + siblings
- ✅ Per-period sparklines: 7d/30d/90d fetch actual history data from API
- ✅ Sparkline hover: crosshair + value tooltip on mouse hover
- ✅ Relative time labels: "5m ago" instead of raw timestamps
- ✅ Backend trend fix: falls back to earliest point in window (fixes null trends for new data)
- ✅ Better null trend display: "no data yet" instead of "collecting…"
- ✅ Responsive mobile layout: 1-col mobile, 2-col tablet, viewport-filling desktop
- ✅ GitHub Actions CI/CD — cargo test + Docker build + push to ghcr.io (a425e2a)
- ✅ Touch-friendly period buttons on mobile
- ✅ Stacked header on small screens, responsive font sizes

## What's Next

1. **Add to /mylinks page** — Jordan wants the private dashboard linked on <staging-domain>/mylinks (Jordan direction 2026-02-10)

3. **Staging domain** — Add dashboard.<staging-domain> to Cloudflare wildcard (Jordan action)
4. **Unit suffixes** — Add contextual units to metrics (e.g., "656 agents", "444 commits")

## ⚠️ Gotchas

- CI workflow push confirmed working (Feb 13)
- Manage key is auto-generated on first run and printed to stdout — save it
- Current manage key on staging: `dash_0e54dee0985e417b8ecb78b3607ad816`
- Frontend requires `bun` for Docker build (same pattern as other HNR projects)
- openapi.json must be COPY'd in Dockerfile backend stage (include_str! needs it at compile time)
- Staging runs ghcr.io image + Watchtower label enabled (auto-pulls :dev every 5 min)
- Collector cron ID: 31f1ab2e-c191-4872-a9ce-f746e5d74928
- moltbook_health reads 0 when platformHealth.status != "healthy" (check moltbook-state.json)
- Data retention: 90 days default, runs on startup, manual trigger via POST /api/v1/stats/prune

## Incoming Directions (Work Queue)

<!-- WORK_QUEUE_DIRECTIONS_START -->
- [ ] Add Private Dashboard to /mylinks page — Jordan wants the Private Dashboard linked on the staging /mylinks page (Jordan; 2026-02-13T18:49:54.763Z; task_id: 5dfaeec7-6373-4bac-8ed2-8ca2ed37fb16)
- [ ] Dashboard responsive layout for mobile — dashboard needs responsive layout for mobile (Jordan; 2026-02-13 07:52:02; task_id: 98f0acf7-1a6b-45a1-bea7-27c9aac6e0e3)
- [ ] Private Dashboard UI/UX improvements — Triage check: verify if this was completed. If evidence in git/code that it's done, close it. If not, work on it. (Jordan; 2026-02-13T09:59:54.311Z; task_id: f5f0e7fb-c012-4170-9c1f-9e761901c012)
- [ ] Private Dashboard: llms.txt + OpenAPI spec + test coverage expansion — Triage check: verify if this was completed. If evidence in git/code that it's done, close it. If not, work on it. (Jordan; 2026-02-13T09:59:54.620Z; task_id: 4cb81346-087a-44f8-85fa-2f82422fd37d)
- [ ] Private Dashboard: data retention (auto-prune + endpoint) — Auto-prune on startup (90-day retention), POST /api/v1/stats/prune manual endpoint, health endpoint shows retention info + oldest stat, 9 new tests (47 total), OpenAPI + llms.txt updated. Deployed to staging. (Jordan; 2026-02-13T09:59:54.747Z; task_id: a20ac6c2-9433-4197-98ac-3d25331ec539)
<!-- WORK_QUEUE_DIRECTIONS_END -->
