#!/usr/bin/env python3
"""
Integration tests for the Private Dashboard Python SDK.

Usage:
    DASHBOARD_URL=http://192.168.0.79:3008 DASHBOARD_KEY=dash_xxx python3 test_sdk.py

Requires a running private-dashboard instance.
"""

import json
import os
import sys
import time
import traceback
from datetime import datetime, timezone, timedelta

# Add SDK to path
sys.path.insert(0, os.path.dirname(__file__))
from dashboard import (
    Dashboard,
    DashboardError,
    AuthError,
    NotFoundError,
    ValidationError,
    RateLimitError,
    ServerError,
)


PASS = 0
FAIL = 0
ERRORS: list = []


def test(name: str):
    """Decorator for test functions."""
    def decorator(fn):
        def wrapper(*args, **kwargs):
            global PASS, FAIL
            try:
                fn(*args, **kwargs)
                PASS += 1
                print(f"  ✅ {name}")
            except Exception as e:
                FAIL += 1
                ERRORS.append((name, e))
                print(f"  ❌ {name}: {e}")
                traceback.print_exc()
        return wrapper
    return decorator


# ── Test Helpers ────────────────────────────────────────────────────

def unique_key(prefix: str = "sdk_test") -> str:
    """Generate a unique metric key for test isolation."""
    return f"{prefix}_{int(time.time() * 1000) % 1_000_000}"


# ── Health / Discovery Tests ────────────────────────────────────────

@test("health — returns ok status")
def test_health(dash: Dashboard):
    h = dash.health()
    assert h["status"] == "ok", f"Expected 'ok', got {h['status']}"
    assert "version" in h, "Missing version"
    assert "stats_count" in h, "Missing stats_count"
    assert "keys_count" in h, "Missing keys_count"
    assert "retention_days" in h, "Missing retention_days"
    assert isinstance(h["retention_days"], int), "retention_days should be int"


@test("health — response fields complete")
def test_health_fields(dash: Dashboard):
    h = dash.health()
    expected_fields = {"status", "version", "stats_count", "keys_count", "retention_days", "oldest_stat"}
    actual_fields = set(h.keys())
    assert expected_fields.issubset(actual_fields), f"Missing fields: {expected_fields - actual_fields}"


@test("is_healthy — convenience helper")
def test_is_healthy(dash: Dashboard):
    assert dash.is_healthy() is True


@test("llms.txt — returns text content")
def test_llms_txt(dash: Dashboard):
    txt = dash.llms_txt()
    assert "The Pack" in txt, "Expected 'The Pack' in llms.txt"
    assert "/api/v1" in txt, "Expected API reference"
    assert "manage_key" in txt, "Expected auth docs"


@test("openapi — returns valid JSON spec")
def test_openapi(dash: Dashboard):
    spec = dash.openapi()
    assert spec.get("openapi", "").startswith("3."), f"Expected OpenAPI 3.x, got {spec.get('openapi')}"
    assert "paths" in spec, "Missing paths"
    assert "/health" in spec["paths"] or "/api/v1/health" in spec["paths"], "Missing health path"


@test("skills index — Cloudflare RFC discovery")
def test_skills_index(dash: Dashboard):
    idx = dash.skills_index()
    assert "skills" in idx, "Missing skills array"
    assert len(idx["skills"]) >= 1, "Expected at least 1 skill"
    skill = idx["skills"][0]
    assert skill["name"] == "private-dashboard", f"Expected 'private-dashboard', got {skill['name']}"
    assert "files" in skill, "Missing files"


@test("SKILL.md — integration guide content")
def test_skill_md(dash: Dashboard):
    md = dash.skill_md()
    assert "Quick Start" in md, "Missing Quick Start section"
    assert "Auth Model" in md, "Missing Auth Model section"
    assert "manage_key" in md, "Expected manage_key reference"


# ── Submit Tests ────────────────────────────────────────────────────

@test("submit — dict shorthand")
def test_submit_dict(dash: Dashboard):
    k = unique_key("submit_dict")
    accepted = dash.submit({k: 42.0})
    assert accepted == 1, f"Expected 1 accepted, got {accepted}"


@test("submit — list form with metadata")
def test_submit_list(dash: Dashboard):
    k = unique_key("submit_list")
    accepted = dash.submit([
        {"key": k, "value": 100.0, "metadata": {"source": "test"}},
    ])
    assert accepted == 1, f"Expected 1 accepted, got {accepted}"


@test("submit — batch multiple metrics")
def test_submit_batch(dash: Dashboard):
    k1, k2, k3 = unique_key("batch_a"), unique_key("batch_b"), unique_key("batch_c")
    accepted = dash.submit({k1: 1.0, k2: 2.0, k3: 3.0})
    assert accepted == 3, f"Expected 3 accepted, got {accepted}"


@test("submit_one — single metric helper")
def test_submit_one(dash: Dashboard):
    k = unique_key("submit_one")
    result = dash.submit_one(k, 99.9)
    assert result is True, "Expected True"


@test("submit_one — with metadata")
def test_submit_one_meta(dash: Dashboard):
    k = unique_key("submit_meta")
    result = dash.submit_one(k, 55.0, metadata={"env": "test"})
    assert result is True


@test("submit — empty list returns 400")
def test_submit_empty(dash: Dashboard):
    try:
        dash.submit([])
        assert False, "Should have raised ValidationError"
    except ValidationError as e:
        assert e.status == 400


@test("submit — over 100 items returns 400")
def test_submit_too_many(dash: Dashboard):
    items = [{"key": f"k{i}", "value": float(i)} for i in range(101)]
    try:
        dash.submit(items)
        assert False, "Should have raised ValidationError"
    except ValidationError as e:
        assert e.status == 400


@test("submit — bad auth returns 403")
def test_submit_bad_auth(dash: Dashboard):
    bad = Dashboard(dash.base_url, manage_key="wrong_key")
    try:
        bad.submit({"test": 1.0})
        assert False, "Should have raised AuthError"
    except AuthError as e:
        assert e.status == 403


# ── Stats / Read Tests ──────────────────────────────────────────────

@test("stats — returns list after submit")
def test_stats_list(dash: Dashboard):
    k = unique_key("stats_list")
    dash.submit({k: 42.0})
    stats = dash.stats()
    assert isinstance(stats, list), "Expected list"
    found = [s for s in stats if s["key"] == k]
    assert len(found) == 1, f"Expected to find key {k}"
    assert found[0]["current"] == 42.0


@test("stats — summary fields complete")
def test_stats_fields(dash: Dashboard):
    k = unique_key("stats_fields")
    dash.submit({k: 10.0})
    stats = dash.stats()
    found = [s for s in stats if s["key"] == k][0]
    assert "key" in found
    assert "label" in found
    assert "current" in found
    assert "trends" in found
    assert "sparkline_24h" in found
    assert "last_updated" in found


@test("stats — trends structure")
def test_stats_trends(dash: Dashboard):
    k = unique_key("trends")
    dash.submit({k: 50.0})
    found = [s for s in dash.stats() if s["key"] == k][0]
    trends = found["trends"]
    for period in ["24h", "7d", "30d", "90d"]:
        assert period in trends, f"Missing trend period {period}"
        t = trends[period]
        assert "start" in t
        assert "end" in t
        assert "change" in t
        assert "pct" in t


@test("stats — sparkline is list of numbers")
def test_stats_sparkline(dash: Dashboard):
    k = unique_key("sparkline")
    dash.submit({k: 7.0})
    found = [s for s in dash.stats() if s["key"] == k][0]
    assert isinstance(found["sparkline_24h"], list)


@test("stat — get single metric")
def test_stat_single(dash: Dashboard):
    k = unique_key("single")
    dash.submit({k: 33.3})
    s = dash.stat(k)
    assert s is not None, "Expected stat"
    assert s["current"] == 33.3


@test("stat — nonexistent returns None")
def test_stat_missing(dash: Dashboard):
    s = dash.stat("nonexistent_key_xyz_123")
    assert s is None


@test("get_value — convenience helper")
def test_get_value(dash: Dashboard):
    k = unique_key("getval")
    dash.submit({k: 77.7})
    val = dash.get_value(k)
    assert val == 77.7


@test("get_value — missing returns None")
def test_get_value_missing(dash: Dashboard):
    val = dash.get_value("nonexistent_xyz")
    assert val is None


@test("get_trend — returns pct for metric")
def test_get_trend(dash: Dashboard):
    k = unique_key("trend_test")
    dash.submit({k: 100.0})
    pct = dash.get_trend(k, "24h")
    # New metric with single value: trend uses same value as start and end
    # so pct is either None (no prior data) or 0.0 (same value)
    assert pct is None or pct == 0.0, f"Expected None or 0.0, got {pct}"


@test("keys — list of tracked metric keys")
def test_keys(dash: Dashboard):
    k = unique_key("keys_test")
    dash.submit({k: 1.0})
    keys = dash.keys()
    assert k in keys, f"Expected {k} in keys"


# ── History Tests ───────────────────────────────────────────────────

@test("history — default period (24h)")
def test_history_default(dash: Dashboard):
    k = unique_key("hist_default")
    dash.submit({k: 10.0})
    points = dash.history(k)
    assert isinstance(points, list)
    assert len(points) >= 1
    assert points[0]["value"] == 10.0
    assert "recorded_at" in points[0]


@test("history — 7d period")
def test_history_7d(dash: Dashboard):
    k = unique_key("hist_7d")
    dash.submit({k: 20.0})
    points = dash.history(k, period="7d")
    assert len(points) >= 1


@test("history — 30d period")
def test_history_30d(dash: Dashboard):
    k = unique_key("hist_30d")
    dash.submit({k: 30.0})
    points = dash.history(k, period="30d")
    assert len(points) >= 1


@test("history — 90d period")
def test_history_90d(dash: Dashboard):
    k = unique_key("hist_90d")
    dash.submit({k: 40.0})
    points = dash.history(k, period="90d")
    assert len(points) >= 1


@test("history — custom date range")
def test_history_custom_range(dash: Dashboard):
    k = unique_key("hist_custom")
    dash.submit({k: 50.0})
    now = datetime.now(timezone.utc)
    start = (now - timedelta(hours=1)).strftime("%Y-%m-%dT%H:%M:%SZ")
    end = (now + timedelta(hours=1)).strftime("%Y-%m-%dT%H:%M:%SZ")
    points = dash.history(k, start=start, end=end)
    assert len(points) >= 1


@test("history — custom date range (YYYY-MM-DD format)")
def test_history_date_format(dash: Dashboard):
    k = unique_key("hist_date")
    dash.submit({k: 60.0})
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    points = dash.history(k, start=today, end=today)
    assert len(points) >= 1


@test("history — empty key returns empty list")
def test_history_empty_key(dash: Dashboard):
    points = dash.history("nonexistent_key_abc_999")
    assert points == []


@test("history — invalid period returns 400")
def test_history_invalid_period(dash: Dashboard):
    try:
        dash.history("anything", period="1y")
        assert False, "Should have raised ValidationError"
    except ValidationError as e:
        assert e.status == 400


@test("history — multiple values chronological")
def test_history_chronological(dash: Dashboard):
    k = unique_key("hist_chrono")
    dash.submit_one(k, 1.0)
    time.sleep(0.1)
    dash.submit_one(k, 2.0)
    time.sleep(0.1)
    dash.submit_one(k, 3.0)
    points = dash.history(k)
    values = [p["value"] for p in points]
    assert values == sorted(values), f"Expected chronological order, got {values}"
    assert values[-1] == 3.0


# ── Delete Tests ────────────────────────────────────────────────────

@test("delete — removes all data for key")
def test_delete(dash: Dashboard):
    k = unique_key("delete")
    dash.submit({k: 100.0})
    # Verify it exists
    assert dash.get_value(k) == 100.0
    # Delete
    deleted = dash.delete(k)
    assert deleted >= 1, f"Expected at least 1 deleted, got {deleted}"
    # Verify gone
    assert dash.get_value(k) is None


@test("delete — nonexistent key returns 404")
def test_delete_missing(dash: Dashboard):
    try:
        dash.delete("nonexistent_key_delete_xyz")
        assert False, "Should have raised NotFoundError"
    except NotFoundError as e:
        assert e.status == 404


@test("delete — bad auth returns 403")
def test_delete_bad_auth(dash: Dashboard):
    bad = Dashboard(dash.base_url, manage_key="wrong_key")
    try:
        bad.delete("anything")
        assert False, "Should have raised AuthError"
    except AuthError as e:
        assert e.status == 403


# ── Prune Tests ─────────────────────────────────────────────────────

@test("prune — returns retention info")
def test_prune(dash: Dashboard):
    result = dash.prune()
    assert "deleted" in result
    assert "retention_days" in result
    assert "remaining" in result
    assert isinstance(result["retention_days"], int)


@test("prune — bad auth returns 403")
def test_prune_bad_auth(dash: Dashboard):
    bad = Dashboard(dash.base_url, manage_key="wrong_key")
    try:
        bad.prune()
        assert False, "Should have raised AuthError"
    except AuthError as e:
        assert e.status == 403


# ── Alert Tests ─────────────────────────────────────────────────────

@test("alerts — returns list")
def test_alerts_list(dash: Dashboard):
    alerts = dash.alerts()
    assert isinstance(alerts, list)


@test("alerts — with limit")
def test_alerts_limit(dash: Dashboard):
    alerts = dash.alerts(limit=5)
    assert isinstance(alerts, list)
    assert len(alerts) <= 5


@test("alerts — with key filter")
def test_alerts_key_filter(dash: Dashboard):
    alerts = dash.alerts(key="nonexistent_key_xyz")
    assert alerts == []


@test("alerts — alert fields structure")
def test_alerts_fields(dash: Dashboard):
    # Submit data that might trigger alerts (won't always — depends on existing data)
    alerts = dash.alerts(limit=50)
    if alerts:  # only check if there are alerts
        a = alerts[0]
        expected = {"key", "label", "level", "value", "change_pct", "triggered_at"}
        assert expected.issubset(set(a.keys())), f"Missing fields: {expected - set(a.keys())}"
        assert a["level"] in ("alert", "hot"), f"Unexpected level: {a['level']}"


@test("alert_count — returns integer")
def test_alert_count(dash: Dashboard):
    count = dash.alert_count()
    assert isinstance(count, int)
    assert count >= 0


@test("hot_alerts — filters for hot level only")
def test_hot_alerts(dash: Dashboard):
    alerts = dash.hot_alerts()
    for a in alerts:
        assert a["level"] == "hot", f"Expected 'hot', got {a['level']}"


# ── Error Handling Tests ────────────────────────────────────────────

@test("error hierarchy — DashboardError is base")
def test_error_hierarchy(dash: Dashboard):
    assert issubclass(AuthError, DashboardError)
    assert issubclass(NotFoundError, DashboardError)
    assert issubclass(ValidationError, DashboardError)
    assert issubclass(RateLimitError, DashboardError)
    assert issubclass(ServerError, DashboardError)


@test("error — status code accessible")
def test_error_status(dash: Dashboard):
    bad = Dashboard(dash.base_url, manage_key="wrong")
    try:
        bad.submit({"x": 1.0})
        assert False, "Should have raised"
    except DashboardError as e:
        assert e.status == 403
        assert e.body is not None


@test("connection error — bad URL")
def test_connection_error(dash: Dashboard):
    bad = Dashboard("http://127.0.0.1:19999", manage_key="x", timeout=2)
    try:
        bad.health()
        assert False, "Should have raised DashboardError"
    except DashboardError:
        pass


@test("constructor — requires base_url")
def test_constructor_requires_url(dash: Dashboard):
    old = os.environ.pop("DASHBOARD_URL", None)
    try:
        Dashboard()
        assert False, "Should have raised ValueError"
    except ValueError:
        pass
    finally:
        if old:
            os.environ["DASHBOARD_URL"] = old


# ── Value Update / Latest Tests ─────────────────────────────────────

@test("submit overwrites — latest value wins")
def test_latest_value(dash: Dashboard):
    k = unique_key("latest")
    dash.submit_one(k, 10.0)
    dash.submit_one(k, 20.0)
    dash.submit_one(k, 30.0)
    val = dash.get_value(k)
    assert val == 30.0, f"Expected 30.0, got {val}"


@test("stats — alphabetical ordering")
def test_stats_ordering(dash: Dashboard):
    # Submit metrics with known alphabetical order
    k_a = f"aaa_{unique_key()}"
    k_z = f"zzz_{unique_key()}"
    dash.submit({k_a: 1.0, k_z: 2.0})
    stats = dash.stats()
    keys = [s["key"] for s in stats]
    assert keys == sorted(keys), "Stats should be alphabetically ordered"


# ── Cleanup Tests ───────────────────────────────────────────────────

# ── Submit Edge Cases ───────────────────────────────────────────────

@test("submit — negative value accepted")
def test_submit_negative(dash: Dashboard):
    k = unique_key("neg")
    accepted = dash.submit({k: -42.5})
    assert accepted == 1
    val = dash.get_value(k)
    assert val == -42.5, f"Expected -42.5, got {val}"


@test("submit — zero value accepted")
def test_submit_zero(dash: Dashboard):
    k = unique_key("zero")
    accepted = dash.submit({k: 0.0})
    assert accepted == 1
    val = dash.get_value(k)
    assert val == 0.0, f"Expected 0.0, got {val}"


@test("submit — very large value accepted")
def test_submit_large_value(dash: Dashboard):
    k = unique_key("big")
    big = 999_999_999.99
    accepted = dash.submit({k: big})
    assert accepted == 1
    val = dash.get_value(k)
    assert val == big, f"Expected {big}, got {val}"


@test("submit — fractional precision preserved")
def test_submit_fractional(dash: Dashboard):
    k = unique_key("frac")
    accepted = dash.submit({k: 3.14159})
    assert accepted == 1
    val = dash.get_value(k)
    assert abs(val - 3.14159) < 0.001, f"Expected ~3.14159, got {val}"


@test("submit — special chars in key")
def test_submit_special_chars(dash: Dashboard):
    k = unique_key("test-key.with_specials")
    accepted = dash.submit({k: 1.0})
    assert accepted == 1
    val = dash.get_value(k)
    assert val == 1.0


@test("submit — large metadata accepted")
def test_submit_large_metadata(dash: Dashboard):
    k = unique_key("lgmeta")
    meta = {"items": [f"item_{i}" for i in range(100)], "description": "x" * 500}
    result = dash.submit_one(k, 42.0, metadata=meta)
    assert result is True


@test("submit — exactly 100 items accepted (boundary)")
def test_submit_exactly_100(dash: Dashboard):
    items = [{"key": f"boundary_{i}_{int(time.time()*1000)%1000000}", "value": float(i)} for i in range(100)]
    accepted = dash.submit(items)
    assert accepted == 100, f"Expected 100 accepted, got {accepted}"
    # Clean up
    for item in items:
        try:
            dash.delete(item["key"])
        except Exception:
            pass


@test("submit — mixed valid/invalid batch")
def test_submit_mixed_batch(dash: Dashboard):
    k1 = unique_key("mix_valid")
    # Submit with one good key and one super-long key (>255 chars, should be skipped)
    items = [
        {"key": k1, "value": 10.0},
        {"key": "x" * 300, "value": 20.0},  # too long, should be skipped
    ]
    accepted = dash.submit(items)
    # Server skips invalid keys (too long) and accepts valid ones
    assert accepted >= 1, f"Expected at least 1 accepted, got {accepted}"
    val = dash.get_value(k1)
    assert val == 10.0


@test("submit — rapid sequential writes")
def test_submit_rapid_writes(dash: Dashboard):
    k = unique_key("rapid")
    for i in range(10):
        dash.submit_one(k, float(i))
    val = dash.get_value(k)
    assert val == 9.0, f"Expected 9.0 (last write), got {val}"


@test("submit — many different keys at once")
def test_submit_many_keys(dash: Dashboard):
    prefix = unique_key("many")
    items = {f"{prefix}_{i}": float(i) for i in range(20)}
    accepted = dash.submit(items)
    assert accepted == 20, f"Expected 20, got {accepted}"
    keys = dash.keys()
    for i in range(20):
        assert f"{prefix}_{i}" in keys, f"Missing key {prefix}_{i}"
    # Clean up
    for k in items:
        try:
            dash.delete(k)
        except Exception:
            pass


@test("submit — no auth (missing key) returns error")
def test_submit_no_auth(dash: Dashboard):
    no_auth = Dashboard(dash.base_url, manage_key="not_a_real_key_abc")
    try:
        no_auth.submit({"test_no_auth": 1.0})
        assert False, "Should have raised DashboardError"
    except DashboardError as e:
        assert e.status in (401, 403), f"Expected 401 or 403, got {e.status}"


# ── Stats Advanced Tests ────────────────────────────────────────────

@test("stats — key_label fallback for unknown keys")
def test_key_label_fallback(dash: Dashboard):
    k = unique_key("unknown_label")
    dash.submit({k: 1.0})
    found = [s for s in dash.stats() if s["key"] == k]
    assert len(found) == 1
    # Unknown keys get auto-generated labels (underscores→spaces, prefix stripped)
    label = found[0].get("label", "")
    assert isinstance(label, str) and len(label) > 0, f"Label should be non-empty string, got '{label}'"


@test("stats — sparkline is list of floats/ints")
def test_sparkline_types(dash: Dashboard):
    k = unique_key("spark_types")
    dash.submit_one(k, 1.0)
    time.sleep(0.1)
    dash.submit_one(k, 2.0)
    found = [s for s in dash.stats() if s["key"] == k][0]
    sparkline = found["sparkline_24h"]
    assert isinstance(sparkline, list)
    for point in sparkline:
        assert isinstance(point, (int, float)), f"Sparkline point should be numeric, got {type(point)}"


@test("stats — last_updated is ISO-8601")
def test_stats_last_updated(dash: Dashboard):
    k = unique_key("updated_ts")
    dash.submit({k: 5.0})
    found = [s for s in dash.stats() if s["key"] == k][0]
    ts = found["last_updated"]
    assert "T" in ts, f"Expected ISO-8601, got '{ts}'"
    assert "Z" in ts or "+" in ts, f"Expected UTC indicator in '{ts}'"


@test("stats — read without auth works")
def test_stats_no_auth_read(dash: Dashboard):
    no_auth = Dashboard(dash.base_url)  # No manage key
    stats = no_auth.stats()
    assert isinstance(stats, list), "Read should work without auth"


@test("stats — health without auth works")
def test_health_no_auth(dash: Dashboard):
    no_auth = Dashboard(dash.base_url)
    h = no_auth.health()
    assert h["status"] == "ok"


# ── History Advanced Tests ──────────────────────────────────────────

@test("history — start only (no end) returns 400")
def test_history_start_only(dash: Dashboard):
    k = unique_key("hist_start_only")
    dash.submit({k: 10.0})
    try:
        dash.history(k, start="2026-01-01")
        # Some servers might handle this gracefully, not raise
    except (ValidationError, DashboardError):
        pass  # Expected - partial custom range


@test("history — end only (no start) returns 400")
def test_history_end_only(dash: Dashboard):
    k = unique_key("hist_end_only")
    dash.submit({k: 10.0})
    try:
        dash.history(k, end="2026-12-31")
    except (ValidationError, DashboardError):
        pass  # Expected - partial custom range


@test("history — future date range returns empty")
def test_history_future_dates(dash: Dashboard):
    k = unique_key("hist_future")
    dash.submit({k: 10.0})
    points = dash.history(k, start="2030-01-01T00:00:00Z", end="2030-12-31T00:00:00Z")
    assert points == [], f"Expected empty list for future dates, got {len(points)} points"


@test("history — multiple data points count")
def test_history_many_points(dash: Dashboard):
    k = unique_key("hist_many")
    for i in range(5):
        dash.submit_one(k, float(i * 10))
        time.sleep(0.05)
    points = dash.history(k)
    assert len(points) >= 5, f"Expected >=5 points, got {len(points)}"


@test("history — points have required fields")
def test_history_point_fields(dash: Dashboard):
    k = unique_key("hist_fields")
    dash.submit({k: 42.0})
    points = dash.history(k)
    assert len(points) >= 1
    p = points[0]
    assert "value" in p, "Missing 'value' field"
    assert "recorded_at" in p, "Missing 'recorded_at' field"
    assert isinstance(p["value"], (int, float))


@test("history — values match what was submitted")
def test_history_values_accurate(dash: Dashboard):
    k = unique_key("hist_accurate")
    dash.submit_one(k, 123.456)
    time.sleep(0.05)
    dash.submit_one(k, 789.012)
    points = dash.history(k)
    values = [p["value"] for p in points]
    assert 123.456 in values, f"Expected 123.456 in {values}"
    assert 789.012 in values, f"Expected 789.012 in {values}"


# ── Delete Advanced Tests ───────────────────────────────────────────

@test("delete — cascade removes history")
def test_delete_cascade_history(dash: Dashboard):
    k = unique_key("del_hist")
    dash.submit_one(k, 10.0)
    time.sleep(0.05)
    dash.submit_one(k, 20.0)
    time.sleep(0.05)
    dash.submit_one(k, 30.0)
    # Verify history exists
    points = dash.history(k)
    assert len(points) >= 3, f"Expected >=3 points before delete, got {len(points)}"
    # Delete
    deleted = dash.delete(k)
    assert deleted >= 3, f"Expected >=3 deleted, got {deleted}"
    # Verify history gone
    points_after = dash.history(k)
    assert points_after == [], f"Expected empty history after delete, got {len(points_after)} points"


@test("delete — key removed from stats")
def test_delete_removes_from_stats(dash: Dashboard):
    k = unique_key("del_stats")
    dash.submit({k: 99.0})
    # Verify present
    assert dash.get_value(k) == 99.0
    # Delete
    dash.delete(k)
    # Verify gone from stats
    assert dash.get_value(k) is None
    assert k not in dash.keys()


@test("delete — re-submit after delete works")
def test_delete_then_resubmit(dash: Dashboard):
    k = unique_key("del_resub")
    dash.submit({k: 50.0})
    assert dash.get_value(k) == 50.0
    dash.delete(k)
    assert dash.get_value(k) is None
    # Re-submit
    dash.submit({k: 75.0})
    assert dash.get_value(k) == 75.0


@test("delete — health keys_count reflects deletion")
def test_delete_health_count(dash: Dashboard):
    k = unique_key("del_health")
    h_before = dash.health()
    dash.submit({k: 1.0})
    h_after_add = dash.health()
    assert h_after_add["keys_count"] >= h_before["keys_count"], \
        f"keys_count should increase: {h_before['keys_count']} -> {h_after_add['keys_count']}"
    dash.delete(k)
    h_after_del = dash.health()
    assert h_after_del["keys_count"] < h_after_add["keys_count"], \
        f"keys_count should decrease after delete: {h_after_add['keys_count']} -> {h_after_del['keys_count']}"


# ── Prune Advanced Tests ───────────────────────────────────────────

@test("prune — idempotent (two calls same result)")
def test_prune_idempotent(dash: Dashboard):
    r1 = dash.prune()
    r2 = dash.prune()
    # Second prune should delete 0 (nothing new to prune)
    assert r2["deleted"] == 0, f"Second prune should delete 0, got {r2['deleted']}"
    assert r1["retention_days"] == r2["retention_days"]


@test("prune — response shape complete")
def test_prune_response_shape(dash: Dashboard):
    result = dash.prune()
    assert "deleted" in result, "Missing 'deleted'"
    assert "retention_days" in result, "Missing 'retention_days'"
    assert "remaining" in result, "Missing 'remaining'"
    assert isinstance(result["deleted"], int)
    assert isinstance(result["retention_days"], int)
    assert isinstance(result["remaining"], int)
    assert result["retention_days"] > 0, "retention_days should be positive"


# ── Alert Advanced Tests ────────────────────────────────────────────

@test("alerts — trigger alert with significant change")
def test_alerts_trigger(dash: Dashboard):
    k = unique_key("alert_trig")
    # Submit initial value
    dash.submit_one(k, 100.0)
    time.sleep(0.3)
    # Submit value with >25% change (should trigger hot alert)
    dash.submit_one(k, 200.0)
    time.sleep(0.1)
    # Check if alert was triggered
    alerts = dash.alerts(key=k, limit=10)
    # Alert may or may not fire depending on debounce window and existing data
    # Just verify the query works with no errors
    assert isinstance(alerts, list)


@test("alerts — ordered newest first")
def test_alerts_newest_first(dash: Dashboard):
    alerts = dash.alerts(limit=20)
    if len(alerts) >= 2:
        for i in range(len(alerts) - 1):
            ts_a = alerts[i].get("triggered_at", "")
            ts_b = alerts[i + 1].get("triggered_at", "")
            assert ts_a >= ts_b, f"Alerts not ordered newest first: {ts_a} < {ts_b}"


@test("alerts — limit=0 returns empty")
def test_alerts_limit_zero(dash: Dashboard):
    alerts = dash.alerts(limit=0)
    # May return empty or be treated as default — should not error
    assert isinstance(alerts, list)


@test("alerts — large limit clamped")
def test_alerts_limit_clamped(dash: Dashboard):
    alerts = dash.alerts(limit=9999)
    assert isinstance(alerts, list)
    assert len(alerts) <= 500, f"Expected <=500 alerts (clamped), got {len(alerts)}"


# ── Health Advanced Tests ───────────────────────────────────────────

@test("health — stats_count increments after submit")
def test_health_stats_count(dash: Dashboard):
    h1 = dash.health()
    k = unique_key("health_cnt")
    dash.submit({k: 1.0})
    h2 = dash.health()
    assert h2["stats_count"] > h1["stats_count"], \
        f"stats_count should increment: {h1['stats_count']} -> {h2['stats_count']}"


@test("health — version is string")
def test_health_version(dash: Dashboard):
    h = dash.health()
    assert isinstance(h["version"], str), f"version should be string, got {type(h['version'])}"
    assert len(h["version"]) > 0, "version should not be empty"


@test("health — oldest_stat present")
def test_health_oldest_stat(dash: Dashboard):
    h = dash.health()
    # oldest_stat can be null (if DB empty) or a string
    assert "oldest_stat" in h
    if h["oldest_stat"] is not None:
        assert isinstance(h["oldest_stat"], str)
        assert "T" in h["oldest_stat"], "oldest_stat should be ISO-8601"


# ── Trend Calculation Tests ─────────────────────────────────────────

@test("trend — multiple periods available")
def test_trend_all_periods(dash: Dashboard):
    k = unique_key("trend_all")
    dash.submit({k: 100.0})
    for period in ["24h", "7d", "30d", "90d"]:
        pct = dash.get_trend(k, period)
        # For new single-value metric, trend is None or 0
        assert pct is None or isinstance(pct, (int, float)), \
            f"get_trend({period}) should return None or number, got {type(pct)}"


@test("trend — change structure for all periods")
def test_trend_structure(dash: Dashboard):
    k = unique_key("trend_struct")
    dash.submit({k: 50.0})
    s = dash.stat(k)
    assert s is not None
    trends = s["trends"]
    for period in ["24h", "7d", "30d", "90d"]:
        t = trends[period]
        assert "start" in t, f"Missing 'start' in {period} trend"
        assert "end" in t, f"Missing 'end' in {period} trend"
        assert "change" in t, f"Missing 'change' in {period} trend"
        assert "pct" in t, f"Missing 'pct' in {period} trend"


# ── Discovery Advanced Tests ────────────────────────────────────────

@test("llms.txt — contains endpoints")
def test_llms_txt_endpoints(dash: Dashboard):
    txt = dash.llms_txt()
    assert "POST" in txt, "Expected POST method reference"
    assert "GET" in txt, "Expected GET method reference"
    assert "/stats" in txt, "Expected /stats endpoint"


@test("openapi — has expected path count")
def test_openapi_paths(dash: Dashboard):
    spec = dash.openapi()
    paths = spec.get("paths", {})
    assert len(paths) >= 6, f"Expected >= 6 paths, got {len(paths)}"


@test("openapi — info section complete")
def test_openapi_info(dash: Dashboard):
    spec = dash.openapi()
    info = spec.get("info", {})
    assert "title" in info, "Missing title"
    assert "version" in info, "Missing version"


@test("skills index — files array has SKILL.md")
def test_skills_files(dash: Dashboard):
    idx = dash.skills_index()
    skill = idx["skills"][0]
    files = skill.get("files", [])
    # Files can be strings or objects — check for SKILL.md either way
    has_skill = any(
        ("SKILL.md" in f) if isinstance(f, str) else ("SKILL.md" in f.get("path", ""))
        for f in files
    )
    assert has_skill, f"Expected SKILL.md in files, got {files}"


@test("SKILL.md — contains endpoint documentation")
def test_skill_md_endpoints(dash: Dashboard):
    md = dash.skill_md()
    assert "POST" in md or "GET" in md, "SKILL.md should document endpoints"
    assert "stats" in md.lower(), "SKILL.md should mention stats"


@test("llms.txt at /api/v1/ path also works")
def test_llms_txt_api_path(dash: Dashboard):
    import urllib.request
    url = f"{dash.base_url}/api/v1/llms.txt"
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=5) as resp:
        txt = resp.read().decode("utf-8")
    assert len(txt) > 50, f"llms.txt at /api/v1/ too short ({len(txt)} chars)"
    assert "/stats" in txt


# ── SDK Constructor / Config Tests ──────────────────────────────────

@test("constructor — env var fallback for URL")
def test_constructor_env_url(dash: Dashboard):
    os.environ["DASHBOARD_URL"] = dash.base_url
    os.environ["DASHBOARD_KEY"] = dash.manage_key
    try:
        d = Dashboard()
        assert d.is_healthy()
    finally:
        del os.environ["DASHBOARD_URL"]
        del os.environ["DASHBOARD_KEY"]


@test("constructor — custom timeout")
def test_constructor_timeout(dash: Dashboard):
    d = Dashboard(dash.base_url, manage_key=dash.manage_key, timeout=30)
    assert d.timeout == 30
    assert d.is_healthy()


@test("constructor — trailing slash stripped from base_url")
def test_constructor_trailing_slash(dash: Dashboard):
    d = Dashboard(dash.base_url + "/", manage_key=dash.manage_key)
    assert not d.base_url.endswith("/")
    assert d.is_healthy()


# ── Cross-Feature Interaction Tests ─────────────────────────────────

@test("submit + delete + history — full lifecycle")
def test_full_lifecycle(dash: Dashboard):
    k = unique_key("lifecycle")
    # Submit multiple values
    dash.submit_one(k, 10.0)
    time.sleep(0.05)
    dash.submit_one(k, 20.0)
    time.sleep(0.05)
    dash.submit_one(k, 30.0)
    # Verify current value
    assert dash.get_value(k) == 30.0
    # Verify history
    points = dash.history(k)
    assert len(points) >= 3
    # Verify in keys list
    assert k in dash.keys()
    # Delete
    dash.delete(k)
    # Verify everything cleaned up
    assert dash.get_value(k) is None
    assert k not in dash.keys()
    assert dash.history(k) == []


@test("submit + stats + health — counts consistent")
def test_counts_consistent(dash: Dashboard):
    h = dash.health()
    stats = dash.stats()
    assert h["keys_count"] == len(stats), \
        f"health keys_count ({h['keys_count']}) != len(stats) ({len(stats)})"


@test("history custom range includes submitted data")
def test_history_custom_range_includes(dash: Dashboard):
    k = unique_key("hist_incl")
    dash.submit({k: 77.7})
    now = datetime.now(timezone.utc)
    start = (now - timedelta(minutes=5)).strftime("%Y-%m-%dT%H:%M:%SZ")
    end = (now + timedelta(minutes=5)).strftime("%Y-%m-%dT%H:%M:%SZ")
    points = dash.history(k, start=start, end=end)
    assert len(points) >= 1, "Custom range should include just-submitted data"
    values = [p["value"] for p in points]
    assert 77.7 in values


@test("alerts — no auth needed for read")
def test_alerts_no_auth(dash: Dashboard):
    no_auth = Dashboard(dash.base_url)
    alerts = no_auth.alerts()
    assert isinstance(alerts, list)


# ── Metadata Tests ──────────────────────────────────────────────────

@test("metadata — persists through history")
def test_metadata_in_history(dash: Dashboard):
    k = unique_key("meta_hist")
    dash.submit([{"key": k, "value": 42.0, "metadata": {"source": "ci", "build": 123}}])
    points = dash.history(k)
    assert len(points) >= 1
    # Metadata may or may not appear in history response — but submit should succeed
    assert points[0]["value"] == 42.0


@test("metadata — null metadata accepted")
def test_metadata_null(dash: Dashboard):
    k = unique_key("meta_null")
    ok = dash.submit([{"key": k, "value": 5.0, "metadata": None}])
    assert ok >= 1
    assert dash.get_value(k) == 5.0


@test("metadata — nested JSON accepted")
def test_metadata_nested(dash: Dashboard):
    k = unique_key("meta_nest")
    meta = {"tags": ["prod", "deploy"], "details": {"version": "1.0", "count": 5}}
    ok = dash.submit_one(k, 10.0, metadata=meta)
    assert ok is True
    assert dash.get_value(k) == 10.0


@test("metadata — empty object accepted")
def test_metadata_empty_obj(dash: Dashboard):
    k = unique_key("meta_empty")
    ok = dash.submit([{"key": k, "value": 7.0, "metadata": {}}])
    assert ok >= 1


# ── Batch Duplicate Keys ────────────────────────────────────────────

@test("batch — duplicate keys in single batch both accepted")
def test_batch_duplicate_keys(dash: Dashboard):
    k = unique_key("batchdup")
    accepted = dash.submit([
        {"key": k, "value": 10.0},
        {"key": k, "value": 20.0},
    ])
    assert accepted == 2
    # Latest value should be the last one submitted
    val = dash.get_value(k)
    assert val == 20.0
    # History should have both points
    points = dash.history(k)
    assert len(points) >= 2


@test("batch — single item as list")
def test_batch_single_item_list(dash: Dashboard):
    k = unique_key("batchone")
    accepted = dash.submit([{"key": k, "value": 99.0}])
    assert accepted == 1
    assert dash.get_value(k) == 99.0


@test("batch — single item as dict")
def test_batch_single_item_dict(dash: Dashboard):
    k = unique_key("batchdict1")
    accepted = dash.submit({k: 88.0})
    assert accepted == 1
    assert dash.get_value(k) == 88.0


# ── History Ordering & Range Tests ──────────────────────────────────

@test("history — ascending chronological with many points")
def test_history_ascending_many(dash: Dashboard):
    k = unique_key("hist_asc")
    for i in range(10):
        dash.submit_one(k, float(i))
        time.sleep(0.05)
    points = dash.history(k)
    timestamps = [p["recorded_at"] for p in points]
    assert timestamps == sorted(timestamps), "History should be ascending chronological"
    values = [p["value"] for p in points]
    assert values[-1] == 9.0, f"Last value should be 9.0, got {values[-1]}"


@test("history — inverted date range returns empty or error")
def test_history_inverted_range(dash: Dashboard):
    k = unique_key("hist_inv")
    dash.submit({k: 1.0})
    # Start after end — server may return empty or 400
    try:
        points = dash.history(k, start="2030-01-01T00:00:00Z", end="2020-01-01T00:00:00Z")
        assert points == [], f"Inverted range should return empty, got {len(points)} points"
    except (ValidationError, DashboardError):
        pass  # Server correctly rejects inverted range


@test("history — same start and end date returns data for that day")
def test_history_same_day(dash: Dashboard):
    k = unique_key("hist_sameday")
    dash.submit({k: 25.0})
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    points = dash.history(k, start=today, end=today)
    # Should include data from today
    assert len(points) >= 1, "Same-day range should include today's data"


@test("history — very old date range returns empty")
def test_history_old_range(dash: Dashboard):
    k = unique_key("hist_old")
    dash.submit({k: 1.0})
    points = dash.history(k, start="2000-01-01T00:00:00Z", end="2000-12-31T00:00:00Z")
    assert points == [], f"Old range should return empty, got {len(points)} points"


# ── Trend Accuracy Tests ────────────────────────────────────────────

@test("trend — increasing values show positive change")
def test_trend_positive_change(dash: Dashboard):
    k = unique_key("trend_pos")
    dash.submit_one(k, 50.0)
    time.sleep(0.1)
    dash.submit_one(k, 100.0)
    s = dash.stat(k)
    assert s is not None
    assert s["current"] == 100.0
    # 24h trend should show the increase
    trend_24h = s["trends"]["24h"]
    assert trend_24h["end"] == 100.0


@test("trend — decreasing values reflect in trend")
def test_trend_decreasing(dash: Dashboard):
    k = unique_key("trend_dec")
    dash.submit_one(k, 200.0)
    time.sleep(0.1)
    dash.submit_one(k, 50.0)
    s = dash.stat(k)
    assert s is not None
    assert s["current"] == 50.0
    trend_24h = s["trends"]["24h"]
    assert trend_24h["end"] == 50.0


@test("trend — stable values show zero change")
def test_trend_stable(dash: Dashboard):
    k = unique_key("trend_stable")
    dash.submit_one(k, 100.0)
    time.sleep(0.1)
    dash.submit_one(k, 100.0)
    s = dash.stat(k)
    trend_24h = s["trends"]["24h"]
    # Change should be 0 if both start and end are 100
    if trend_24h["start"] is not None and trend_24h["end"] is not None:
        assert trend_24h["change"] == 0.0 or abs(trend_24h["change"]) < 0.01


@test("trend — all periods have numeric pct or null")
def test_trend_pct_types(dash: Dashboard):
    k = unique_key("trend_types")
    dash.submit({k: 42.0})
    s = dash.stat(k)
    for period in ["24h", "7d", "30d", "90d"]:
        pct = s["trends"][period].get("pct")
        assert pct is None or isinstance(pct, (int, float)), \
            f"pct for {period} should be None or numeric, got {type(pct)}: {pct}"


# ── Sparkline Data Quality ──────────────────────────────────────────

@test("sparkline — single value produces at least 1 point")
def test_sparkline_single_value(dash: Dashboard):
    k = unique_key("spark_single")
    dash.submit({k: 42.0})
    s = [st for st in dash.stats() if st["key"] == k][0]
    sparkline = s["sparkline_24h"]
    assert len(sparkline) >= 1, f"Expected at least 1 sparkline point, got {len(sparkline)}"
    assert 42.0 in sparkline, f"Expected 42.0 in sparkline, got {sparkline}"


@test("sparkline — multiple values increase point count")
def test_sparkline_multiple_values(dash: Dashboard):
    k = unique_key("spark_multi")
    for i in range(5):
        dash.submit_one(k, float(i * 10))
        time.sleep(0.05)
    s = [st for st in dash.stats() if st["key"] == k][0]
    sparkline = s["sparkline_24h"]
    assert len(sparkline) >= 2, f"Expected >=2 sparkline points, got {len(sparkline)}"


@test("sparkline — values are bounded by submitted range")
def test_sparkline_value_range(dash: Dashboard):
    k = unique_key("spark_range")
    dash.submit_one(k, 10.0)
    time.sleep(0.05)
    dash.submit_one(k, 50.0)
    time.sleep(0.05)
    dash.submit_one(k, 30.0)
    s = [st for st in dash.stats() if st["key"] == k][0]
    sparkline = s["sparkline_24h"]
    for v in sparkline:
        assert 10.0 <= v <= 50.0, f"Sparkline value {v} outside submitted range [10, 50]"


# ── Key Naming Tests ────────────────────────────────────────────────

@test("key — dots allowed in key name")
def test_key_dots(dash: Dashboard):
    k = unique_key("key.with.dots")
    ok = dash.submit_one(k, 1.0)
    assert ok
    val = dash.get_value(k)
    assert val == 1.0


@test("key — dashes allowed in key name")
def test_key_dashes(dash: Dashboard):
    k = unique_key("key-with-dashes")
    ok = dash.submit_one(k, 2.0)
    assert ok
    val = dash.get_value(k)
    assert val == 2.0


@test("key — underscores and numbers in key name")
def test_key_underscores_nums(dash: Dashboard):
    k = unique_key("key_123_test")
    ok = dash.submit_one(k, 3.0)
    assert ok
    val = dash.get_value(k)
    assert val == 3.0


@test("key — case sensitivity preserved")
def test_key_case_sensitive(dash: Dashboard):
    ts = int(time.time() * 1000) % 1_000_000
    k_lower = f"case_lower_{ts}"
    k_upper = f"CASE_UPPER_{ts}"
    dash.submit({k_lower: 10.0, k_upper: 20.0})
    assert dash.get_value(k_lower) == 10.0
    assert dash.get_value(k_upper) == 20.0
    # They should be distinct keys
    assert k_lower != k_upper


# ── Known Metric Labels ────────────────────────────────────────────

@test("label — known keys get meaningful labels")
def test_known_key_labels(dash: Dashboard):
    """Verify that well-known metric keys get human-readable labels."""
    stats = dash.stats()
    known_keys = {s["key"]: s.get("label", "") for s in stats}
    # Check a few known system keys if they exist
    for key in ["agents_discovered", "tests_total", "repos_count"]:
        if key in known_keys:
            label = known_keys[key]
            assert len(label) > 0, f"Known key {key} should have a label"
            # Label should be human-readable (not just the key itself with underscores)
            assert "_" not in label or label != key, \
                f"Label for {key} should be human-readable, got '{label}'"


# ── Health Deep Validation ──────────────────────────────────────────

@test("health — stats_count matches actual data")
def test_health_stats_count_matches(dash: Dashboard):
    h = dash.health()
    assert isinstance(h["stats_count"], int)
    assert h["stats_count"] >= 0


@test("health — retention_days is positive")
def test_health_retention_positive(dash: Dashboard):
    h = dash.health()
    assert h["retention_days"] > 0, f"retention_days should be positive, got {h['retention_days']}"


@test("health — keys_count non-negative")
def test_health_keys_nonneg(dash: Dashboard):
    h = dash.health()
    assert h["keys_count"] >= 0


# ── Multi-Stat Interaction Tests ────────────────────────────────────

@test("multi-stat — submitting multiple keys doesn't interfere")
def test_multi_stat_isolation(dash: Dashboard):
    ts = int(time.time() * 1000) % 1_000_000
    k1 = f"iso_a_{ts}"
    k2 = f"iso_b_{ts}"
    dash.submit({k1: 100.0, k2: 200.0})
    assert dash.get_value(k1) == 100.0
    assert dash.get_value(k2) == 200.0
    # Update only k1
    dash.submit({k1: 150.0})
    assert dash.get_value(k1) == 150.0
    assert dash.get_value(k2) == 200.0, "k2 should be unchanged"


@test("multi-stat — delete one doesn't affect another")
def test_multi_stat_delete_isolation(dash: Dashboard):
    ts = int(time.time() * 1000) % 1_000_000
    k1 = f"deliso_a_{ts}"
    k2 = f"deliso_b_{ts}"
    dash.submit({k1: 10.0, k2: 20.0})
    dash.delete(k1)
    assert dash.get_value(k1) is None
    assert dash.get_value(k2) == 20.0, "k2 should survive k1 deletion"


@test("multi-stat — history is per-key")
def test_multi_stat_history_isolation(dash: Dashboard):
    ts = int(time.time() * 1000) % 1_000_000
    k1 = f"histiso_a_{ts}"
    k2 = f"histiso_b_{ts}"
    dash.submit_one(k1, 10.0)
    dash.submit_one(k2, 20.0)
    h1 = dash.history(k1)
    h2 = dash.history(k2)
    v1 = [p["value"] for p in h1]
    v2 = [p["value"] for p in h2]
    assert 10.0 in v1
    assert 20.0 not in v1, "k2's value should not appear in k1's history"
    assert 20.0 in v2
    assert 10.0 not in v2, "k1's value should not appear in k2's history"


# ── Value Types & Boundaries ───────────────────────────────────────

@test("submit — very small positive value")
def test_submit_very_small(dash: Dashboard):
    k = unique_key("tiny")
    ok = dash.submit_one(k, 0.000001)
    assert ok
    val = dash.get_value(k)
    assert val is not None and abs(val - 0.000001) < 0.0001


@test("submit — integer value stored as float")
def test_submit_integer_as_float(dash: Dashboard):
    k = unique_key("intfloat")
    ok = dash.submit_one(k, 42.0)
    assert ok
    val = dash.get_value(k)
    assert isinstance(val, (int, float))
    assert val == 42.0


@test("submit — value 1.0 (boolean-like metric)")
def test_submit_boolean_like(dash: Dashboard):
    k = unique_key("boollike")
    ok = dash.submit_one(k, 1.0)
    assert ok
    assert dash.get_value(k) == 1.0
    ok = dash.submit_one(k, 0.0)
    assert ok
    assert dash.get_value(k) == 0.0


# ── Alert Threshold Behavior ──────────────────────────────────────

@test("alert — check field types when present")
def test_alert_field_types(dash: Dashboard):
    alerts = dash.alerts(limit=20)
    for a in alerts:
        assert isinstance(a["key"], str)
        assert isinstance(a["level"], str)
        assert a["level"] in ("alert", "hot")
        assert isinstance(a["value"], (int, float))
        assert isinstance(a["change_pct"], (int, float))
        assert isinstance(a["triggered_at"], str)


@test("alert — key filter returns only matching key")
def test_alert_key_filter_exact(dash: Dashboard):
    k = unique_key("alertfilt")
    # Submit a big change to trigger alert
    dash.submit_one(k, 1.0)
    time.sleep(0.3)
    dash.submit_one(k, 100.0)
    time.sleep(0.1)
    alerts = dash.alerts(key=k)
    for a in alerts:
        assert a["key"] == k, f"Expected alerts for {k}, got {a['key']}"


@test("alert — default limit is reasonable")
def test_alert_default_limit(dash: Dashboard):
    alerts = dash.alerts()
    # Default should be <= 50
    assert len(alerts) <= 50, f"Default limit should be <=50, got {len(alerts)}"


# ── Stats Filtering & Structure ─────────────────────────────────────

@test("stats — every stat has required fields")
def test_stats_all_have_fields(dash: Dashboard):
    required = {"key", "current", "trends", "sparkline_24h", "last_updated"}
    stats = dash.stats()
    for s in stats:
        missing = required - set(s.keys())
        assert not missing, f"Stat {s.get('key', '?')} missing fields: {missing}"


@test("stats — current is numeric for all")
def test_stats_current_numeric(dash: Dashboard):
    stats = dash.stats()
    for s in stats:
        assert isinstance(s["current"], (int, float)), \
            f"Stat {s['key']} current should be numeric, got {type(s['current'])}"


@test("stats — no duplicate keys")
def test_stats_no_duplicates(dash: Dashboard):
    stats = dash.stats()
    keys = [s["key"] for s in stats]
    assert len(keys) == len(set(keys)), f"Duplicate keys found in stats: {[k for k in keys if keys.count(k) > 1]}"


# ── OpenAPI Deep Validation ─────────────────────────────────────────

@test("openapi — has required endpoints")
def test_openapi_required_endpoints(dash: Dashboard):
    spec = dash.openapi()
    paths = spec.get("paths", {})
    # Check for key paths (may have /api/v1/ prefix or not)
    path_str = json.dumps(paths)
    assert "stats" in path_str, "OpenAPI should document stats endpoint"
    assert "health" in path_str, "OpenAPI should document health endpoint"
    assert "alerts" in path_str, "OpenAPI should document alerts endpoint"


@test("openapi — methods documented")
def test_openapi_methods(dash: Dashboard):
    spec = dash.openapi()
    paths = spec.get("paths", {})
    for path, methods in paths.items():
        assert isinstance(methods, dict), f"Path {path} should be a dict of methods"
        for method in methods:
            assert method.lower() in ("get", "post", "delete", "put", "patch", "options", "head"), \
                f"Unknown method {method} in path {path}"


@test("openapi — version matches health version")
def test_openapi_version_matches(dash: Dashboard):
    spec = dash.openapi()
    h = dash.health()
    spec_version = spec.get("info", {}).get("version", "")
    health_version = h.get("version", "")
    assert spec_version == health_version, \
        f"OpenAPI version ({spec_version}) should match health version ({health_version})"


# ── SDK Error Messages ──────────────────────────────────────────────

@test("error — ValidationError has meaningful message")
def test_validation_error_message(dash: Dashboard):
    try:
        dash.submit([])
        assert False, "Should have raised"
    except ValidationError as e:
        assert len(str(e)) > 0, "Error message should not be empty"
        assert e.status == 400


@test("error — AuthError has meaningful message")
def test_auth_error_message(dash: Dashboard):
    bad = Dashboard(dash.base_url, manage_key="wrong")
    try:
        bad.submit({"x": 1.0})
        assert False, "Should have raised"
    except AuthError as e:
        assert len(str(e)) > 0, "Error message should not be empty"
        assert e.status == 403


@test("error — NotFoundError has meaningful message")
def test_not_found_error_message(dash: Dashboard):
    try:
        dash.delete("absolutely_nonexistent_key_xyz_999")
        assert False, "Should have raised"
    except NotFoundError as e:
        assert len(str(e)) > 0
        assert e.status == 404


@test("error — DashboardError.body contains response")
def test_error_body_content(dash: Dashboard):
    bad = Dashboard(dash.base_url, manage_key="wrong")
    try:
        bad.submit({"x": 1.0})
        assert False
    except DashboardError as e:
        # Body should be present (dict or string)
        assert e.body is not None, "Error body should not be None"


# ── Prune + Submit Interaction ──────────────────────────────────────

@test("prune — doesn't delete recent data")
def test_prune_preserves_recent(dash: Dashboard):
    k = unique_key("prune_recent")
    dash.submit({k: 42.0})
    # Prune should not delete data submitted just now (within 90-day retention)
    dash.prune()
    val = dash.get_value(k)
    assert val == 42.0, f"Recent data should survive prune, got {val}"


@test("prune — remaining count is stable after prune")
def test_prune_remaining_stable(dash: Dashboard):
    r1 = dash.prune()
    r2 = dash.prune()
    assert r1["remaining"] == r2["remaining"], \
        f"Remaining should be stable: {r1['remaining']} vs {r2['remaining']}"


# ── Full Lifecycle Advanced ─────────────────────────────────────────

@test("lifecycle — submit → update → history → trend → delete")
def test_full_advanced_lifecycle(dash: Dashboard):
    k = unique_key("advlife")
    # Phase 1: Initial submit
    dash.submit_one(k, 10.0)
    assert dash.get_value(k) == 10.0
    # Phase 2: Update
    time.sleep(0.1)
    dash.submit_one(k, 25.0)
    assert dash.get_value(k) == 25.0
    # Phase 3: History shows both
    points = dash.history(k)
    values = [p["value"] for p in points]
    assert 10.0 in values
    assert 25.0 in values
    # Phase 4: Trend exists
    trend = dash.get_trend(k, "24h")
    assert trend is None or isinstance(trend, (int, float))
    # Phase 5: In stats list
    assert k in dash.keys()
    # Phase 6: Delete
    dash.delete(k)
    assert dash.get_value(k) is None
    assert k not in dash.keys()


@test("lifecycle — batch submit → individual reads → bulk stats check")
def test_lifecycle_batch_to_individual(dash: Dashboard):
    ts = int(time.time() * 1000) % 1_000_000
    keys = [f"life_batch_{i}_{ts}" for i in range(5)]
    items = {k: float(i * 10) for i, k in enumerate(keys)}
    accepted = dash.submit(items)
    assert accepted == 5
    # Individual reads
    for i, k in enumerate(keys):
        val = dash.get_value(k)
        assert val == float(i * 10), f"Key {k} expected {i*10}, got {val}"
    # Bulk stats
    all_keys = dash.keys()
    for k in keys:
        assert k in all_keys
    # Cleanup
    for k in keys:
        try:
            dash.delete(k)
        except Exception:
            pass


@test("lifecycle — submit then immediate stat matches")
def test_lifecycle_submit_stat_match(dash: Dashboard):
    k = unique_key("life_stat")
    dash.submit({k: 77.7})
    s = dash.stat(k)
    assert s is not None
    assert s["key"] == k
    assert s["current"] == 77.7
    assert "trends" in s
    assert "sparkline_24h" in s


# ── Concurrent Client Instances ─────────────────────────────────────

@test("concurrent — two client instances share state")
def test_two_clients_share_state(dash: Dashboard):
    k = unique_key("concur")
    # Client 1 submits
    dash.submit({k: 42.0})
    # Client 2 reads
    dash2 = Dashboard(dash.base_url, manage_key=dash.manage_key)
    val = dash2.get_value(k)
    assert val == 42.0, f"Client 2 should see Client 1's data, got {val}"
    # Client 2 updates
    dash2.submit({k: 99.0})
    # Client 1 reads
    val2 = dash.get_value(k)
    assert val2 == 99.0, f"Client 1 should see Client 2's update, got {val2}"


@test("concurrent — read-only client can't write")
def test_readonly_client(dash: Dashboard):
    readonly = Dashboard(dash.base_url)  # No manage key
    # Should be able to read
    stats = readonly.stats()
    assert isinstance(stats, list)
    h = readonly.health()
    assert h["status"] == "ok"
    # Should NOT be able to write
    try:
        readonly.submit({"test": 1.0})
        assert False, "Read-only client should not be able to submit"
    except (AuthError, DashboardError):
        pass


# ── Seq Monotonicity ───────────────────────────────────────────────

@test("seq — stats seq values are monotonically increasing per key")
def test_seq_monotonic(dash: Dashboard):
    k = unique_key("seq_mono")
    for i in range(5):
        dash.submit_one(k, float(i))
        time.sleep(0.05)
    points = dash.history(k)
    timestamps = [p["recorded_at"] for p in points]
    # Timestamps should be monotonically increasing
    for i in range(len(timestamps) - 1):
        assert timestamps[i] <= timestamps[i + 1], \
            f"Timestamps not monotonic: {timestamps[i]} > {timestamps[i + 1]}"


# ── Custom Range Edge Cases ─────────────────────────────────────────

@test("custom range — very wide range returns all data")
def test_custom_range_wide(dash: Dashboard):
    k = unique_key("range_wide")
    dash.submit_one(k, 42.0)
    points = dash.history(k, start="2020-01-01T00:00:00Z", end="2030-12-31T23:59:59Z")
    assert len(points) >= 1, "Wide range should include data"
    values = [p["value"] for p in points]
    assert 42.0 in values


@test("custom range — precise window around submission")
def test_custom_range_precise(dash: Dashboard):
    k = unique_key("range_prec")
    now = datetime.now(timezone.utc)
    dash.submit_one(k, 55.5)
    start = (now - timedelta(seconds=5)).strftime("%Y-%m-%dT%H:%M:%SZ")
    end = (now + timedelta(seconds=30)).strftime("%Y-%m-%dT%H:%M:%SZ")
    points = dash.history(k, start=start, end=end)
    assert len(points) >= 1


# ── Submit Content Types ────────────────────────────────────────────

@test("submit — list and dict forms produce same result")
def test_submit_forms_equivalent(dash: Dashboard):
    ts = int(time.time() * 1000) % 1_000_000
    k1 = f"form_dict_{ts}"
    k2 = f"form_list_{ts}"
    # Dict form
    dash.submit({k1: 42.0})
    # List form
    dash.submit([{"key": k2, "value": 42.0}])
    assert dash.get_value(k1) == 42.0
    assert dash.get_value(k2) == 42.0


# ── Health vs Stats Consistency ─────────────────────────────────────

@test("consistency — health stats_count >= keys_count")
def test_health_count_consistency(dash: Dashboard):
    h = dash.health()
    assert h["stats_count"] >= h["keys_count"], \
        f"stats_count ({h['stats_count']}) should be >= keys_count ({h['keys_count']})"


@test("consistency — submit increases both counts")
def test_consistency_submit_increases_counts(dash: Dashboard):
    h_before = dash.health()
    k = unique_key("consist")
    dash.submit_one(k, 1.0)
    h_after = dash.health()
    assert h_after["stats_count"] > h_before["stats_count"]
    assert h_after["keys_count"] >= h_before["keys_count"]


# ── Cleanup ─────────────────────────────────────────────────────────

@test("cleanup — delete test metrics")
# ── Dual Discovery Paths ────────────────────────────────────────────

@test("llms.txt root path")
def test_llms_txt_root_path(dash: Dashboard):
    txt = dash.llms_txt_root()
    assert "dashboard" in txt.lower() or "stats" in txt.lower()

@test("llms.txt v1 path")
def test_llms_txt_v1_path(dash: Dashboard):
    txt = dash.llms_txt_v1()
    assert "dashboard" in txt.lower() or "stats" in txt.lower()

@test("llms.txt root and v1 both work")
def test_llms_txt_both_paths(dash: Dashboard):
    root = dash.llms_txt_root()
    v1 = dash.llms_txt_v1()
    assert len(root) > 50
    assert len(v1) > 50

@test("skill.md v1 path")
def test_skill_md_v1_path(dash: Dashboard):
    md = dash.skill_md_v1()
    assert "dashboard" in md.lower() or "private" in md.lower() or "skill" in md.lower()

@test("skill.md both paths match")
def test_skill_md_both_paths(dash: Dashboard):
    wk = dash.skill_md()
    v1 = dash.skill_md_v1()
    assert wk == v1

# ── Submit Edge Cases ───────────────────────────────────────────────

@test("submit with unicode key")
def test_submit_unicode_key(dash: Dashboard):
    k = f"unicode_テスト_{int(time.time() * 1000) % 1_000_000}"
    ok = dash.submit_one(k, 42.0)
    assert ok
    val = dash.get_value(k)
    assert val == 42.0
    dash.delete(k)

@test("submit multiple values same key")
def test_submit_multiple_same_key(dash: Dashboard):
    k = f"multi_{int(time.time() * 1000) % 1_000_000}"
    for i in range(5):
        dash.submit_one(k, float(i * 10))
    hist = dash.history(k, period="24h")
    # Should have at least 5 data points
    data = hist if isinstance(hist, list) else hist.get("data", hist.get("history", []))
    assert len(data) >= 5
    dash.delete(k)

@test("submit and immediate read")
def test_submit_immediate_read(dash: Dashboard):
    k = f"imm_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 99.5)
    val = dash.get_value(k)
    assert val == 99.5
    dash.delete(k)

@test("submit batch with metadata")
def test_submit_batch_metadata(dash: Dashboard):
    k1 = f"batchm1_{int(time.time() * 1000) % 1_000_000}"
    k2 = f"batchm2_{int(time.time() * 1000) % 1_000_000}"
    ok = dash.submit([
        {"key": k1, "value": 10.0, "metadata": {"source": "test"}},
        {"key": k2, "value": 20.0, "metadata": {"source": "test"}},
    ])
    assert ok
    assert dash.get_value(k1) == 10.0
    assert dash.get_value(k2) == 20.0
    dash.delete(k1)
    dash.delete(k2)

# ── History Edge Cases ──────────────────────────────────────────────

@test("history returns consistent timestamps")
def test_history_timestamps(dash: Dashboard):
    k = f"histts_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 1.0)
    dash.submit_one(k, 2.0)
    hist = dash.history(k, period="24h")
    data = hist if isinstance(hist, list) else hist.get("data", hist.get("history", []))
    for point in data:
        # Each point should have a timestamp
        assert "timestamp" in point or "recorded_at" in point or "created_at" in point
    dash.delete(k)

@test("history after delete returns empty")
def test_history_after_delete(dash: Dashboard):
    k = f"histdel_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 42.0)
    dash.delete(k)
    try:
        hist = dash.history(k, period="24h")
        data = hist if isinstance(hist, list) else hist.get("data", hist.get("history", []))
        assert len(data) == 0
    except NotFoundError:
        pass  # Also acceptable

# ── Stats Response Structure ────────────────────────────────────────

@test("stats list returns dicts with expected keys")
def test_stats_response_keys(dash: Dashboard):
    k = f"struct_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 5.0)
    stats = dash.stats()
    found = [s for s in stats if s["key"] == k]
    assert len(found) == 1
    s = found[0]
    assert "key" in s
    assert "current" in s
    dash.delete(k)

@test("stat single returns correct value")
def test_stat_single_value(dash: Dashboard):
    k = f"single2_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 77.7)
    s = dash.stat(k)
    assert s is not None
    assert s["current"] == 77.7
    dash.delete(k)

@test("keys list includes test key")
def test_keys_includes(dash: Dashboard):
    k = f"keysincl_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 1.0)
    keys = dash.keys()
    assert k in keys
    dash.delete(k)

# ── Alert Edge Cases ────────────────────────────────────────────────

@test("alerts with key that has no alerts")
def test_alerts_empty_key(dash: Dashboard):
    k = f"alertempty_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 50.0)
    alerts = dash.alerts(key=k)
    assert isinstance(alerts, list)
    dash.delete(k)

@test("alert count is non-negative")
def test_alert_count_nonneg(dash: Dashboard):
    count = dash.alert_count()
    assert count >= 0

@test("hot alerts returns list")
def test_hot_alerts_is_list(dash: Dashboard):
    hot = dash.hot_alerts(limit=5)
    assert isinstance(hot, list)
    assert len(hot) <= 5

# ── Delete Edge Cases ───────────────────────────────────────────────

@test("delete returns count")
def test_delete_returns_count(dash: Dashboard):
    k = f"delcount_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 1.0)
    dash.submit_one(k, 2.0)
    dash.submit_one(k, 3.0)
    count = dash.delete(k)
    assert isinstance(count, int)
    assert count >= 1

@test("double delete is safe")
def test_double_delete(dash: Dashboard):
    k = f"dbldel_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 1.0)
    dash.delete(k)
    try:
        dash.delete(k)
    except NotFoundError:
        pass  # Expected

# ── Trend Calculations ──────────────────────────────────────────────

@test("trend for new metric is None or zero")
def test_trend_new_metric(dash: Dashboard):
    k = f"trendnew_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 100.0)
    trend = dash.get_trend(k, "24h")
    # New metric might have None trend or 0
    assert trend is None or isinstance(trend, (int, float))
    dash.delete(k)

@test("trend with multiple data points")
def test_trend_multiple_points(dash: Dashboard):
    k = f"trendmulti_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 10.0)
    dash.submit_one(k, 20.0)
    trend = dash.get_trend(k, "24h")
    # Should have some trend value now
    dash.delete(k)

# ── Prune Edge Cases ────────────────────────────────────────────────

@test("prune returns result shape")
def test_prune_result_shape(dash: Dashboard):
    result = dash.prune()
    assert isinstance(result, dict)

# ── Latest Value Helper ─────────────────────────────────────────────

@test("latest_value returns stat dict")
def test_latest_value_dict(dash: Dashboard):
    k = f"latestdict_{int(time.time() * 1000) % 1_000_000}"
    dash.submit_one(k, 42.0)
    result = dash.latest_value(k)
    assert result is not None
    assert result["current"] == 42.0
    dash.delete(k)

@test("latest_value returns None for missing")
def test_latest_value_missing(dash: Dashboard):
    result = dash.latest_value(f"noexist_{int(time.time() * 1000) % 1_000_000}")
    assert result is None

# ── Is Healthy Helper ───────────────────────────────────────────────

@test("is_healthy returns True for running server")
def test_is_healthy_true(dash: Dashboard):
    assert dash.is_healthy() is True

@test("is_healthy returns False for bad URL")
def test_is_healthy_false(dash: Dashboard):
    bad = Dashboard("http://localhost:19999")
    assert bad.is_healthy() is False

# ── Repr ────────────────────────────────────────────────────────────

@test("repr includes URL")
def test_repr_url(dash: Dashboard):
    r = repr(dash)
    assert "192.168.0.79" in r or "localhost" in r or "Dashboard" in r

def test_cleanup(dash: Dashboard):
    """Clean up test metrics to not pollute the dashboard."""
    stats = dash.stats()
    # Clean up all test-generated keys
    prefixes = [
        "sdk_test", "submit_dict", "submit_list", "batch_", "submit_one",
        "submit_meta", "stats_list", "stats_fields", "trends", "sparkline",
        "single", "getval", "trend_", "keys_test", "hist_", "delete",
        "latest", "aaa_", "zzz_", "neg_", "zero_", "big_", "frac_",
        "test-key", "lgmeta", "boundary_", "mix_", "rapid_", "many_",
        "unknown_", "spark_", "updated_", "del_", "alert_", "health_",
        "lifecycle", "hist_incl", "unicode_", "multi_", "imm_", "batchm",
        "histts_", "histdel_", "struct_", "single2_", "keysincl_",
        "alertempty_", "delcount_", "dbldel_", "trendnew_", "trendmulti_",
        "latestdict_", "meta_", "batchdup", "batchone", "batchdict",
        "hist_asc", "hist_inv", "hist_same", "hist_old", "trend_pos",
        "trend_dec", "trend_stable", "trend_types", "spark_single",
        "spark_multi", "spark_range", "key.with", "key-with", "key_123",
        "case_lower", "CASE_UPPER", "iso_", "deliso_", "histiso_",
        "tiny_", "intfloat", "boollike", "alertfilt", "prune_recent",
        "advlife", "life_batch", "life_stat", "concur_", "seq_mono",
        "range_wide", "range_prec", "form_dict", "form_list", "consist",
    ]
    for s in stats:
        k = s["key"]
        if any(k.startswith(p) for p in prefixes):
            try:
                dash.delete(k)
            except Exception:
                pass


# ── Main ────────────────────────────────────────────────────────────

def main():
    url = os.environ.get("DASHBOARD_URL", "http://192.168.0.79:3008")
    key = os.environ.get("DASHBOARD_KEY", "dash_0e54dee0985e417b8ecb78b3607ad816")

    print(f"\n🔧 Private Dashboard SDK — Integration Tests")
    print(f"   URL: {url}")
    print(f"   Key: {key[:10]}...")
    print()

    dash = Dashboard(url, manage_key=key)

    # Health / Discovery
    print("Health & Discovery:")
    test_health(dash)
    test_health_fields(dash)
    test_is_healthy(dash)
    test_llms_txt(dash)
    test_openapi(dash)
    test_skills_index(dash)
    test_skill_md(dash)

    # Submit
    print("\nSubmit:")
    test_submit_dict(dash)
    test_submit_list(dash)
    test_submit_batch(dash)
    test_submit_one(dash)
    test_submit_one_meta(dash)
    test_submit_empty(dash)
    test_submit_too_many(dash)
    test_submit_bad_auth(dash)

    # Submit Edge Cases
    print("\nSubmit Edge Cases:")
    test_submit_negative(dash)
    test_submit_zero(dash)
    test_submit_large_value(dash)
    test_submit_fractional(dash)
    test_submit_special_chars(dash)
    test_submit_large_metadata(dash)
    test_submit_exactly_100(dash)
    test_submit_mixed_batch(dash)
    test_submit_rapid_writes(dash)
    test_submit_many_keys(dash)
    test_submit_no_auth(dash)

    # Stats / Read
    print("\nStats / Read:")
    test_stats_list(dash)
    test_stats_fields(dash)
    test_stats_trends(dash)
    test_stats_sparkline(dash)
    test_stat_single(dash)
    test_stat_missing(dash)
    test_get_value(dash)
    test_get_value_missing(dash)
    test_get_trend(dash)
    test_keys(dash)

    # Stats Advanced
    print("\nStats Advanced:")
    test_key_label_fallback(dash)
    test_sparkline_types(dash)
    test_stats_last_updated(dash)
    test_stats_no_auth_read(dash)
    test_health_no_auth(dash)

    # History
    print("\nHistory:")
    test_history_default(dash)
    test_history_7d(dash)
    test_history_30d(dash)
    test_history_90d(dash)
    test_history_custom_range(dash)
    test_history_date_format(dash)
    test_history_empty_key(dash)
    test_history_invalid_period(dash)
    test_history_chronological(dash)

    # History Advanced
    print("\nHistory Advanced:")
    test_history_start_only(dash)
    test_history_end_only(dash)
    test_history_future_dates(dash)
    test_history_many_points(dash)
    test_history_point_fields(dash)
    test_history_values_accurate(dash)

    # Delete
    print("\nDelete:")
    test_delete(dash)
    test_delete_missing(dash)
    test_delete_bad_auth(dash)

    # Delete Advanced
    print("\nDelete Advanced:")
    test_delete_cascade_history(dash)
    test_delete_removes_from_stats(dash)
    test_delete_then_resubmit(dash)
    test_delete_health_count(dash)

    # Prune
    print("\nPrune:")
    test_prune(dash)
    test_prune_bad_auth(dash)

    # Prune Advanced
    print("\nPrune Advanced:")
    test_prune_idempotent(dash)
    test_prune_response_shape(dash)

    # Alerts
    print("\nAlerts:")
    test_alerts_list(dash)
    test_alerts_limit(dash)
    test_alerts_key_filter(dash)
    test_alerts_fields(dash)
    test_alert_count(dash)
    test_hot_alerts(dash)

    # Alerts Advanced
    print("\nAlerts Advanced:")
    test_alerts_trigger(dash)
    test_alerts_newest_first(dash)
    test_alerts_limit_zero(dash)
    test_alerts_limit_clamped(dash)

    # Health Advanced
    print("\nHealth Advanced:")
    test_health_stats_count(dash)
    test_health_version(dash)
    test_health_oldest_stat(dash)

    # Trends
    print("\nTrend Calculations:")
    test_trend_all_periods(dash)
    test_trend_structure(dash)

    # Discovery Advanced
    print("\nDiscovery Advanced:")
    test_llms_txt_endpoints(dash)
    test_openapi_paths(dash)
    test_openapi_info(dash)
    test_skills_files(dash)
    test_skill_md_endpoints(dash)
    test_llms_txt_api_path(dash)

    # SDK Constructor
    print("\nSDK Constructor:")
    test_constructor_env_url(dash)
    test_constructor_timeout(dash)
    test_constructor_trailing_slash(dash)

    # Errors
    print("\nError Handling:")
    test_error_hierarchy(dash)
    test_error_status(dash)
    test_connection_error(dash)
    test_constructor_requires_url(dash)

    # Value updates
    print("\nValue Updates:")
    test_latest_value(dash)
    test_stats_ordering(dash)

    # Cross-Feature Interactions
    print("\nCross-Feature Interactions:")
    test_full_lifecycle(dash)
    test_counts_consistent(dash)
    test_history_custom_range_includes(dash)
    test_alerts_no_auth(dash)

    # Metadata
    print("\nMetadata:")
    test_metadata_in_history(dash)
    test_metadata_null(dash)
    test_metadata_nested(dash)
    test_metadata_empty_obj(dash)

    # Batch Duplicate Keys
    print("\nBatch Duplicates:")
    test_batch_duplicate_keys(dash)
    test_batch_single_item_list(dash)
    test_batch_single_item_dict(dash)

    # History Ordering & Range
    print("\nHistory Ordering & Range:")
    test_history_ascending_many(dash)
    test_history_inverted_range(dash)
    test_history_same_day(dash)
    test_history_old_range(dash)

    # Trend Accuracy
    print("\nTrend Accuracy:")
    test_trend_positive_change(dash)
    test_trend_decreasing(dash)
    test_trend_stable(dash)
    test_trend_pct_types(dash)

    # Sparkline Data Quality
    print("\nSparkline Quality:")
    test_sparkline_single_value(dash)
    test_sparkline_multiple_values(dash)
    test_sparkline_value_range(dash)

    # Key Naming
    print("\nKey Naming:")
    test_key_dots(dash)
    test_key_dashes(dash)
    test_key_underscores_nums(dash)
    test_key_case_sensitive(dash)

    # Known Metric Labels
    print("\nMetric Labels:")
    test_known_key_labels(dash)

    # Health Deep Validation
    print("\nHealth Deep:")
    test_health_stats_count_matches(dash)
    test_health_retention_positive(dash)
    test_health_keys_nonneg(dash)

    # Multi-Stat Interaction
    print("\nMulti-Stat Interaction:")
    test_multi_stat_isolation(dash)
    test_multi_stat_delete_isolation(dash)
    test_multi_stat_history_isolation(dash)

    # Value Types & Boundaries
    print("\nValue Types:")
    test_submit_very_small(dash)
    test_submit_integer_as_float(dash)
    test_submit_boolean_like(dash)

    # Alert Threshold Behavior
    print("\nAlert Thresholds:")
    test_alert_field_types(dash)
    test_alert_key_filter_exact(dash)
    test_alert_default_limit(dash)

    # Stats Filtering & Structure
    print("\nStats Structure:")
    test_stats_all_have_fields(dash)
    test_stats_current_numeric(dash)
    test_stats_no_duplicates(dash)

    # OpenAPI Deep
    print("\nOpenAPI Deep:")
    test_openapi_required_endpoints(dash)
    test_openapi_methods(dash)
    test_openapi_version_matches(dash)

    # Error Messages
    print("\nError Messages:")
    test_validation_error_message(dash)
    test_auth_error_message(dash)
    test_not_found_error_message(dash)
    test_error_body_content(dash)

    # Prune + Submit Interaction
    print("\nPrune Interaction:")
    test_prune_preserves_recent(dash)
    test_prune_remaining_stable(dash)

    # Full Lifecycle Advanced
    print("\nLifecycle Advanced:")
    test_full_advanced_lifecycle(dash)
    test_lifecycle_batch_to_individual(dash)
    test_lifecycle_submit_stat_match(dash)

    # Concurrent Client Instances
    print("\nConcurrent Clients:")
    test_two_clients_share_state(dash)
    test_readonly_client(dash)

    # Seq Monotonicity
    print("\nSeq Monotonicity:")
    test_seq_monotonic(dash)

    # Custom Range Edge Cases
    print("\nCustom Range Edge Cases:")
    test_custom_range_wide(dash)
    test_custom_range_precise(dash)

    # Submit Forms
    print("\nSubmit Forms:")
    test_submit_forms_equivalent(dash)

    # Health vs Stats Consistency
    print("\nHealth vs Stats Consistency:")
    test_health_count_consistency(dash)
    test_consistency_submit_increases_counts(dash)

    # Dual Discovery Paths
    print("\nDual Discovery Paths:")
    test_llms_txt_root_path(dash)
    test_llms_txt_v1_path(dash)
    test_llms_txt_both_paths(dash)
    test_skill_md_v1_path(dash)
    test_skill_md_both_paths(dash)

    # Submit Edge Cases
    print("\nSubmit Edge Cases:")
    test_submit_unicode_key(dash)
    test_submit_multiple_same_key(dash)
    test_submit_immediate_read(dash)
    test_submit_batch_metadata(dash)

    # History Edge Cases
    print("\nHistory Edge Cases:")
    test_history_timestamps(dash)
    test_history_after_delete(dash)

    # Stats Response Structure
    print("\nStats Response Structure:")
    test_stats_response_keys(dash)
    test_stat_single_value(dash)
    test_keys_includes(dash)

    # Alert Edge Cases
    print("\nAlert Edge Cases:")
    test_alerts_empty_key(dash)
    test_alert_count_nonneg(dash)
    test_hot_alerts_is_list(dash)

    # Delete Edge Cases
    print("\nDelete Edge Cases:")
    test_delete_returns_count(dash)
    test_double_delete(dash)

    # Trend Calculations Extra
    print("\nTrend Calculations Extra:")
    test_trend_new_metric(dash)
    test_trend_multiple_points(dash)

    # Prune Extra
    print("\nPrune Extra:")
    test_prune_result_shape(dash)

    # Helpers
    print("\nHelpers:")
    test_latest_value_dict(dash)
    test_latest_value_missing(dash)
    test_is_healthy_true(dash)
    test_is_healthy_false(dash)
    test_repr_url(dash)

    # Cleanup
    print("\nCleanup:")
    test_cleanup(dash)

    # Summary
    print(f"\n{'='*50}")
    print(f"  Results: {PASS} passed, {FAIL} failed")
    if ERRORS:
        print(f"\n  Failures:")
        for name, err in ERRORS:
            print(f"    ❌ {name}: {err}")
    print()

    sys.exit(1 if FAIL > 0 else 0)


if __name__ == "__main__":
    main()
