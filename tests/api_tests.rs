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
        ])
        .mount("/", rocket::routes![
            private_dashboard::routes::llms_txt,
            private_dashboard::routes::openapi_spec,
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
        ])
        .mount("/", rocket::routes![
            private_dashboard::routes::llms_txt,
            private_dashboard::routes::openapi_spec,
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
    assert_eq!(key_label("repos_count"), "Repos");
    assert_eq!(key_label("tests_total"), "Total Tests");
    assert_eq!(key_label("siblings_count"), "Sibling Agents");
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
    assert!(body.contains("Private Dashboard"));
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
    assert_eq!(body["info"]["title"], "Private Dashboard API");
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
        ("moltbook_interesting", "Moltbook Interesting"),
        ("moltbook_spam", "Moltbook Spam"),
        ("outreach_sent", "Outreach Sent"),
        ("outreach_received", "Outreach Received"),
        ("repos_count", "Repos"),
        ("tests_total", "Total Tests"),
        ("deploys_count", "Deploys"),
        ("commits_total", "Total Commits"),
        ("twitter_headlines", "Twitter Headlines"),
        ("siblings_count", "Sibling Agents"),
        ("siblings_active", "Siblings Active"),
        ("moltbook_health", "Moltbook Health"),
        ("moltbook_my_posts", "Moltbook Posts"),
        ("twitter_accounts", "Twitter Accounts"),
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
    let (client, key, db) = test_client_with_db();

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
