use bevy::prelude::*;
use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use crate::brain::model::SemanticBrain;
use crate::brain::projection::Projector;
use crate::turso::TursoClient;

lazy_static! {
    static ref SEMANTIC_BRAIN: Arc<Mutex<SemanticBrain>> = Arc::new(Mutex::new(SemanticBrain::new()));
    static ref PROJECTOR: Arc<Mutex<Projector>> = Arc::new(Mutex::new(Projector::new()));
    static ref TURSO_CLIENT: Arc<Mutex<Option<TursoClient>>> = Arc::new(Mutex::new(None));
    static ref NODE_QUEUE: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
}

#[derive(Component)]
struct VectorNode {
    text: String,
    position: Vec3,
}

#[derive(Resource)]
struct AppState {
    target_pos: Option<Vec3>,
    transition_timer: f32,
    is_manual_control: bool,
    last_interaction: f32,
}

#[derive(Component)]
struct PanOrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
    pub target_radius: f32,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 300.0,
            target_radius: 300.0,
        }
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    let database_url = match option_env!("TURSO_DATABASE_URL") {
        Some(url) => url.to_string(),
        None => "".to_string(),
    };
    let auth_token = match option_env!("TURSO_AUTH_TOKEN") {
        Some(token) => token.to_string(),
        None => "".to_string(),
    };
    
    if !database_url.is_empty() {
        *TURSO_CLIENT.lock().unwrap() = Some(TursoClient::new(database_url, auth_token));
    }

    let mut brain = SEMANTIC_BRAIN.lock().unwrap();
    
    // EMBEDDED BRAIN: This will cause Cloudflare to fail but works perfectly locally
    let model_bytes = include_bytes!("../trained_brain.bin");
    if model_bytes.len() > 0 {
        match SemanticBrain::from_bytes(model_bytes) {
            Ok(loaded_brain) => {
                *brain = loaded_brain;
                web_sys::console::log_1(&"Embedded Semantic Brain Loaded!".into());
            }
            Err(e) => {
                web_sys::console::log_1(&format!("Failed to load embedded brain: {}. Using blank brain.", e).into());
            }
        }
    }
}

#[wasm_bindgen]
pub fn run_bevy_app() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::Windowed,
                canvas: Some("#bevy".to_string()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::Srgba(Srgba::hex("#050505").unwrap())))
        .insert_resource(AppState { 
            target_pos: None, 
            transition_timer: 0.0,
            is_manual_control: false,
            last_interaction: 0.0,
        })
        .add_systems(Startup, (setup_scene, setup_axis, spawn_load_task, fit_projector))
        .add_systems(Update, (
            process_node_queue, 
            camera_controller,
            sync_camera_commands,
            handle_picking
        ))
        .run();
}

fn fit_projector() {
    let brain = SEMANTIC_BRAIN.lock().unwrap();
    let mut projector = PROJECTOR.lock().unwrap();
    if !brain.embeddings.is_empty() {
        projector.fit(&brain.embeddings);
        web_sys::console::log_1(&"3D Projection Space calibrated to semantic brain!".into());
    }
}

// ... (rest of the file stays same)
