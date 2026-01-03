mod brain;
mod train;

use crate::brain::model::SemanticBrain;
use crate::train::wiki::WikipediaTrainer;
use axum::{
    routing::{get, post},
    extract::{State, Query},
    Json, Router,
};
use std::sync::Arc;
use parking_lot::RwLock;
use serde_json::json;
use serde::Deserialize;

#[derive(Deserialize)]
struct StartParams {
    max_articles: Option<usize>,
}

#[derive(Deserialize)]
struct SimilarParams {
    word: String,
    n: Option<usize>,
}

#[derive(Deserialize)]
struct AnalogyParams {
    a: String,
    b: String,
    c: String,
}

#[derive(Deserialize)]
struct AttentionParams {
    target: String,
    context: Vec<String>,
}

#[derive(Deserialize)]
struct RelationshipParams {
    a1: String,
    b1: String,
    a2: String,
    b2: String,
}

struct AppState {
    trainer: WikipediaTrainer,
    brain: Arc<RwLock<SemanticBrain>>,
}

#[tokio::main]
async fn main() {
    println!("============================================");
    println!("   SEMANTIC BRAIN SERVER - WIKI TRAINER     ");
    println!("============================================");

    let brain_data = if let Ok(bytes) = std::fs::read("data/model.bin") {
        println!("> Loading existing model from data/model.bin...");
        SemanticBrain::from_bytes(&bytes).unwrap_or_else(|_| SemanticBrain::new())
    } else if let Ok(bytes) = std::fs::read("trained_brain.bin") {
        println!("> Loading existing model from trained_brain.bin...");
        SemanticBrain::from_bytes(&bytes).unwrap_or_else(|_| SemanticBrain::new())
    } else {
        println!("> Starting with a fresh brain...");
        SemanticBrain::new()
    };

    let brain = Arc::new(RwLock::new(brain_data));
    let trainer = WikipediaTrainer::new(brain.clone());
    let app_state = Arc::new(AppState { trainer, brain: brain.clone() });

    let app = Router::new()
        .route("/train/wiki/start", post(start_training))
        .route("/train/wiki/stop", post(stop_training))
        .route("/train/wiki/status", get(get_status))
        .route("/train/wiki/sanitize", post(sanitize_model))
        .route("/predict/similar", get(predict_similar))
        .route("/predict/vector", get(get_vector))
        .route("/predict/analogy", get(predict_analogy))
        .route("/predict/attention", post(predict_attention))
        .route("/predict/relationship", get(predict_relationship))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("> Server listening on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn start_training(
    State(state): State<Arc<AppState>>,
    Query(_params): Query<StartParams>,
) -> Json<serde_json::Value> {
    state.trainer.start();
    Json(json!({ "started": true }))
}

async fn stop_training(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    state.trainer.stop();
    Json(json!({ "stopped": true }))
}

async fn get_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let s = state.trainer.state.read();
    let b = state.brain.read();
    
    Json(json!({
        "running": s.running,
        "articles_processed": s.articles_processed,
        "tokens_processed": s.tokens_processed,
        "last_title": s.last_title,
        "error": s.error,
        "vocab_size": b.vocabulary.len(),
        "embeddings_len": b.embeddings.len(),
    }))
}

async fn sanitize_model(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let mut b = state.brain.write();
    println!("> Sanitizing model (healing NaNs and centering vectors)...");
    b.balance_vectors();
    Json(json!({ "sanitized": true, "vocab_size": b.vocabulary.len() }))
}

async fn predict_similar(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SimilarParams>,
) -> Json<serde_json::Value> {
    let b = state.brain.read();
    let word_upper = params.word.to_uppercase();
    let found = b.vocabulary.contains_key(&word_upper);
    let results = b.find_most_similar(&params.word, params.n.unwrap_or(10));
    Json(json!({ 
        "word": params.word, 
        "found": found,
        "vocab_size": b.vocabulary.len(),
        "similar": results 
    }))
}

async fn get_vector(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SimilarParams>,
) -> Json<serde_json::Value> {
    let b = state.brain.read();
    let vec = b.get_embedding(&params.word);
    Json(json!({ 
        "word": params.word, 
        "vector": vec.map(|v| &v.data[..5])
    }))
}

async fn predict_analogy(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AnalogyParams>,
) -> Json<serde_json::Value> {
    let b = state.brain.read();
    let results = b.calculate_analogy(&params.a, &params.b, &params.c, 10);
    Json(json!({ 
        "analogy": format!("{} is to {} as {} is to ...", params.a, params.b, params.c),
        "results": results 
    }))
}

async fn predict_attention(
    State(state): State<Arc<AppState>>,
    Json(params): Json<AttentionParams>,
) -> Json<serde_json::Value> {
    let b = state.brain.read();
    let results = b.calculate_attention(&params.target, &params.context);
    Json(json!({ "target": params.target, "attention": results }))
}

async fn predict_relationship(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RelationshipParams>,
) -> Json<serde_json::Value> {
    let b = state.brain.read();
    let score = b.compare_relationships(&params.a1, &params.b1, &params.a2, &params.b2);
    Json(json!({ 
        "pair1": format!("{} -> {}", params.a1, params.b1),
        "pair2": format!("{} -> {}", params.a2, params.b2),
        "similarity_score": score 
    }))
}
