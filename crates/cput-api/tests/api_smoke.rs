//! In-process HTTP smoke tests for the dashboard API.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use cput_api::{router, ApiConfig, ApiState};
use cput_layer0_compute::TelemetrySample;
use cput_queue::{NodeRegistration, PipelineQueue, TelemetryPoll};
use http_body_util::BodyExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tower::ServiceExt;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(name: &str) -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "cput-api-test-{name}-{n}-{}",
        std::process::id()
    ))
}

fn seed_queue(queue_path: &std::path::Path) {
    {
        let queue = PipelineQueue::open(queue_path).unwrap();
        queue
            .put_node_registration(&NodeRegistration {
                node_key: "gpu-h100-0".into(),
                hardware_class: "gpu-h100".into(),
                rated_gflops_per_sec: 1_000_000,
                jurisdiction: "US".into(),
                enclave_measurement: [1u8; 32],
                workload_hash: [2u8; 32],
                verified_hash: [3u8; 32],
            })
            .unwrap();
        queue
            .push_telemetry(&TelemetryPoll {
                node_key: "gpu-h100-0".into(),
                epoch: 1007,
                tick: 0,
                sample_interval_secs: 30,
                sample: TelemetrySample {
                    flops_per_sec: 900_000,
                    vram_util_bps: 8400,
                    thermal_c: 72,
                    power_w: 350,
                },
            })
            .unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(50));
}

fn test_state() -> Arc<ApiState> {
    let queue_path = temp_dir("queue");
    let relayer_path = temp_dir("relayer");
    let _ = std::fs::remove_dir_all(&queue_path);
    let _ = std::fs::remove_dir_all(&relayer_path);

    seed_queue(&queue_path);

    let cfg = ApiConfig {
        queue_path,
        relayer_path,
        sui_config: None,
        listen_addr: "127.0.0.1:0".into(),
    };
    ApiState::open(&cfg).unwrap()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn health_endpoint_ok() {
    let app = router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn dashboard_returns_live_pipeline_data() {
    let app = router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["source"], "live");
    assert!(json["pipeline"]["telemetry"].as_u64().unwrap() >= 1);
    assert!(!json["tasks"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn mint_endpoint_accepts_valid_amount() {
    let state = test_state();
    let app = router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mint")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"amount":500}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["status"], "accepted");
    assert_eq!(json["amount"], 500);
}
