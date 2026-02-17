use rocket::http::{ContentType, Header, Status};
use rocket::local::blocking::Client;
use std::sync::Arc;

fn test_client_with_db() -> (Client, String, Arc<private_dashboard::db::Db>) {
    let db = Arc::new(private_dashboard::db::Db::new(":memory:").unwrap());
    let key = format!("dash_test_{}", uuid::Uuid::new_v4().simple());
    db.set_manage_key(&key);

    let cors = rocket_cors::CorsOptions::default()
        .allowed_origins(rocket_cors::AllowedOrigins::all())
        .to_cors()
        .unwrap();

    let rocket = rocket::build()
        .attach(cors)
        .manage(db.clone())
        .mount("/api/v1", rocket::routes![
            private_dashboard::routes::health,
            private_dashboard::routes::submit_stats,
            private_dashboard::routes::get_stats,
            private_dashboard::routes::get_stat_history,
            private_dashboard::routes::prune_stats,
            private_dashboard::routes::delete_stat,
            private_dashboard::routes::get_alerts,
        ])
        .mount("/", rocket::routes![
            private_dashboard::routes::llms_txt,
            private_dashboard::routes::openapi_spec,
            private_dashboard::routes::skills_index,
            private_dashboard::routes::skills_skill_md,
        ]);

    (Client::tracked(rocket).unwrap(), key, db)
}

fn test_client() -> (Client, String) {
    let db = Arc::new(private_dashboard::db::Db::new(":memory:").unwrap());
    let key = format!("dash_test_{}", uuid::Uuid::new_v4().simple());
    db.set_manage_key(&key);

    let cors = rocket_cors::CorsOptions::default()
        .allowed_origins(rocket_cors::AllowedOrigins::all())
        .to_cors()
        .unwrap();

    let rocket = rocket::build()
        .attach(cors)
        .manage(db)
        .mount("/api/v1", rocket::routes![
            private_dashboard::routes::health,
            private_dashboard::routes::submit_stats,
            private_dashboard::routes::get_stats,
            private_dashboard::routes::get_stat_history,
            private_dashboard::routes::prune_stats,
            private_dashboard::routes::delete_stat,
            private_dashboard::routes::get_alerts,
        ])
        .mount("/", rocket::routes![
            private_dashboard::routes::llms_txt,
            private_dashboard::routes::openapi_spec,
            private_dashboard::routes::skills_index,
            private_dashboard::routes::skills_skill_md,
        ]);

    (Client::tracked(rocket).unwrap(), key)
}

#[test]
fn test_health() {
    let (client, _) = test_client();
    let response = client.get("/api/v1/health").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["stats_count"], 0);
    assert_eq!(body["keys_count"], 0);
}

#[test]
fn test_submit_stats_no_auth() {
    let (client, _) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .body(r#"[{"key":"test","value":42}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Unauthorized);
}

#[test]
fn test_submit_stats_wrong_key() {
    let (client, _) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer wrong_key"))
        .body(r#"[{"key":"test","value":42}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Forbidden);
}

#[test]
fn test_submit_and_get_stats() {
    let (client, key) = test_client();

    // Submit stats
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"agents_discovered","value":645},{"key":"repos_count","value":7}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 2);

    // Get all stats
    let response = client.get("/api/v1/stats").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    let stats = body["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 2);

    // Check first stat
    let agents = stats.iter().find(|s| s["key"] == "agents_discovered").unwrap();
    assert_eq!(agents["current"], 645.0);
    assert_eq!(agents["label"], "Agents Discovered");
}

#[test]
fn test_submit_empty_array() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body("[]")
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
}

#[test]
fn test_submit_with_metadata() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"test_metric","value":100,"metadata":{"source":"manual"}}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 1);
}

#[test]
fn test_get_stat_history() {
    let (client, key) = test_client();

    // Submit a stat
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"test_key","value":10}]"#)
        .dispatch();

    // Get history
    let response = client.get("/api/v1/stats/test_key?period=24h").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["key"], "test_key");
    let points = body["points"].as_array().unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0]["value"], 10.0);
}

#[test]
fn test_get_stat_history_invalid_period() {
    let (client, _) = test_client();
    let response = client.get("/api/v1/stats/test_key?period=invalid").dispatch();
    assert_eq!(response.status(), Status::BadRequest);
}

#[test]
fn test_health_after_data() {
    let (client, key) = test_client();

    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"a","value":1},{"key":"b","value":2}]"#)
        .dispatch();

    let response = client.get("/api/v1/health").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["stats_count"], 2);
    assert_eq!(body["keys_count"], 2);
}

#[test]
fn test_submit_skips_invalid_keys() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"","value":1},{"key":"valid","value":2}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 1);
}

#[test]
fn test_multiple_submits_same_key() {
    let (client, key) = test_client();

    // Submit first value
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"counter","value":10}]"#)
        .dispatch();

    // Submit second value
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"counter","value":20}]"#)
        .dispatch();

    // Latest should be 20
    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stats = body["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0]["current"], 20.0);

    // History should have both points
    let response = client.get("/api/v1/stats/counter?period=24h").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let points = body["points"].as_array().unwrap();
    assert_eq!(points.len(), 2);
}

#[test]
fn test_get_stats_empty() {
    let (client, _) = test_client();
    let response = client.get("/api/v1/stats").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["stats"].as_array().unwrap().len(), 0);
}

#[test]
fn test_submit_too_many_stats() {
    let (client, key) = test_client();
    // Build a batch of 101 stats (over the limit of 100)
    let stats: Vec<serde_json::Value> = (0..101)
        .map(|i| serde_json::json!({"key": format!("metric_{}", i), "value": i}))
        .collect();
    let body = serde_json::to_string(&stats).unwrap();

    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(body)
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    let body: serde_json::Value = response.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("max 100"));
}

#[test]
fn test_submit_exactly_100_stats() {
    let (client, key) = test_client();
    let stats: Vec<serde_json::Value> = (0..100)
        .map(|i| serde_json::json!({"key": format!("m_{}", i), "value": i}))
        .collect();
    let body = serde_json::to_string(&stats).unwrap();

    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(body)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 100);
}

#[test]
fn test_submit_skips_long_keys() {
    let (client, key) = test_client();
    let long_key = "a".repeat(101); // over 100 char limit
    let body = format!(
        r#"[{{"key":"{}","value":1}},{{"key":"valid_key","value":2}}]"#,
        long_key
    );

    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(body)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 1); // only valid_key accepted
}

#[test]
fn test_get_stat_history_default_period() {
    let (client, key) = test_client();

    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"default_period_test","value":55}]"#)
        .dispatch();

    // No period param — should default to 24h
    let response = client.get("/api/v1/stats/default_period_test").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["key"], "default_period_test");
    assert_eq!(body["points"].as_array().unwrap().len(), 1);
}

#[test]
fn test_get_stat_history_nonexistent_key() {
    let (client, _) = test_client();
    let response = client.get("/api/v1/stats/nonexistent_key?period=7d").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["key"], "nonexistent_key");
    assert_eq!(body["points"].as_array().unwrap().len(), 0);
}

#[test]
fn test_sparkline_populated() {
    let (client, key) = test_client();

    // Submit a stat
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"spark_test","value":10}]"#)
        .dispatch();

    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "spark_test").unwrap();
    // Sparkline should be an array (may have 1 element since we only submitted once)
    assert!(stat["sparkline_24h"].is_array());
    let sparkline = stat["sparkline_24h"].as_array().unwrap();
    assert!(!sparkline.is_empty());
    assert_eq!(sparkline[0], 10.0);
}

#[test]
fn test_key_labels() {
    use private_dashboard::models::key_label;
    assert_eq!(key_label("agents_discovered"), "Agents Discovered");
    assert_eq!(key_label("repos_count"), "Repositories");
    assert_eq!(key_label("tests_total"), "Total Tests");
    assert_eq!(key_label("siblings_count"), "Sibling Agents");
    // Kanban metrics
    assert_eq!(key_label("kanban_backlog"), "Backlog");
    assert_eq!(key_label("kanban_in_progress"), "In Progress");
    assert_eq!(key_label("kanban_review"), "In Review");
    assert_eq!(key_label("kanban_active"), "Active Tasks");
    // Unknown key gets underscores replaced with spaces
    assert_eq!(key_label("custom_metric_name"), "custom metric name");
    assert_eq!(key_label("singleword"), "singleword");
}

#[test]
fn test_stat_trends_structure() {
    let (client, key) = test_client();

    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"metric","value":100}]"#)
        .dispatch();

    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stat = &body["stats"][0];

    // Trends should have all time windows
    assert!(stat["trends"]["24h"].is_object());
    assert!(stat["trends"]["7d"].is_object());
    assert!(stat["trends"]["30d"].is_object());
    assert!(stat["trends"]["90d"].is_object());

    // Each trend should have end = current
    assert_eq!(stat["trends"]["24h"]["end"], 100.0);
}

#[test]
fn test_llms_txt() {
    let (client, _) = test_client();
    let response = client.get("/llms.txt").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().unwrap();
    assert!(body.contains("The Pack"));
    assert!(body.contains("POST /api/v1/stats"));
    assert!(body.contains("GET /api/v1/stats"));
    assert!(body.contains("manage_key"));
}

#[test]
fn test_openapi_spec() {
    let (client, _) = test_client();
    let response = client.get("/openapi.json").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["openapi"], "3.0.3");
    assert_eq!(body["info"]["title"], "The Pack API");
    assert!(body["paths"]["/stats"].is_object());
    assert!(body["paths"]["/health"].is_object());
    assert!(body["paths"]["/stats/{key}"].is_object());
}

// ── Edge Case Tests ──

#[test]
fn test_submit_negative_values() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"temperature","value":-15.5}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 1);

    // Verify it reads back correctly
    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stat = &body["stats"][0];
    assert_eq!(stat["current"], -15.5);
}

#[test]
fn test_submit_zero_value() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"zero_metric","value":0}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "zero_metric").unwrap();
    assert_eq!(stat["current"], 0.0);
}

#[test]
fn test_submit_very_large_value() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"big_number","value":999999999999.99}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "big_number").unwrap();
    assert!(stat["current"].as_f64().unwrap() > 999_999_999_999.0);
}

#[test]
fn test_submit_fractional_value() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"fraction","value":0.001}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "fraction").unwrap();
    assert!((stat["current"].as_f64().unwrap() - 0.001).abs() < f64::EPSILON);
}

#[test]
fn test_submit_special_chars_in_key() {
    let (client, key) = test_client();
    // Underscores, hyphens, dots should work
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"my-metric.v2_count","value":42}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 1);

    // Verify via history endpoint (key is URL path param)
    let response = client.get("/api/v1/stats/my-metric.v2_count?period=24h").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["key"], "my-metric.v2_count");
    assert_eq!(body["points"].as_array().unwrap().len(), 1);
}

#[test]
fn test_submit_invalid_json_body() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body("this is not json")
        .dispatch();
    // Rocket returns 422 for malformed JSON
    assert!(response.status() == Status::UnprocessableEntity || response.status() == Status::BadRequest);
}

#[test]
fn test_submit_wrong_content_type() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::Plain)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"test","value":1}]"#)
        .dispatch();
    // Rocket returns 404 when format doesn't match (no route matches)
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn test_submit_object_instead_of_array() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"key":"test","value":1}"#)
        .dispatch();
    // Should reject — expects an array
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn test_submit_missing_value_field() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"test"}]"#)
        .dispatch();
    // Missing required field — should be rejected by serde
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn test_submit_missing_key_field() {
    let (client, key) = test_client();
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"value":42}]"#)
        .dispatch();
    // Missing required field
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn test_submit_large_metadata() {
    let (client, key) = test_client();
    // Large but valid metadata
    let big_meta: serde_json::Value = serde_json::json!({
        "description": "a".repeat(1000),
        "tags": (0..50).map(|i| format!("tag_{}", i)).collect::<Vec<_>>(),
        "nested": {"deep": {"deeper": "value"}}
    });
    let body = serde_json::json!([{"key": "meta_test", "value": 1, "metadata": big_meta}]);

    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(body.to_string())
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 1);
}

#[test]
fn test_get_history_all_periods() {
    let (client, key) = test_client();

    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"period_test","value":99}]"#)
        .dispatch();

    // Test all valid periods
    for period in &["24h", "7d", "30d", "90d"] {
        let url = format!("/api/v1/stats/period_test?period={}", period);
        let response = client.get(&url).dispatch();
        assert_eq!(response.status(), Status::Ok, "Failed for period {}", period);
        let body: serde_json::Value = response.into_json().unwrap();
        assert_eq!(body["key"], "period_test");
        assert!(body["points"].is_array());
    }
}

#[test]
fn test_submit_all_items_invalid() {
    let (client, key) = test_client();
    // All keys are empty — all should be skipped
    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"","value":1},{"key":"","value":2}]"#)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 0);
}

#[test]
fn test_rapid_sequential_writes() {
    let (client, key) = test_client();

    // Simulate rapid writes (like a collector posting frequently)
    for i in 0..10 {
        let body = format!(r#"[{{"key":"rapid","value":{}}}]"#, i);
        let response = client
            .post("/api/v1/stats")
            .header(ContentType::JSON)
            .header(Header::new("Authorization", format!("Bearer {}", key)))
            .body(body)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    // Latest should be 9
    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "rapid").unwrap();
    assert_eq!(stat["current"], 9.0);

    // History should have all 10 points
    let response = client.get("/api/v1/stats/rapid?period=24h").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["points"].as_array().unwrap().len(), 10);
}

#[test]
fn test_many_different_keys() {
    let (client, key) = test_client();

    // Submit 50 different metrics in one batch
    let stats: Vec<serde_json::Value> = (0..50)
        .map(|i| serde_json::json!({"key": format!("metric_{:03}", i), "value": i as f64}))
        .collect();

    let response = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(serde_json::to_string(&stats).unwrap())
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["accepted"], 50);

    // All 50 should show up in GET /stats
    let response = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["stats"].as_array().unwrap().len(), 50);
}

#[test]
fn test_key_label_all_known_keys() {
    use private_dashboard::models::key_label;

    // All known keys should have proper labels (not just underscore replacement)
    let known = vec![
        ("agents_discovered", "Agents Discovered"),
        ("moltbook_interesting", "Interesting Posts"),
        ("moltbook_spam", "Spam Detected"),
        ("outreach_sent", "Outreach Sent"),
        ("outreach_received", "Inbound Messages"),
        ("repos_count", "Repositories"),
        ("tests_total", "Total Tests"),
        ("deploys_count", "Deployments"),
        ("commits_total", "Total Commits"),
        ("twitter_headlines", "Flagged Tweets"),
        ("siblings_count", "Sibling Agents"),
        ("siblings_active", "Active Siblings"),
        ("moltbook_health", "Platform Health"),
        ("moltbook_my_posts", "My Posts"),
        ("twitter_accounts", "Tracked Accounts"),
        ("cron_jobs_active", "Active Cron Jobs"),
    ];

    for (key, expected) in known {
        assert_eq!(key_label(key), expected, "Label mismatch for key '{}'", key);
    }
}

// ── Prune tests ──

#[test]
fn test_prune_no_auth() {
    let (client, _key) = test_client();
    let resp = client.post("/api/v1/stats/prune").dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[test]
fn test_prune_wrong_key() {
    let (client, _key) = test_client();
    let resp = client
        .post("/api/v1/stats/prune")
        .header(Header::new("Authorization", "Bearer wrong_key"))
        .dispatch();
    assert_eq!(resp.status(), Status::Forbidden);
}

#[test]
fn test_prune_empty_db() {
    let (client, key) = test_client();
    let resp = client
        .post("/api/v1/stats/prune")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["deleted"], 0);
    assert_eq!(body["remaining"], 0);
    assert_eq!(body["retention_days"], 90);
}

#[test]
fn test_prune_keeps_recent_data() {
    let (client, key, _db) = test_client_with_db();

    // Submit some fresh stats via the API
    let stats = serde_json::json!([
        {"key": "test_metric", "value": 42.0},
        {"key": "another_metric", "value": 100.0}
    ]);
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(stats.to_string())
        .dispatch();

    // Prune — nothing should be deleted since data is fresh
    let resp = client
        .post("/api/v1/stats/prune")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["deleted"], 0);
    assert_eq!(body["remaining"], 2);
}

#[test]
fn test_prune_deletes_old_data() {
    let (client, key, db) = test_client_with_db();

    // Insert stats with an old timestamp (100 days ago) directly via DB
    let old_time = (chrono::Utc::now() - chrono::Duration::days(100)).to_rfc3339();
    db.insert_stat("old_metric", 1.0, &old_time, None);
    db.insert_stat("old_metric", 2.0, &old_time, None);
    db.insert_stat("old_metric_2", 50.0, &old_time, None);

    // Insert a fresh stat via the API
    let stats = serde_json::json!([{"key": "fresh_metric", "value": 99.0}]);
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(stats.to_string())
        .dispatch();

    // Verify we have 4 total
    assert_eq!(db.get_stat_count(), 4);

    // Prune — should delete 3 old ones, keep 1 fresh
    let resp = client
        .post("/api/v1/stats/prune")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["deleted"], 3);
    assert_eq!(body["remaining"], 1);
    assert_eq!(body["retention_days"], 90);
}

#[test]
fn test_prune_boundary_89_days_kept() {
    let (_client, _key, db) = test_client_with_db();

    // Insert stat exactly 89 days ago — should NOT be pruned (within 90-day window)
    let time_89d = (chrono::Utc::now() - chrono::Duration::days(89)).to_rfc3339();
    db.insert_stat("boundary_metric", 10.0, &time_89d, None);

    let deleted = db.prune_old_stats(90);
    assert_eq!(deleted, 0);
    assert_eq!(db.get_stat_count(), 1);
}

#[test]
fn test_prune_boundary_91_days_deleted() {
    let (_client, _key, db) = test_client_with_db();

    // Insert stat 91 days ago — should be pruned
    let time_91d = (chrono::Utc::now() - chrono::Duration::days(91)).to_rfc3339();
    db.insert_stat("old_boundary", 10.0, &time_91d, None);

    let deleted = db.prune_old_stats(90);
    assert_eq!(deleted, 1);
    assert_eq!(db.get_stat_count(), 0);
}

#[test]
fn test_health_includes_retention_info() {
    let (client, key, _db) = test_client_with_db();

    // Insert a stat so we have an oldest timestamp
    let stats = serde_json::json!([{"key": "test", "value": 1.0}]);
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(stats.to_string())
        .dispatch();

    let resp = client.get("/api/v1/health").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["retention_days"], 90);
    assert!(body["oldest_stat"].is_string());
}

#[test]
fn test_health_oldest_stat_null_when_empty() {
    let (client, _key) = test_client();
    let resp = client.get("/api/v1/health").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert!(body["oldest_stat"].is_null());
}

// ── Custom Date Range Tests ──

#[test]
fn test_stat_history_custom_date_range_iso8601() {
    let (client, _key, db) = test_client_with_db();

    // Insert stats at specific times
    db.insert_stat("cpu", 10.0, "2026-02-01T00:00:00Z", None);
    db.insert_stat("cpu", 20.0, "2026-02-05T00:00:00Z", None);
    db.insert_stat("cpu", 30.0, "2026-02-10T00:00:00Z", None);
    db.insert_stat("cpu", 40.0, "2026-02-15T00:00:00Z", None);

    // Query range that includes middle two
    let resp = client.get("/api/v1/stats/cpu?start=2026-02-03T00:00:00Z&end=2026-02-12T00:00:00Z").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["key"], "cpu");
    let points = body["points"].as_array().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0]["value"], 20.0);
    assert_eq!(points[1]["value"], 30.0);
}

#[test]
fn test_stat_history_custom_date_range_yyyy_mm_dd() {
    let (client, _key, db) = test_client_with_db();

    db.insert_stat("mem", 100.0, "2026-02-01T12:00:00Z", None);
    db.insert_stat("mem", 200.0, "2026-02-10T12:00:00Z", None);
    db.insert_stat("mem", 300.0, "2026-02-20T12:00:00Z", None);

    // Use YYYY-MM-DD format
    let resp = client.get("/api/v1/stats/mem?start=2026-02-01&end=2026-02-15").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let points = body["points"].as_array().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0]["value"], 100.0);
    assert_eq!(points[1]["value"], 200.0);
}

#[test]
fn test_stat_history_custom_range_start_after_end() {
    let (client, _key, _db) = test_client_with_db();

    let resp = client.get("/api/v1/stats/test?start=2026-02-20T00:00:00Z&end=2026-02-01T00:00:00Z").dispatch();
    assert_eq!(resp.status(), Status::BadRequest);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("start must be before end"));
}

#[test]
fn test_stat_history_custom_range_invalid_date() {
    let (client, _key, _db) = test_client_with_db();

    let resp = client.get("/api/v1/stats/test?start=not-a-date&end=2026-02-01").dispatch();
    assert_eq!(resp.status(), Status::BadRequest);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid date format"));
}

#[test]
fn test_stat_history_custom_range_empty_result() {
    let (client, _key, db) = test_client_with_db();

    db.insert_stat("disk", 50.0, "2026-01-01T00:00:00Z", None);

    // Query range that doesn't include the data point
    let resp = client.get("/api/v1/stats/disk?start=2026-02-01T00:00:00Z&end=2026-02-28T00:00:00Z").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let points = body["points"].as_array().unwrap();
    assert_eq!(points.len(), 0);
}

#[test]
fn test_stat_history_period_still_works() {
    let (client, _key, db) = test_client_with_db();

    let now = chrono::Utc::now();
    let recent = (now - chrono::Duration::hours(1)).to_rfc3339();
    db.insert_stat("net", 42.0, &recent, None);

    // Standard period param still works
    let resp = client.get("/api/v1/stats/net?period=24h").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let points = body["points"].as_array().unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0]["value"], 42.0);
}

// ── DELETE /api/v1/stats/:key tests ──

#[test]
fn test_delete_stat_no_auth() {
    let (client, _key, _db) = test_client_with_db();
    let resp = client.delete("/api/v1/stats/some_key").dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[test]
fn test_delete_stat_wrong_key() {
    let (client, _key, _db) = test_client_with_db();
    let resp = client
        .delete("/api/v1/stats/some_key")
        .header(Header::new("Authorization", "Bearer wrong_key"))
        .dispatch();
    assert_eq!(resp.status(), Status::Forbidden);
}

#[test]
fn test_delete_stat_nonexistent_key() {
    let (client, key, _db) = test_client_with_db();
    let resp = client
        .delete("/api/v1/stats/no_such_metric")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(resp.status(), Status::NotFound);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["key"], "no_such_metric");
}

#[test]
fn test_delete_stat_success() {
    let (client, key, db) = test_client_with_db();

    // Insert some data
    let now = chrono::Utc::now().to_rfc3339();
    db.insert_stat("stale_metric", 100.0, &now, None);
    db.insert_stat("stale_metric", 200.0, &now, None);
    db.insert_stat("keep_metric", 50.0, &now, None);

    // Delete stale_metric
    let resp = client
        .delete("/api/v1/stats/stale_metric")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["key"], "stale_metric");
    assert_eq!(body["deleted"], 2);

    // Verify stale_metric is gone from stats listing
    let stats_resp = client.get("/api/v1/stats").dispatch();
    let stats_body: serde_json::Value = stats_resp.into_json().unwrap();
    let keys: Vec<&str> = stats_body["stats"].as_array().unwrap()
        .iter().map(|s| s["key"].as_str().unwrap()).collect();
    assert!(!keys.contains(&"stale_metric"));
    assert!(keys.contains(&"keep_metric"));
}

#[test]
fn test_delete_stat_removes_all_history() {
    let (client, key, db) = test_client_with_db();

    // Insert multiple data points across time
    db.insert_stat("to_delete", 10.0, "2026-01-01T00:00:00Z", None);
    db.insert_stat("to_delete", 20.0, "2026-01-15T00:00:00Z", None);
    db.insert_stat("to_delete", 30.0, "2026-02-01T00:00:00Z", None);

    // Verify we have history
    let resp = client.get("/api/v1/stats/to_delete?period=90d").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["points"].as_array().unwrap().len(), 3);

    // Delete
    let resp = client
        .delete("/api/v1/stats/to_delete")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_json::<serde_json::Value>().unwrap()["deleted"], 3);

    // Verify history is empty
    let resp = client.get("/api/v1/stats/to_delete?period=90d").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["points"].as_array().unwrap().len(), 0);
}

#[test]
fn test_delete_stat_health_reflects_change() {
    let (client, key, db) = test_client_with_db();

    let now = chrono::Utc::now().to_rfc3339();
    db.insert_stat("metric_a", 1.0, &now, None);
    db.insert_stat("metric_b", 2.0, &now, None);

    // Health before delete
    let resp = client.get("/api/v1/health").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["keys_count"], 2);
    assert_eq!(body["stats_count"], 2);

    // Delete metric_a
    client
        .delete("/api/v1/stats/metric_a")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();

    // Health after delete
    let resp = client.get("/api/v1/health").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["keys_count"], 1);
    assert_eq!(body["stats_count"], 1);
}

// ── Alert History Tests ──

#[test]
fn test_alerts_empty() {
    let (client, _key, _db) = test_client_with_db();
    let resp = client.get("/api/v1/alerts").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["alerts"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 0);
}

#[test]
fn test_alerts_triggered_on_significant_change() {
    let (client, key, db) = test_client_with_db();

    // Insert baseline 25 hours ago (outside 24h window)
    let baseline_time = (chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
    db.insert_stat("test_metric", 100.0, &baseline_time, None);

    // Submit a 30% increase (should trigger "hot" alert)
    let resp = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key": "test_metric", "value": 130.0}]"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // Check alerts
    let resp = client.get("/api/v1/alerts").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let alerts = body["alerts"].as_array().unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["key"], "test_metric");
    assert_eq!(alerts[0]["level"], "hot");
    assert_eq!(alerts[0]["value"], 130.0);
    assert!(alerts[0]["change_pct"].as_f64().unwrap() > 29.0); // ~30%
    assert!(alerts[0]["label"].as_str().is_some());
}

#[test]
fn test_alerts_alert_level_threshold() {
    let (client, key, db) = test_client_with_db();

    // Insert baseline 25 hours ago
    let baseline_time = (chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
    db.insert_stat("metric_a", 100.0, &baseline_time, None);

    // Submit a 15% increase (should trigger "alert" not "hot")
    let resp = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key": "metric_a", "value": 115.0}]"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get("/api/v1/alerts").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let alerts = body["alerts"].as_array().unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["level"], "alert");
}

#[test]
fn test_alerts_no_alert_for_small_change() {
    let (client, key, db) = test_client_with_db();

    // Insert baseline 25 hours ago
    let baseline_time = (chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
    db.insert_stat("stable_metric", 100.0, &baseline_time, None);

    // Submit a 5% increase (should NOT trigger alert)
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key": "stable_metric", "value": 105.0}]"#)
        .dispatch();

    let resp = client.get("/api/v1/alerts").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["alerts"].as_array().unwrap().len(), 0);
}

#[test]
fn test_alerts_negative_change() {
    let (client, key, db) = test_client_with_db();

    // Insert baseline 25 hours ago
    let baseline_time = (chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
    db.insert_stat("drop_metric", 100.0, &baseline_time, None);

    // Submit a -20% decrease (should trigger "alert")
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key": "drop_metric", "value": 80.0}]"#)
        .dispatch();

    let resp = client.get("/api/v1/alerts").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let alerts = body["alerts"].as_array().unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["level"], "alert");
    assert!(alerts[0]["change_pct"].as_f64().unwrap() < -19.0); // ~-20%
}

#[test]
fn test_alerts_filter_by_key() {
    let (client, _key, db) = test_client_with_db();

    let now = chrono::Utc::now().to_rfc3339();
    db.insert_alert("metric_a", "alert", 150.0, 15.0, &now);
    db.insert_alert("metric_b", "hot", 200.0, 30.0, &now);

    // Filter by key
    let resp = client.get("/api/v1/alerts?key=metric_a").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let alerts = body["alerts"].as_array().unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["key"], "metric_a");
}

#[test]
fn test_alerts_limit() {
    let (client, _key, db) = test_client_with_db();

    // Insert 5 alerts
    for i in 0..5 {
        let t = (chrono::Utc::now() - chrono::Duration::minutes(i * 10)).to_rfc3339();
        db.insert_alert(&format!("m{}", i), "alert", 100.0 + i as f64, 15.0, &t);
    }

    // Limit to 2
    let resp = client.get("/api/v1/alerts?limit=2").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["alerts"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 5); // total still shows all
}

#[test]
fn test_alerts_debounce() {
    let (client, key, db) = test_client_with_db();

    // Insert baseline 25 hours ago
    let baseline_time = (chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
    db.insert_stat("bounce_metric", 100.0, &baseline_time, None);

    // First submit: triggers alert
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key": "bounce_metric", "value": 130.0}]"#)
        .dispatch();

    // Second submit: should be debounced (within 6h)
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key": "bounce_metric", "value": 135.0}]"#)
        .dispatch();

    let resp = client.get("/api/v1/alerts").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    // Only 1 alert despite 2 significant submissions
    assert_eq!(body["alerts"].as_array().unwrap().len(), 1);
}

#[test]
fn test_alerts_ordered_newest_first() {
    let (client, _key, db) = test_client_with_db();

    db.insert_alert("old", "alert", 100.0, 15.0, "2026-02-01T00:00:00Z");
    db.insert_alert("new", "hot", 200.0, 30.0, "2026-02-14T00:00:00Z");

    let resp = client.get("/api/v1/alerts").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let alerts = body["alerts"].as_array().unwrap();
    assert_eq!(alerts[0]["key"], "new");
    assert_eq!(alerts[1]["key"], "old");
}

// ── New tests: coverage expansion ──

#[test]
fn test_stats_returned_alphabetically() {
    let (client, key) = test_client();

    // Submit stats with keys out of alphabetical order
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[
            {"key":"zebra_metric","value":1},
            {"key":"alpha_metric","value":2},
            {"key":"middle_metric","value":3}
        ]"#)
        .dispatch();

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stats = body["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 3);
    assert_eq!(stats[0]["key"], "alpha_metric");
    assert_eq!(stats[1]["key"], "middle_metric");
    assert_eq!(stats[2]["key"], "zebra_metric");
}

#[test]
fn test_stats_response_all_fields_present() {
    let (client, key) = test_client();

    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"agents_discovered","value":100}]"#)
        .dispatch();

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stat = &body["stats"][0];

    // Verify all required fields
    assert!(stat["key"].is_string());
    assert!(stat["label"].is_string());
    assert!(stat["current"].is_number());
    assert!(stat["last_updated"].is_string());
    assert!(stat["sparkline_24h"].is_array());

    // Verify trends sub-object has all 4 periods
    let trends = &stat["trends"];
    assert!(trends["24h"].is_object());
    assert!(trends["7d"].is_object());
    assert!(trends["30d"].is_object());
    assert!(trends["90d"].is_object());

    // Each trend has end field at minimum
    assert_eq!(trends["24h"]["end"], 100.0);
    assert_eq!(trends["7d"]["end"], 100.0);
}

#[test]
fn test_health_all_fields_present() {
    let (client, key, _db) = test_client_with_db();

    // Submit one stat so health has data
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"test","value":42}]"#)
        .dispatch();

    let resp = client.get("/api/v1/health").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();

    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    assert_eq!(body["stats_count"], 1);
    assert_eq!(body["keys_count"], 1);
    assert_eq!(body["retention_days"], 90);
    assert!(body["oldest_stat"].is_string()); // should be set now
}

#[test]
fn test_sparkline_downsampling() {
    let (client, _key, db) = test_client_with_db();

    // Insert 30 data points within last 24h to trigger downsampling (sparkline is 12 points)
    let now = chrono::Utc::now();
    for i in 0..30 {
        let t = (now - chrono::Duration::minutes(i * 40)).to_rfc3339();
        db.insert_stat("spark_test", i as f64, &t, None);
    }

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "spark_test").unwrap();
    let sparkline = stat["sparkline_24h"].as_array().unwrap();

    // Should be downsampled to exactly 12 points
    assert_eq!(sparkline.len(), 12);
}

#[test]
fn test_sparkline_few_points_no_downsample() {
    let (client, _key, db) = test_client_with_db();

    // Insert only 5 points — fewer than 12, so no downsampling
    let now = chrono::Utc::now();
    for i in 0..5 {
        let t = (now - chrono::Duration::hours(i * 4)).to_rfc3339();
        db.insert_stat("spark_few", (i + 1) as f64, &t, None);
    }

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "spark_few").unwrap();
    let sparkline = stat["sparkline_24h"].as_array().unwrap();

    // Should return all points (≤12)
    assert_eq!(sparkline.len(), 5);
}

#[test]
fn test_trend_with_zero_start_value() {
    let (client, _key, db) = test_client_with_db();

    // Insert a zero value 2 hours ago, then a non-zero value now
    let now = chrono::Utc::now();
    let two_hours_ago = (now - chrono::Duration::hours(2)).to_rfc3339();
    let now_str = now.to_rfc3339();

    db.insert_stat("zero_start", 0.0, &two_hours_ago, None);
    db.insert_stat("zero_start", 50.0, &now_str, None);

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "zero_start").unwrap();

    // 24h trend: start=0, end=50, change=50, pct=null (division by zero avoided)
    let trend_24h = &stat["trends"]["24h"];
    assert_eq!(trend_24h["start"], 0.0);
    assert_eq!(trend_24h["end"], 50.0);
    assert_eq!(trend_24h["change"], 50.0);
    assert!(trend_24h["pct"].is_null()); // Can't compute % change from 0
}

#[test]
fn test_trend_with_no_prior_data() {
    let (client, key) = test_client();

    // Submit a single stat just now — no historical data exists
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"brand_new","value":42}]"#)
        .dispatch();

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "brand_new").unwrap();

    // All trends should use earliest-in-window fallback (which is the single point itself)
    // So start=42, end=42, change=0, pct=0
    let trend = &stat["trends"]["24h"];
    assert_eq!(trend["end"], 42.0);
}

#[test]
fn test_key_label_unknown_key_fallback() {
    let (client, key) = test_client();

    // Submit a stat with an unrecognized key
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"my_custom_metric","value":99}]"#)
        .dispatch();

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "my_custom_metric").unwrap();

    // Unknown keys should have underscores replaced with spaces
    assert_eq!(stat["label"], "my custom metric");
}

#[test]
fn test_key_label_all_known_keys_have_labels() {
    let (client, key) = test_client();

    let known_keys = vec![
        "agents_discovered", "moltbook_interesting", "moltbook_spam",
        "outreach_sent", "outreach_received", "repos_count",
        "tests_total", "deploys_count", "commits_total",
        "twitter_headlines", "siblings_count", "siblings_active",
        "moltbook_health", "moltbook_my_posts", "twitter_accounts",
        "cron_jobs_active", "kanban_backlog", "kanban_up_next",
        "kanban_in_progress", "kanban_review", "kanban_done", "kanban_active",
    ];

    // Submit all known keys
    let stats_json: Vec<String> = known_keys.iter().enumerate()
        .map(|(i, k)| format!(r#"{{"key":"{}","value":{}}}"#, k, i))
        .collect();
    let body_str = format!("[{}]", stats_json.join(","));

    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(body_str)
        .dispatch();

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stats = body["stats"].as_array().unwrap();

    // Every known key should have a proper label (not just underscore-to-space fallback)
    for stat in stats {
        let key_str = stat["key"].as_str().unwrap();
        let label = stat["label"].as_str().unwrap();
        assert!(!label.is_empty(), "Empty label for key: {}", key_str);
        // Known keys should have proper capitalized labels
        assert!(label.chars().next().unwrap().is_uppercase(),
            "Label for '{}' should start with uppercase: '{}'", key_str, label);
    }
}

#[test]
fn test_seq_monotonically_increases() {
    let (_client, _key, db) = test_client_with_db();

    let now = chrono::Utc::now().to_rfc3339();

    let seq1 = db.insert_stat("seq_test", 1.0, &now, None);
    let seq2 = db.insert_stat("seq_test", 2.0, &now, None);
    let seq3 = db.insert_stat("other_key", 3.0, &now, None);

    assert!(seq2 > seq1, "seq2 ({}) should be > seq1 ({})", seq2, seq1);
    assert!(seq3 > seq2, "seq3 ({}) should be > seq2 ({})", seq3, seq2);
    assert_eq!(seq1, 1);
    assert_eq!(seq2, 2);
    assert_eq!(seq3, 3);
}

#[test]
fn test_custom_range_only_start_no_end() {
    let (client, key) = test_client();

    // Submit some data
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"range_test","value":10}]"#)
        .dispatch();

    // Only start, no end — should fall through to period-based logic (default 24h)
    let resp = client.get("/api/v1/stats/range_test?start=2026-01-01").dispatch();
    assert_eq!(resp.status(), Status::Ok);
}

#[test]
fn test_custom_range_only_end_no_start() {
    let (client, key) = test_client();

    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"range_test2","value":10}]"#)
        .dispatch();

    // Only end, no start — should fall through to period-based logic
    let resp = client.get("/api/v1/stats/range_test2?end=2026-12-31").dispatch();
    assert_eq!(resp.status(), Status::Ok);
}

#[test]
fn test_openapi_spec_valid_json() {
    let (client, _) = test_client();
    let resp = client.get("/openapi.json").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();

    // Basic OpenAPI structure validation
    assert_eq!(body["openapi"], "3.0.3");
    assert!(body["info"].is_object());
    assert!(body["info"]["title"].is_string());
    assert!(body["info"]["version"].is_string());
    assert!(body["paths"].is_object());

    // Check key endpoints are documented (paths are relative, without /api/v1 prefix)
    let paths = body["paths"].as_object().unwrap();
    assert!(paths.contains_key("/health"), "Missing /health path");
    assert!(paths.contains_key("/stats"), "Missing /stats path");
    assert!(paths.contains_key("/alerts"), "Missing /alerts path");
    assert!(paths.contains_key("/stats/prune"), "Missing /stats/prune path");
    let stats_key_path = "/stats/{key}";
    assert!(paths.contains_key(stats_key_path), "Missing /stats/{{key}} path");
}

#[test]
fn test_prune_does_not_affect_alerts() {
    let (client, key, db) = test_client_with_db();

    // Insert old alert and old stat
    db.insert_alert("old_metric", "alert", 100.0, 15.0, "2025-01-01T00:00:00Z");
    let old_time = "2025-01-01T00:00:00Z";
    db.insert_stat("old_metric", 100.0, old_time, None);

    // Prune stats
    let resp = client
        .post("/api/v1/stats/prune")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert!(body["deleted"].as_i64().unwrap() >= 1); // stat was pruned

    // Alert should still exist (alert_log is independent of stats)
    let alert_resp = client.get("/api/v1/alerts").dispatch();
    let alert_body: serde_json::Value = alert_resp.into_json().unwrap();
    assert_eq!(alert_body["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(alert_body["alerts"][0]["key"], "old_metric");
}

#[test]
fn test_alerts_label_matches_key_label() {
    let (client, _key, db) = test_client_with_db();

    let now = chrono::Utc::now().to_rfc3339();
    db.insert_alert("agents_discovered", "alert", 100.0, 15.0, &now);
    db.insert_alert("unknown_custom_key", "hot", 200.0, 30.0, &now);

    let resp = client.get("/api/v1/alerts").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let alerts = body["alerts"].as_array().unwrap();

    let agents_alert = alerts.iter().find(|a| a["key"] == "agents_discovered").unwrap();
    assert_eq!(agents_alert["label"], "Agents Discovered");

    let custom_alert = alerts.iter().find(|a| a["key"] == "unknown_custom_key").unwrap();
    assert_eq!(custom_alert["label"], "unknown custom key");
}

#[test]
fn test_alerts_limit_clamping() {
    let (client, _key, db) = test_client_with_db();

    // Insert 3 alerts
    for i in 0..3 {
        let t = (chrono::Utc::now() - chrono::Duration::minutes(i * 10)).to_rfc3339();
        db.insert_alert(&format!("m{}", i), "alert", 100.0, 15.0, &t);
    }

    // limit=0 should be clamped to 1
    let resp = client.get("/api/v1/alerts?limit=0").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["alerts"].as_array().unwrap().len(), 1);

    // limit=1000 should be clamped to 500 (but only 3 exist)
    let resp2 = client.get("/api/v1/alerts?limit=1000").dispatch();
    let body2: serde_json::Value = resp2.into_json().unwrap();
    assert_eq!(body2["alerts"].as_array().unwrap().len(), 3);
    assert_eq!(body2["total"], 3);
}

#[test]
fn test_submit_mixed_valid_invalid_in_batch() {
    let (client, key) = test_client();

    // Mix: valid key, empty key (skipped), too-long key (skipped)
    let long_key = "k".repeat(101);
    let body_str = format!(
        r#"[{{"key":"good_key","value":10}},{{"key":"","value":20}},{{"key":"{}","value":30}}]"#,
        long_key
    );

    let resp = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(body_str)
        .dispatch();

    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["accepted"], 1); // Only the valid key

    // Verify only the valid key was stored
    let stats_resp = client.get("/api/v1/stats").dispatch();
    let stats_body: serde_json::Value = stats_resp.into_json().unwrap();
    assert_eq!(stats_body["stats"].as_array().unwrap().len(), 1);
    assert_eq!(stats_body["stats"][0]["key"], "good_key");
}

#[test]
fn test_multiple_submits_latest_value_used() {
    let (client, _key, db) = test_client_with_db();

    // Insert values at different times
    let now = chrono::Utc::now();
    db.insert_stat("evolving", 10.0, &(now - chrono::Duration::hours(3)).to_rfc3339(), None);
    db.insert_stat("evolving", 20.0, &(now - chrono::Duration::hours(2)).to_rfc3339(), None);
    db.insert_stat("evolving", 30.0, &(now - chrono::Duration::hours(1)).to_rfc3339(), None);

    let resp = client.get("/api/v1/stats").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let stat = body["stats"].as_array().unwrap()
        .iter().find(|s| s["key"] == "evolving").unwrap();

    // Should show the latest value (highest seq)
    assert_eq!(stat["current"], 30.0);
}

#[test]
fn test_history_chronological_order() {
    let (client, _key, db) = test_client_with_db();

    let now = chrono::Utc::now();
    // Insert out of order
    db.insert_stat("chrono", 30.0, &(now - chrono::Duration::hours(1)).to_rfc3339(), None);
    db.insert_stat("chrono", 10.0, &(now - chrono::Duration::hours(3)).to_rfc3339(), None);
    db.insert_stat("chrono", 20.0, &(now - chrono::Duration::hours(2)).to_rfc3339(), None);

    let resp = client.get("/api/v1/stats/chrono?period=24h").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let points = body["points"].as_array().unwrap();

    assert_eq!(points.len(), 3);
    // Should be in chronological (ASC) order
    assert_eq!(points[0]["value"], 10.0); // oldest
    assert_eq!(points[1]["value"], 20.0);
    assert_eq!(points[2]["value"], 30.0); // newest
}

#[test]
fn test_history_nonexistent_key_returns_empty() {
    let (client, _) = test_client();
    let resp = client.get("/api/v1/stats/does_not_exist?period=7d").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["key"], "does_not_exist");
    assert_eq!(body["points"].as_array().unwrap().len(), 0);
}

#[test]
fn test_delete_stat_cleans_up_completely() {
    let (client, key, db) = test_client_with_db();

    // Insert multiple data points
    let now = chrono::Utc::now();
    for i in 0..5 {
        db.insert_stat("to_delete", (i * 10) as f64, &(now - chrono::Duration::hours(i)).to_rfc3339(), None);
    }

    // Verify they exist
    let resp = client.get("/api/v1/stats/to_delete?period=24h").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["points"].as_array().unwrap().len(), 5);

    // Delete
    let del_resp = client
        .delete("/api/v1/stats/to_delete")
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .dispatch();
    assert_eq!(del_resp.status(), Status::Ok);
    let del_body: serde_json::Value = del_resp.into_json().unwrap();
    assert_eq!(del_body["deleted"], 5);

    // Verify history is empty
    let after_resp = client.get("/api/v1/stats/to_delete?period=24h").dispatch();
    let after_body: serde_json::Value = after_resp.into_json().unwrap();
    assert_eq!(after_body["points"].as_array().unwrap().len(), 0);

    // Verify stats listing doesn't include it
    let stats_resp = client.get("/api/v1/stats").dispatch();
    let stats_body: serde_json::Value = stats_resp.into_json().unwrap();
    let has_key = stats_body["stats"].as_array().unwrap()
        .iter().any(|s| s["key"] == "to_delete");
    assert!(!has_key, "Deleted key should not appear in stats listing");
}

#[test]
fn test_submit_auth_only_bearer_supported() {
    let (client, key) = test_client();

    // X-API-Key header should NOT work (only Bearer supported)
    let resp = client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("X-API-Key", key.clone()))
        .body(r#"[{"key":"via_header","value":42}]"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);

    // ?key= query param should NOT work (only Bearer supported)
    let resp2 = client
        .post(format!("/api/v1/stats?key={}", key))
        .header(ContentType::JSON)
        .body(r#"[{"key":"via_query","value":42}]"#)
        .dispatch();
    assert_eq!(resp2.status(), Status::Unauthorized);
}

#[test]
fn test_health_keys_count_multiple() {
    let (client, key) = test_client();

    // Submit 3 different keys
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"a","value":1},{"key":"b","value":2},{"key":"c","value":3}]"#)
        .dispatch();

    // Submit duplicate of key "a"
    client
        .post("/api/v1/stats")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"[{"key":"a","value":10}]"#)
        .dispatch();

    let resp = client.get("/api/v1/health").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["keys_count"], 3); // 3 distinct keys
    assert_eq!(body["stats_count"], 4); // 4 total data points
}

#[test]
fn test_llms_txt_contains_endpoints() {
    let (client, _) = test_client();
    let resp = client.get("/llms.txt").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();

    // Verify key sections are documented
    assert!(body.contains("GET /api/v1/health"), "Missing health endpoint");
    assert!(body.contains("POST /api/v1/stats"), "Missing submit endpoint");
    assert!(body.contains("GET /api/v1/stats"), "Missing get stats endpoint");
    assert!(body.contains("DELETE /api/v1/stats"), "Missing delete endpoint");
    assert!(body.contains("GET /api/v1/alerts"), "Missing alerts endpoint");
    assert!(body.contains("POST /api/v1/stats/prune"), "Missing prune endpoint");
    assert!(body.contains("Bearer"), "Missing auth documentation");
    assert!(body.contains("/.well-known/skills/"), "Missing skills discovery documentation");
}

// ── Well-Known Skills Discovery ──

#[test]
fn test_skills_index_json() {
    let (client, _) = test_client();
    let resp = client.get("/.well-known/skills/index.json").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let skills = body["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "private-dashboard");
    assert!(skills[0]["description"].as_str().unwrap().len() > 10);
    let files = skills[0]["files"].as_array().unwrap();
    assert!(files.contains(&serde_json::json!("SKILL.md")));
}

#[test]
fn test_skills_skill_md() {
    let (client, _) = test_client();
    let resp = client.get("/.well-known/skills/private-dashboard/SKILL.md").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();

    // YAML frontmatter
    assert!(body.starts_with("---"), "Missing YAML frontmatter");
    assert!(body.contains("name: private-dashboard"), "Missing skill name");
    assert!(body.contains("description:"), "Missing skill description");

    // Key content sections
    assert!(body.contains("## Quick Start"), "Missing Quick Start section");
    assert!(body.contains("## Auth Model"), "Missing Auth Model section");
    assert!(body.contains("## Known Metric Keys"), "Missing metric keys section");
    assert!(body.contains("## Gotchas"), "Missing Gotchas section");
    assert!(body.contains("/api/v1/stats"), "Missing stats endpoint reference");
    assert!(body.contains("manage_key"), "Missing auth reference");
}
