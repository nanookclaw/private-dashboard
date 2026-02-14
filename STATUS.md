# STATUS.md — Private Dashboard

## Current State

**Phase:** Deployed to staging, collector live, data retention active  
**Tests:** 53 passing  
**Last Updated:** 2026-02-14 02:25 UTC

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
- ✅ **Responsive fix (ee54118):** flex-1/min-h-0 → lg:flex-1/lg:min-h-0 on stats wrapper, groups, and grids. Mobile/tablet now size naturally and scroll; desktop unchanged.
- ✅ Unit suffixes — contextual units next to metric values (agents, commits, tests, repos, etc.) (aafae8a)
- ✅ Already on /mylinks page (hnrstage.xyz/mylinks) — first card with all links
- ✅ Metric grouping — Development, Network, Moltbook, Social sections with labeled dividers (7403127)
- ✅ Binary metric display — moltbook_health shows "Healthy"/"Down" with color instead of 1/0 (7403127)
- ✅ Trend alerts — pulsing dot + glow border for significant 24h changes (±10% alert, ±25% hot with ⚡) (f0ea09d)
- ✅ **Metric detail view** — click any card for full modal with interactive chart, trend summaries, data table (6e14145)
- ✅ **Custom date range** — "Custom" period option with date pickers; backend accepts start/end params (ISO-8601 or YYYY-MM-DD); 6 new tests (74f445a)

## What's Next

1. **Staging domain** — Add dashboard.<staging-domain> to Cloudflare wildcard (Jordan action)
2. **Alert history log** — Track when metrics triggered alerts (optional)

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
- ~~**2026-02-14 01:10 UTC (Jordan):** Responsive layout is not working properly. Please re-investigate. (task 98f0acf7)~~ → Fixed in ee54118: root cause was unconditional flex-1/min-h-0 crushing groups on mobile.
<!-- WORK_QUEUE_DIRECTIONS_END -->
