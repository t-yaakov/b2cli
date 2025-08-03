// tests/api_integration.rs
// Testes de integração da API

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use b2cli::{AppState, models::*};
use hyper::body::Bytes;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio_cron_scheduler::JobScheduler;
use tower::util::ServiceExt;

mod common;
use common::TestDatabase;

// Helper para criar app de teste
async fn create_test_app() -> Router {
    let test_db = TestDatabase::new().await;
    let scheduler = JobScheduler::new().await.expect("Failed to create scheduler");
    
    let app_state = AppState {
        db_pool: test_db.pool.clone(),
        scheduler: Arc::new(scheduler),
    };

    // Criar app usando as mesmas rotas do main
    Router::new()
        .route("/health", axum::routing::get(b2cli::routes::health::health_check))
        .route("/readiness", axum::routing::get(b2cli::routes::readiness::readiness_check))
        .route("/backups", 
            axum::routing::post(b2cli::routes::backups::create_backup)
                .get(b2cli::routes::backups::list_backups))
        .route("/backups/:id", 
            axum::routing::get(b2cli::routes::backups::get_backup)
                .put(b2cli::routes::backups::update_backup)
                .delete(b2cli::routes::backups::delete_backup))
        .route("/backups/:id/run", 
            axum::routing::post(b2cli::routes::backups::run_backup))
        .with_state(app_state)
}

async fn parse_response_body(body: Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = parse_response_body(response.into_body()).await;
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_readiness_endpoint() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = parse_response_body(response.into_body()).await;
    assert_eq!(body["status"], "ready");
    assert_eq!(body["dependencies"]["database"]["status"], "healthy");
}

#[tokio::test]
async fn test_create_backup_job() {
    let app = create_test_app().await;
    
    let new_job = json!({
        "name": "Test Backup",
        "mappings": {
            "/tmp/test": ["/tmp/backup"]
        }
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backups")
                .header("content-type", "application/json")
                .body(Body::from(new_job.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let body = parse_response_body(response.into_body()).await;
    assert_eq!(body["name"], "Test Backup");
    assert_eq!(body["status"], "PENDING");
    assert_eq!(body["is_active"], true);
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn test_list_backup_jobs() {
    let app = create_test_app().await;
    
    // Primeiro, criar um backup job
    let new_job = json!({
        "name": "List Test Backup",
        "mappings": {
            "/tmp/source": ["/tmp/dest"]
        }
    });
    
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backups")
                .header("content-type", "application/json")
                .body(Body::from(new_job.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(create_response.status(), StatusCode::CREATED);
    
    // Agora listar todos os backups
    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/backups")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(list_response.status(), StatusCode::OK);
    
    let body = parse_response_body(list_response.into_body()).await;
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["name"], "List Test Backup");
}

#[tokio::test]
async fn test_get_backup_job() {
    let app = create_test_app().await;
    
    // Criar backup job
    let new_job = json!({
        "name": "Get Test Backup",
        "mappings": {
            "/tmp/source": ["/tmp/dest1", "/tmp/dest2"]
        }
    });
    
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backups")
                .header("content-type", "application/json")
                .body(Body::from(new_job.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let create_body = parse_response_body(create_response.into_body()).await;
    let job_id = create_body["id"].as_str().unwrap();
    
    // Buscar o backup job criado
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/backups/{}", job_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(get_response.status(), StatusCode::OK);
    
    let body = parse_response_body(get_response.into_body()).await;
    assert_eq!(body["name"], "Get Test Backup");
    assert_eq!(body["id"], job_id);
    assert_eq!(body["mappings"]["/tmp/source"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_nonexistent_backup() {
    let app = create_test_app().await;
    let fake_id = "00000000-0000-0000-0000-000000000000";
    
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/backups/{}", fake_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_backup_job() {
    let app = create_test_app().await;
    
    // Criar backup job
    let new_job = json!({
        "name": "Delete Test Backup",
        "mappings": {
            "/tmp/source": ["/tmp/dest"]
        }
    });
    
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backups")
                .header("content-type", "application/json")
                .body(Body::from(new_job.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let create_body = parse_response_body(create_response.into_body()).await;
    let job_id = create_body["id"].as_str().unwrap();
    
    // Deletar o backup job (soft delete)
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/backups/{}", job_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
    
    // Verificar que não aparece mais na listagem
    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/backups")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    let list_body = parse_response_body(list_response.into_body()).await;
    assert_eq!(list_body.as_array().unwrap().len(), 0); // Não deve aparecer jobs deletados
}

#[tokio::test]
async fn test_invalid_backup_job_creation() {
    let app = create_test_app().await;
    
    let invalid_job = json!({
        "name": "", // Nome vazio deve falhar
        "mappings": {}
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backups")
                .header("content-type", "application/json")
                .body(Body::from(invalid_job.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    // Deve retornar erro de validação
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_malformed_json() {
    let app = create_test_app().await;
    
    let malformed_json = r#"{"name": "Test", "mappings": invalid_json}"#;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backups")
                .header("content-type", "application/json")
                .body(Body::from(malformed_json))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}