"""
Private Dashboard Python SDK — Zero-dependency client for The Pack agent operations dashboard.

Usage:
    from dashboard import Dashboard

    dash = Dashboard("http://localhost:3008", manage_key="dash_abc123")

    # Submit metrics
    dash.submit({"tests_total": 1500, "repos_count": 9})

    # Read all metrics with trends
    stats = dash.stats()
    for s in stats:
        print(f"{s['label']}: {s['current']} ({s['trends']['24h'].get('pct', 'n/a')}% 24h)")

    # History for a single metric
    points = dash.history("tests_total", period="7d")

    # Alerts
    alerts = dash.alerts(limit=10)

Requires: Python 3.8+, no external dependencies.
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, Dict, List, Optional, Union


# ── Errors ──────────────────────────────────────────────────────────

class DashboardError(Exception):
    """Base error for dashboard client."""
    def __init__(self, message: str, status: int = 0, body: Any = None):
        super().__init__(message)
        self.status = status
        self.body = body


class AuthError(DashboardError):
    """403 — invalid manage key."""
    pass


class NotFoundError(DashboardError):
    """404 — resource not found."""
    pass


class ValidationError(DashboardError):
    """400 — bad request / validation failure."""
    pass


class RateLimitError(DashboardError):
    """429 — rate limited."""
    def __init__(self, message: str, status: int = 429, body: Any = None, retry_after: Optional[float] = None):
        super().__init__(message, status, body)
        self.retry_after = retry_after


class ServerError(DashboardError):
    """500+ — server-side error."""
    pass


# ── Client ──────────────────────────────────────────────────────────

class Dashboard:
    """
    Python client for The Pack — Private Dashboard API.

    Args:
        base_url: Dashboard base URL (e.g. "http://localhost:3008").
                  Falls back to DASHBOARD_URL env var.
        manage_key: Write-access key. Falls back to DASHBOARD_KEY env var.
                    Only required for write operations (submit, prune, delete).
        timeout: HTTP request timeout in seconds (default 10).
    """

    def __init__(
        self,
        base_url: Optional[str] = None,
        manage_key: Optional[str] = None,
        timeout: int = 10,
    ):
        self.base_url = (base_url or os.environ.get("DASHBOARD_URL", "")).rstrip("/")
        if not self.base_url:
            raise ValueError("base_url required (or set DASHBOARD_URL env var)")
        self.manage_key = manage_key or os.environ.get("DASHBOARD_KEY", "")
        self.timeout = timeout

    # ── Core API ────────────────────────────────────────────────────

    def health(self) -> Dict[str, Any]:
        """
        GET /api/v1/health — service status, version, stat counts.

        Returns:
            {"status": "ok", "version": str, "stats_count": int,
             "keys_count": int, "retention_days": int, "oldest_stat": str|None}
        """
        return self._get("/api/v1/health")

    def stats(self) -> List[Dict[str, Any]]:
        """
        GET /api/v1/stats — all metrics with latest value, trends, sparklines.

        Returns list of stat summaries, each containing:
            key, label, current, trends (24h/7d/30d/90d), sparkline_24h, last_updated
        """
        resp = self._get("/api/v1/stats")
        return resp.get("stats", [])

    def stat(self, key: str) -> Optional[Dict[str, Any]]:
        """
        Get a single metric from the stats list.

        Args:
            key: Metric key (e.g. "tests_total").

        Returns:
            Stat summary dict, or None if key not found.
        """
        for s in self.stats():
            if s.get("key") == key:
                return s
        return None

    def history(
        self,
        key: str,
        period: Optional[str] = None,
        start: Optional[str] = None,
        end: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """
        GET /api/v1/stats/<key> — time-series history for a metric.

        Args:
            key: Metric key.
            period: "24h", "7d", "30d", or "90d" (default "24h").
            start: Custom range start (ISO-8601 or YYYY-MM-DD). Requires end.
            end: Custom range end (ISO-8601 or YYYY-MM-DD). Requires start.

        Returns:
            List of {"value": float, "recorded_at": str} points.
        """
        params: Dict[str, str] = {}
        if start and end:
            params["start"] = start
            params["end"] = end
        elif period:
            params["period"] = period

        resp = self._get(f"/api/v1/stats/{urllib.parse.quote(key, safe='')}", params=params)
        return resp.get("points", [])

    def submit(
        self,
        metrics: Union[Dict[str, float], List[Dict[str, Any]]],
    ) -> int:
        """
        POST /api/v1/stats — submit metric values (auth required).

        Args:
            metrics: Either a dict of {key: value} pairs for simple submission,
                     or a list of {"key": str, "value": float, "metadata?": dict} objects.

        Returns:
            Number of accepted metrics.

        Examples:
            # Simple dict form
            dash.submit({"tests_total": 1500, "repos_count": 9})

            # Full form with metadata
            dash.submit([
                {"key": "tests_total", "value": 1500, "metadata": {"source": "CI"}},
            ])
        """
        if isinstance(metrics, dict):
            body = [{"key": k, "value": v} for k, v in metrics.items()]
        else:
            body = metrics

        resp = self._post("/api/v1/stats", body, auth=True)
        return resp.get("accepted", 0)

    def delete(self, key: str) -> int:
        """
        DELETE /api/v1/stats/<key> — delete all data for a metric (auth required).

        Args:
            key: Metric key to delete.

        Returns:
            Number of deleted data points.
        """
        resp = self._delete(f"/api/v1/stats/{urllib.parse.quote(key, safe='')}", auth=True)
        return resp.get("deleted", 0)

    def prune(self) -> Dict[str, Any]:
        """
        POST /api/v1/stats/prune — trigger data retention cleanup (auth required).

        Returns:
            {"deleted": int, "retention_days": int, "remaining": int}
        """
        return self._post("/api/v1/stats/prune", None, auth=True)

    def alerts(
        self,
        key: Optional[str] = None,
        limit: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """
        GET /api/v1/alerts — alert history (significant metric changes).

        Args:
            key: Filter alerts for a specific metric key.
            limit: Max alerts to return (1-500, default 50).

        Returns:
            List of alert dicts with key, label, level, value, change_pct, triggered_at.
        """
        params: Dict[str, str] = {}
        if key:
            params["key"] = key
        if limit is not None:
            params["limit"] = str(limit)

        resp = self._get("/api/v1/alerts", params=params)
        return resp.get("alerts", [])

    def alert_count(self) -> int:
        """Get total number of recorded alerts."""
        resp = self._get("/api/v1/alerts", params={"limit": "1"})
        return resp.get("total", 0)

    # ── Discovery ───────────────────────────────────────────────────

    def llms_txt(self) -> str:
        """GET /llms.txt — AI-readable API summary."""
        return self._get_text("/llms.txt")

    def openapi(self) -> Dict[str, Any]:
        """GET /openapi.json — OpenAPI 3.0.3 specification."""
        return self._get("/openapi.json")

    def skills_index(self) -> Dict[str, Any]:
        """GET /.well-known/skills/index.json — skills discovery index."""
        return self._get("/.well-known/skills/index.json")

    def skill_md(self) -> str:
        """GET /.well-known/skills/private-dashboard/SKILL.md — integration skill."""
        return self._get_text("/.well-known/skills/private-dashboard/SKILL.md")

    # ── Convenience Helpers ─────────────────────────────────────────

    def get_value(self, key: str) -> Optional[float]:
        """Get the current value of a metric, or None if not found."""
        s = self.stat(key)
        return s["current"] if s else None

    def get_trend(self, key: str, period: str = "24h") -> Optional[float]:
        """
        Get the percentage trend for a metric over a period.

        Args:
            key: Metric key.
            period: "24h", "7d", "30d", or "90d".

        Returns:
            Percentage change, or None if not available.
        """
        s = self.stat(key)
        if not s:
            return None
        trend = s.get("trends", {}).get(period, {})
        return trend.get("pct")

    def submit_one(self, key: str, value: float, metadata: Optional[Dict] = None) -> bool:
        """
        Submit a single metric value.

        Args:
            key: Metric key.
            value: Numeric value.
            metadata: Optional JSON metadata.

        Returns:
            True if accepted.
        """
        item: Dict[str, Any] = {"key": key, "value": value}
        if metadata:
            item["metadata"] = metadata
        return self.submit([item]) >= 1

    def is_healthy(self) -> bool:
        """Check if the dashboard service is healthy."""
        try:
            h = self.health()
            return h.get("status") == "ok"
        except DashboardError:
            return False

    def hot_alerts(self, limit: int = 20) -> List[Dict[str, Any]]:
        """Get only 'hot' level alerts (>=25% change)."""
        return [a for a in self.alerts(limit=limit) if a.get("level") == "hot"]

    def keys(self) -> List[str]:
        """Get list of all metric keys currently tracked."""
        return [s["key"] for s in self.stats()]

    # ── HTTP Internals ──────────────────────────────────────────────

    def _get(self, path: str, params: Optional[Dict[str, str]] = None) -> Any:
        url = self.base_url + path
        if params:
            url += "?" + urllib.parse.urlencode(params)
        return self._request("GET", url)

    def _get_text(self, path: str) -> str:
        url = self.base_url + path
        req = urllib.request.Request(url, method="GET")
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                return resp.read().decode("utf-8")
        except urllib.error.HTTPError as e:
            self._handle_http_error(e)
        except urllib.error.URLError as e:
            raise DashboardError(f"Connection error: {e.reason}")
        return ""  # unreachable

    def _post(self, path: str, body: Any, auth: bool = False) -> Any:
        url = self.base_url + path
        data = json.dumps(body).encode("utf-8") if body is not None else b""
        headers = {"Content-Type": "application/json"}
        if auth:
            headers["Authorization"] = f"Bearer {self.manage_key}"
        return self._request("POST", url, data=data, headers=headers)

    def _delete(self, path: str, auth: bool = False) -> Any:
        url = self.base_url + path
        headers = {}
        if auth:
            headers["Authorization"] = f"Bearer {self.manage_key}"
        return self._request("DELETE", url, headers=headers)

    def _request(self, method: str, url: str, data: Optional[bytes] = None, headers: Optional[Dict[str, str]] = None) -> Any:
        req = urllib.request.Request(url, method=method, data=data, headers=headers or {})
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw = resp.read().decode("utf-8")
                if not raw:
                    return {}
                return json.loads(raw)
        except urllib.error.HTTPError as e:
            self._handle_http_error(e)
        except urllib.error.URLError as e:
            raise DashboardError(f"Connection error: {e.reason}")
        return {}  # unreachable

    def _handle_http_error(self, e: urllib.error.HTTPError) -> None:
        body_raw = ""
        try:
            body_raw = e.read().decode("utf-8")
        except Exception:
            pass

        body = None
        try:
            body = json.loads(body_raw)
        except (json.JSONDecodeError, ValueError):
            body = body_raw or None

        msg = ""
        if isinstance(body, dict):
            msg = body.get("error", body_raw)
        else:
            msg = body_raw or f"HTTP {e.code}"

        if e.code == 400:
            raise ValidationError(msg, e.code, body)
        elif e.code == 403:
            raise AuthError(msg, e.code, body)
        elif e.code == 404:
            raise NotFoundError(msg, e.code, body)
        elif e.code == 429:
            retry_after = None
            ra = e.headers.get("Retry-After")
            if ra:
                try:
                    retry_after = float(ra)
                except ValueError:
                    pass
            raise RateLimitError(msg, e.code, body, retry_after=retry_after)
        elif e.code >= 500:
            raise ServerError(msg, e.code, body)
        else:
            raise DashboardError(msg, e.code, body)
