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

@test("cleanup — delete test metrics")
def test_cleanup(dash: Dashboard):
    """Clean up test metrics to not pollute the dashboard."""
    stats = dash.stats()
    test_keys = [s["key"] for s in stats if s["key"].startswith("sdk_test")]
    for k in test_keys:
        try:
            dash.delete(k)
        except Exception:
            pass
    # Also clean up by other prefixes
    for prefix in ["submit_dict", "submit_list", "batch_", "submit_one",
                    "submit_meta", "stats_list", "stats_fields", "trends",
                    "sparkline", "single", "getval", "trend_new", "keys_test",
                    "hist_", "delete", "latest", "aaa_", "zzz_"]:
        remaining = [s["key"] for s in dash.stats() if s["key"].startswith(prefix)]
        for k in remaining:
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

    # Delete
    print("\nDelete:")
    test_delete(dash)
    test_delete_missing(dash)
    test_delete_bad_auth(dash)

    # Prune
    print("\nPrune:")
    test_prune(dash)
    test_prune_bad_auth(dash)

    # Alerts
    print("\nAlerts:")
    test_alerts_list(dash)
    test_alerts_limit(dash)
    test_alerts_key_filter(dash)
    test_alerts_fields(dash)
    test_alert_count(dash)
    test_hot_alerts(dash)

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
