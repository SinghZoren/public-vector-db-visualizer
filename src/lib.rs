mod turso;
mod brain;

use bevy::prelude::*;
use bevy::window::WindowMode;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use wasm_bindgen::prelude::*;
use std::sync::Mutex;
use lazy_static::lazy_static;
use crate::turso::TursoClient;
use crate::brain::model::SemanticBrain;
use crate::brain::projection::Projector;

lazy_static! {
    static ref TURSO_CLIENT: Mutex<Option<TursoClient>> = Mutex::new(None);
    static ref NODE_QUEUE: Mutex<Vec<NodeData>> = Mutex::new(Vec::new());
    static ref CAMERA_COMMAND: Mutex<Option<CameraCmd>> = Mutex::new(None);
    static ref LAST_KNOWN_COUNT: Mutex<usize> = Mutex::new(0);
    static ref SEMANTIC_BRAIN: Mutex<SemanticBrain> = Mutex::new(SemanticBrain::new());
    static ref PROJECTOR: Mutex<Projector> = Mutex::new(Projector::new());
}

enum CameraCmd {
    Focus(Vec3),
    Reset,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = updateUI)]
    fn update_ui(data: JsValue);
    #[wasm_bindgen(js_name = updateRecentNodes)]
    fn update_recent_nodes(data: JsValue);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeData {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub is_new: bool,
    pub created_at: String,
}

#[derive(Component)]
struct Node {
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
    let database_url = env!("TURSO_DATABASE_URL").to_string();
    let auth_token = env!("TURSO_AUTH_TOKEN").to_string();
    *TURSO_CLIENT.lock().unwrap() = Some(TursoClient::new(database_url, auth_token));

    let mut brain = SEMANTIC_BRAIN.lock().unwrap();
    
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

fn get_semantic_pos(text: &str) -> Vec3 {
    let mut brain = SEMANTIC_BRAIN.lock().unwrap();
    let projector = PROJECTOR.lock().unwrap();
    
    brain.train_step(text, &[], &[], 0.0, 0); 
    
    if let Some(v) = brain.get_embedding(text) {
        let (x, y, z) = projector.project(v);
        Vec3::new(x, y, z) * 100.0
    } else {
        Vec3::ZERO
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

fn setup_scene(mut commands: Commands) {
    let translation = Vec3::new(0.0, 150.0, 300.0);
    let focus = Vec3::ZERO;
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(translation).looking_at(focus, Vec3::Y),
            tonemapping: Tonemapping::None,
            ..default()
        },
        PanOrbitCamera {
            focus,
            radius: translation.length(),
            target_radius: translation.length(),
        },
    ));

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 80000.0,
            shadows_enabled: true,
            range: 1000.0,
            ..default()
        },
        transform: Transform::from_xyz(100.0, 100.0, 100.0),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });
}

fn camera_controller(
    time: Res<Time>,
    mut app_state: ResMut<AppState>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform)>,
) {
    let (mut pan_orbit, mut transform) = query.single_mut();
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let now = time.elapsed_seconds();

    for event in mouse_motion_events.read() {
        if mouse_button_input.pressed(MouseButton::Left) {
            rotation_move += event.delta;
            app_state.is_manual_control = true;
            app_state.last_interaction = now;
        } else if mouse_button_input.pressed(MouseButton::Right) {
            let pan_scale = pan_orbit.radius * 0.0005;
            let rot = transform.rotation;
            let right = rot * Vec3::X;
            let up = rot * Vec3::Y;
            pan_orbit.focus += right * -event.delta.x * pan_scale + up * event.delta.y * pan_scale;
            app_state.is_manual_control = true;
            app_state.last_interaction = now;
        }
    }

    for event in mouse_wheel_events.read() {
        scroll += event.y;
        app_state.is_manual_control = true;
        app_state.last_interaction = now;
    }

    if app_state.is_manual_control && (now - app_state.last_interaction > 5.0) {
        app_state.is_manual_control = false;
        app_state.transition_timer = 0.0;
    }

    if scroll != 0.0 {
        let zoom_step = 0.1;
        if scroll > 0.0 {
            pan_orbit.target_radius *= 1.0 - zoom_step;
        } else {
            pan_orbit.target_radius *= 1.0 + zoom_step;
        }
        pan_orbit.target_radius = pan_orbit.target_radius.max(20.0).min(1200.0);
    }
    
    let radius_lerp = (time.delta_seconds() * 8.0).min(1.0);
    pan_orbit.radius += (pan_orbit.target_radius - pan_orbit.radius) * radius_lerp;

    if !app_state.is_manual_control {
        if let Some(target) = app_state.target_pos {
            app_state.transition_timer = (app_state.transition_timer + time.delta_seconds() * 0.6).min(1.0);
            let t = app_state.transition_timer;
            let smooth_t = t * t * (3.0 - 2.0 * t);

            let orbit_radius = 80.0;
            let angle = time.elapsed_seconds() * 0.4;
            let target_orbit_pos = target + Vec3::new(
                orbit_radius * angle.cos(),
                30.0,
                orbit_radius * angle.sin(),
            );

            transform.translation = transform.translation.lerp(target_orbit_pos, smooth_t);
            let look_rotation = transform.looking_at(target, Vec3::Y).rotation;
            transform.rotation = transform.rotation.slerp(look_rotation, smooth_t);
            
            pan_orbit.focus = target;
            pan_orbit.target_radius = orbit_radius;
        } else {
            let radius = 350.0;
            let angle = time.elapsed_seconds() * 0.1;
            let target_pos = Vec3::new(radius * angle.cos(), 150.0, radius * angle.sin());
            
            transform.translation = transform.translation.lerp(target_pos, 0.05);
            let look_rotation = transform.looking_at(Vec3::ZERO, Vec3::Y).rotation;
            transform.rotation = transform.rotation.slerp(look_rotation, 0.05);
            
            pan_orbit.focus = Vec3::ZERO;
            pan_orbit.target_radius = radius;
        }
    } else {
        if rotation_move.length_squared() > 0.0 {
            let window_scale = 0.005;
            let yaw = Quat::from_rotation_y(-rotation_move.x * window_scale);
            let pitch = Quat::from_rotation_x(-rotation_move.y * window_scale);
            transform.rotation = yaw * transform.rotation * pitch;
        }
        let rot_matrix = Mat3::from_quat(transform.rotation);
        transform.translation = pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
    }
}

fn handle_picking(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window>,
    node_query: Query<(&Node, &GlobalTransform)>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let window = window_query.single();
        if let Some(cursor_pos) = window.cursor_position() {
            let (camera, camera_transform) = camera_query.single();
            if let Some(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
                let mut closest_node = None;
                let mut min_dist = f32::MAX;

                for (node, transform) in node_query.iter() {
                    let node_pos = transform.translation();
                    let v = node_pos - ray.origin;
                    let t = v.dot(*ray.direction);
                    if t < 0.0 { continue; }
                    
                    let nearest_point = ray.origin + (*ray.direction * t);
                    let dist = (node_pos - nearest_point).length();

                    if dist < 6.0 && dist < min_dist {
                        min_dist = dist;
                        closest_node = Some(node);
                    }
                }

                if let Some(node) = closest_node {
                    let data = NodeData {
                        text: node.text.clone(),
                        x: node.position.x,
                        y: node.position.y,
                        z: node.position.z,
                        is_new: false,
                        created_at: "".to_string(),
                    };
                    if let Ok(js_val) = serde_wasm_bindgen::to_value(&data) {
                        update_ui(js_val);
                    }
                    let mut cmd = CAMERA_COMMAND.lock().unwrap();
                    *cmd = Some(CameraCmd::Focus(node.position));
                }
            }
        }
    }
}

fn setup_axis(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let axis_length = 250.0;
    let axis_thickness = 0.3;
    let mut spawn_axis = |dir: Vec3, color: Color| {
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(
                if dir.x != 0.0 { axis_length * 2.0 } else { axis_thickness },
                if dir.y != 0.0 { axis_length * 2.0 } else { axis_thickness },
                if dir.z != 0.0 { axis_length * 2.0 } else { axis_thickness },
            )),
            material: materials.add(StandardMaterial {
                base_color: color.with_alpha(0.3),
                unlit: true,
                ..default()
            }),
            ..default()
        });
    };
    spawn_axis(Vec3::X, Color::Srgba(Srgba::RED));
    spawn_axis(Vec3::Y, Color::Srgba(Srgba::GREEN));
    spawn_axis(Vec3::Z, Color::Srgba(Srgba::BLUE));
}

fn spawn_load_task() {
    wasm_bindgen_futures::spawn_local(async {
        loop {
            if let Some(client) = TURSO_CLIENT.lock().unwrap().as_ref() {
                if let Ok(rows) = client.execute_sql("SELECT text, x, y, z, created_at FROM nodes ORDER BY created_at DESC", vec![]).await {
                    let mut current_nodes = Vec::new();
                    let mut queue = NODE_QUEUE.lock().unwrap();
                    let mut last_count = LAST_KNOWN_COUNT.lock().unwrap();

                    if rows.len() > *last_count {
                        for (i, row) in rows.iter().enumerate() {
                            if let (Some(serde_json::Value::String(text)), Some(x_val), Some(y_val), Some(z_val), Some(time_val)) = 
                                (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4)) {
                                
                                let x = x_val.as_f64().unwrap_or(0.0) as f32;
                                let y = y_val.as_f64().unwrap_or(0.0) as f32;
                                let z = z_val.as_f64().unwrap_or(0.0) as f32;
                                let time = time_val.as_str().unwrap_or("").to_string();

                                let data = NodeData { text: text.clone(), x, y, z, is_new: i < (rows.len() - *last_count), created_at: time };
                                current_nodes.push(data.clone());
                                if i < (rows.len() - *last_count) { queue.push(data); }
                            }
                        }
                        *last_count = rows.len();
                        if let Ok(js_val) = serde_wasm_bindgen::to_value(&current_nodes) {
                            update_recent_nodes(js_val);
                        }
                    }
                }
            }
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
                web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 3000).unwrap();
            })).await.unwrap();
        }
    });
}

fn process_node_queue(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut queue = NODE_QUEUE.lock().unwrap();
    if queue.is_empty() { return; }
    let nodes: Vec<NodeData> = queue.drain(..).collect();
    drop(queue);

    for node in nodes {
        let pos = Vec3::new(node.x, node.y, node.z);
        if pos.length() < 0.1 { continue; }
        let hue = (pos.length() * 20.0) % 360.0;
        let color = Color::hsla(hue, 0.9, 0.6, 1.0);
        
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.2, pos.length(), 1.2)),
            material: materials.add(StandardMaterial { base_color: color.with_alpha(0.5), unlit: true, ..default() }),
            transform: Transform::from_translation(pos / 2.0).with_rotation(Quat::from_rotation_arc(Vec3::Y, pos.normalize())),
            ..default()
        });

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(Cone { radius: 6.0, height: 18.0 })),
                material: materials.add(StandardMaterial { base_color: color, unlit: true, ..default() }),
                transform: Transform::from_translation(pos).with_rotation(Quat::from_rotation_arc(Vec3::Y, pos.normalize())),
                ..default()
            },
            Node { text: node.text, position: pos },
        ));
    }
}

fn sync_camera_commands(mut app_state: ResMut<AppState>) {
    let mut cmd = CAMERA_COMMAND.lock().unwrap();
    if let Some(command) = cmd.take() {
        match command {
            CameraCmd::Focus(pos) => {
                app_state.target_pos = Some(pos);
                app_state.transition_timer = 0.0;
                app_state.is_manual_control = false;
            }
            CameraCmd::Reset => {
                app_state.target_pos = None;
                app_state.is_manual_control = false;
                app_state.transition_timer = 0.0;
            }
        }
    }
}

#[wasm_bindgen]
pub async fn add_node_wasm(text: String) -> Result<JsValue, JsValue> {
    let clean_text = text.trim().to_string();
    if clean_text.is_empty() { return Err(JsValue::from_str("Input empty")); }
    
    let pos = get_semantic_pos(&clean_text);
    if let Some(client) = TURSO_CLIENT.lock().unwrap().as_ref() {
        let existing = client.execute_sql("SELECT x, y, z FROM nodes WHERE text = ?", vec![serde_json::json!(clean_text)]).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if !existing.is_empty() {
            let row = &existing[0];
            let p = Vec3::new(row[0].as_f64().unwrap_or(0.0) as f32, row[1].as_f64().unwrap_or(0.0) as f32, row[2].as_f64().unwrap_or(0.0) as f32);
            let mut cmd = CAMERA_COMMAND.lock().unwrap();
            *cmd = Some(CameraCmd::Focus(p));
            let data = NodeData { text: clean_text, x: p.x, y: p.y, z: p.z, is_new: false, created_at: "".to_string() };
            update_ui(serde_wasm_bindgen::to_value(&data)?);
            return Ok(JsValue::NULL);
        }
        client.execute_sql("INSERT INTO nodes (text, x, y, z) VALUES (?, ?, ?, ?)", 
            vec![serde_json::json!(clean_text), serde_json::json!(pos.x), serde_json::json!(pos.y), serde_json::json!(pos.z)]).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut cmd = CAMERA_COMMAND.lock().unwrap();
        *cmd = Some(CameraCmd::Focus(pos));
        let data = NodeData { text: clean_text, x: pos.x, y: pos.y, z: pos.z, is_new: true, created_at: "".to_string() };
        update_ui(serde_wasm_bindgen::to_value(&data)?);
    }
    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_target() {
    let mut cmd = CAMERA_COMMAND.lock().unwrap();
    *cmd = Some(CameraCmd::Reset);
}
