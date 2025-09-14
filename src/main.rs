/* (MVP) Things needed:
 *  > Chunk modules
 *      - Render
 *          - Meshing [✓]
 *      - Logic
 *          - Storage [✓]
 *          - Generation [✓]
 *          - Saving [✓]
 *          - Loading [✓]
 *  > Block modules
 *      - Render
 *          - Mesh info [✓]
 *      - Logic
 *          - ID [✓]
 *  > Player modules
 *      - Camera movement [✓]
 *      - Block interactions
 *  > Atlasing
 *      - Folder definition
 *      - Stitching not bound by startup
 *  > Level
 *      - Settings
 *          - ID [✓]
 *          - Seed [✓]
 *      - Generation [✓]
 *      - Saving
 *      - Loading
 *  > Game state
 *      - Startup
 *      - Resource parsing/atlasing
 *      - Paused
 */

use std::sync::Arc;

use bevy::{
    DefaultPlugins,
    app::{App, Update},
    asset::Assets,
    color::{Alpha, Color},
    core_pipeline::core_3d::Camera3d,
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        query::{With, Without},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res, ResMut, Single},
    },
    image::Image,
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseWheel},
    math::{IVec3, Vec3, primitives::Cuboid},
    pbr::{AmbientLight, MeshMaterial3d, StandardMaterial},
    prelude::PluginGroup,
    render::{
        camera::{Camera, PerspectiveProjection, Projection},
        mesh::{Mesh, Mesh3d},
        texture::ImagePlugin,
    },
    state::{
        app::AppExtStates,
        commands::CommandsStatesExt,
        condition::in_state,
        state::{OnEnter, States},
    },
    text::TextLayout,
    transform::components::Transform,
    ui::{BackgroundColor, Node, PositionType, Val, widget::Text},
    window::{PrimaryWindow, Window},
};
use bevy_asset_loader::loading_state::{
    LoadingState, LoadingStateAppExt, config::ConfigureLoadingState,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockAssets, BlockAtlasManager, BlockRay},
    camera_control::MovableCamera,
    chunk::{Chunk, ChunkGrid},
    level::Level,
};

mod atlas;
mod block;
mod camera_control;
mod chunk;
mod level;

pub const DEFAULT_NAMESPACE: &str = "builtin";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Identifier(pub String, pub String);

impl Identifier {
    pub fn new(namespace: &str, path: &str) -> Self {
        Self(namespace.to_owned(), path.to_owned())
    }

    pub fn as_string(&self) -> String {
        format!("{}:{}", self.0, self.1)
    }

    pub fn with_path(&self, path: &str) -> Self {
        Self(self.0.clone(), path.to_owned())
    }

    pub fn with_namespace(&self, namespace: &str) -> Self {
        Self(namespace.to_owned(), self.1.clone())
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    AssetLoading,
    CreateAtlases,
    InGame,
}

#[derive(Resource)]
struct GameSettings {
    horizontal_render_distance: i32,
    vertical_render_distance: i32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            horizontal_render_distance: 3,
            vertical_render_distance: 3,
        }
    }
}

#[derive(Component)]
struct DebugText;

#[derive(Component)]
struct DebugBlockOutline;

#[derive(Component)]
struct DebugBlockNormalOutline;

#[derive(Default, Resource)]
struct PersistentDebugInformation {
    ray_mesh_entities: Vec<Entity>,
    constant_ray_mesh_entities: Vec<Entity>,
    show_constant_entities: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // TODO; replace with only those needed
        .add_plugins(camera_control::CameraMovementPlugin)
        .add_plugins(level::LevelPlugin)
        .init_resource::<GameSettings>()
        .init_resource::<PersistentDebugInformation>()
        .init_resource::<BlockAtlasManager>()
        .init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::CreateAtlases)
                .load_collection::<BlockAssets>(),
        )
        .add_systems(OnEnter(GameState::CreateAtlases), setup_atlases)
        .add_systems(OnEnter(GameState::InGame), setup_world)
        .add_systems(
            Update,
            (update_debug_text, handle_debug_input).run_if(in_state(GameState::InGame)),
        )
        .run();
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    window_query: Single<&mut Window, With<PrimaryWindow>>,
) {
    // Setup window
    let mut window = window_query.into_inner();
    window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Confined;
    window.cursor_options.visible = false;
    window.focused = true;

    // Setup camera
    commands.spawn((
        AmbientLight {
            brightness: 300.,
            ..Default::default()
        },
        Camera3d::default(),
        MovableCamera {
            speed: 15.,
            sensitivity: 0.002,
        },
        Projection::from(PerspectiveProjection {
            fov: 90_f32.to_radians(),
            ..Default::default()
        }),
    ));

    // Crosshair
    commands.spawn((
        BackgroundColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            justify_self: bevy::ui::JustifySelf::Center,
            align_self: bevy::ui::AlignSelf::Center,
            width: Val::Px(10.),
            height: Val::Px(10.),
            ..Default::default()
        },
    ));

    // Block outline
    commands.spawn((
        DebugBlockOutline,
        Mesh3d(meshes.add(Cuboid::from_length(1.02))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(Color::WHITE.with_alpha(0.5)))),
        Transform::from_translation(Vec3::ZERO),
    ));

    commands.spawn((
        DebugBlockNormalOutline,
        Mesh3d(meshes.add(Cuboid::from_length(1.01))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(Color::srgba(1., 1., 0., 0.25)))),
        Transform::from_translation(Vec3::ZERO),
    ));

    // Debug info
    commands.spawn((
        DebugText,
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..Default::default()
        },
    ));

    commands.spawn((
        Text::new("[Mouse Wheel]: Change camera movement speed\n[Arrow Keys]: Change render distance\n[E]: Place block\n[Q]: Remove block\n[R]: Toggle ray overlay"),
        TextLayout::new_with_justify(bevy::text::JustifyText::Right),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            right: Val::Px(5.0),
            ..Default::default()
        },
    ));
}

fn setup_atlases(
    mut commands: Commands,
    block_assets: Res<BlockAssets>,
    textures: ResMut<Assets<Image>>,
    mut block_atlas_manager: ResMut<BlockAtlasManager>,
) {
    let block_manager = Arc::make_mut(&mut block_atlas_manager.0);

    block_manager.set_error_texture(block_assets.error.clone());
    block_manager.add_data(
        Identifier(DEFAULT_NAMESPACE.to_owned(), "stone".to_owned()),
        block_assets.stone.clone(),
    );
    block_manager.add_data(
        Identifier(DEFAULT_NAMESPACE.to_owned(), "dirt".to_owned()),
        block_assets.dirt.clone(),
    );

    block_manager.rebuild_atlas(textures.into_inner());

    commands.set_state(crate::GameState::InGame);
}

fn update_debug_text(
    settings: Res<GameSettings>,
    camera_query: Single<(&MovableCamera, &Transform)>,
    text_query: Single<&mut Text, With<DebugText>>,
) {
    let camera_position = camera_query.1.translation;
    text_query.into_inner().0 = format!(
        "Raw   x/y/z: {}\nBlock x/y/z: {} ({})\nChunk x/y/z: {}\n\nCamera Speed: {}\nRender Distance: [h:{}, v:{}]",
        camera_position,
        camera_position.floor().as_ivec3(),
        Chunk::to_block_coordinates(camera_position.floor().as_ivec3()),
        ChunkGrid::to_chunk_coordinates(camera_position),
        camera_query.0.speed,
        settings.horizontal_render_distance,
        settings.vertical_render_distance
    );
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn handle_debug_input(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut settings: ResMut<GameSettings>,
    mut debug_info: ResMut<PersistentDebugInformation>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mouse_wheel_input: EventReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Single<(&mut MovableCamera, &Transform)>,
    mut block_outline_query: Single<
        &mut Transform,
        (
            With<DebugBlockOutline>,
            Without<MovableCamera>,
            Without<DebugBlockNormalOutline>,
        ),
    >,
    mut block_outline_normal_query: Single<
        &mut Transform,
        (
            With<DebugBlockNormalOutline>,
            Without<MovableCamera>,
            Without<DebugBlockOutline>,
        ),
    >,
) {
    // Change chunk render distance
    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        settings.vertical_render_distance += 1;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        settings.vertical_render_distance -= 1;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        settings.horizontal_render_distance += 1;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
        settings.horizontal_render_distance -= 1;
    }
    // Toggle visibility of block interaction ray steps for current camera position+rotation
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        debug_info.show_constant_entities = !debug_info.show_constant_entities;
    }
    // Place/Destroy block
    let mut block_interaction = None;
    if keyboard_input.just_pressed(KeyCode::KeyE) {
        block_interaction = Some(true);
    }
    if keyboard_input.just_pressed(KeyCode::KeyQ) {
        block_interaction = Some(false);
    }
    // Change camera move speed
    for event in mouse_wheel_input.read() {
        camera_query.0.speed += event.y;
        camera_query.0.speed = camera_query.0.speed.clamp(0., 100.);
    }
    // Cleanup entities created when rendering block interaction ray steps
    for entity in debug_info.constant_ray_mesh_entities.drain(..) {
        commands.entity(entity).despawn();
    }
    if block_interaction.is_some() {
        for entity in debug_info.ray_mesh_entities.drain(..) {
            commands.entity(entity).despawn();
        }
        // Draw cube indicating camera position when the current ray was cast
        debug_info.ray_mesh_entities.push(
            commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::from_length(0.15))),
                    MeshMaterial3d(
                        materials.add(StandardMaterial::from_color(Color::srgba(0., 1., 1., 0.75))),
                    ),
                    Transform::from_translation(camera_query.1.translation),
                ))
                .id(),
        );
        // Draw line of cubes from camera position/rotation indicating where the ray is expected to end
        for i in 0..200 {
            debug_info.ray_mesh_entities.push(
                commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::from_length(0.075))),
                        MeshMaterial3d(
                            materials
                                .add(StandardMaterial::from_color(Color::srgba(0.25, 0., 1., 1.))),
                        ),
                        Transform::from_translation(
                            camera_query.1.translation + camera_query.1.forward() * (i as f32 / 2.),
                        ),
                    ))
                    .id(),
            )
        }
    }
    // Create block interaction ray
    let mut ray = BlockRay::from_origin_in_direction(
        camera_query.1.translation,
        camera_query.1.forward().normalize(),
    );
    // Initialize first chunk to check for solid blocks
    let mut chunk_position = ChunkGrid::to_chunk_coordinates(ray.position);
    let Some(mut chunk) = level.get_chunk_grid().0.get(&chunk_position) else {
        // Inhabited chunk not loaded so we shouldn't modifiy it
        return;
    };
    // Step over block interaction ray and return a chunk position if a block was hit
    let rebuild = loop {
        // Draw cubes at current ray position and normal
        let position_entity = (
            Mesh3d(meshes.add(Cuboid::from_length(0.1))),
            MeshMaterial3d(
                materials.add(StandardMaterial::from_color(Color::srgba(1., 0., 1., 0.75))),
            ),
            Transform::from_translation(ray.position),
        );
        let normal_entity = (
            Mesh3d(meshes.add(Cuboid::from_length(0.05))),
            MeshMaterial3d(
                materials.add(StandardMaterial::from_color(Color::srgba(0., 1., 0., 0.75))),
            ),
            Transform::from_translation(ray.position + ray.normal * 0.1),
        );
        if debug_info.show_constant_entities {
            debug_info
                .constant_ray_mesh_entities
                .push(commands.spawn(position_entity.clone()).id());
            debug_info
                .constant_ray_mesh_entities
                .push(commands.spawn(normal_entity.clone()).id());
        }
        if block_interaction.is_some() {
            debug_info
                .ray_mesh_entities
                .push(commands.spawn(position_entity).id());
            debug_info
                .ray_mesh_entities
                .push(commands.spawn(normal_entity).id());
        }

        // Check if the position of the current ray step is still within the chunk being checked
        // Seperate variable used to store the variable here to avoid locking chunk rwlock
        let ray_chunk_position = ChunkGrid::to_chunk_coordinates(ray.position);
        if ray_chunk_position != chunk_position {
            chunk_position = ray_chunk_position;
            let Some(ray_chunk) = level.get_chunk_grid().0.get(&chunk_position) else {
                // Chunk at current ray step not loaded so we cant continue checking the ray any further
                break None;
            };
            chunk = ray_chunk;
        }

        // Get index of the block at the current ray step
        let target_block_index =
            Chunk::to_index(Chunk::to_block_coordinates(ray.position.floor().as_ivec3()));
        // Check if block at previously defined index is solid
        if chunk.read().expect("Chunk rw poisoned").contents[target_block_index].is_none() {
            ray.step();
            continue;
        }

        // Set overlay positions so we can see where ray ended up
        block_outline_query.translation = ray.position.floor() + 0.5;
        block_outline_normal_query.translation = ray.position.floor() + ray.normal.floor() + 0.5;

        // Get the block interaction we wish to do this frame or else end the ray here if there is none
        let Some(block_interaction) = block_interaction else {
            break None;
        };

        // Place a block at the ray position offset by the ray normal
        if block_interaction {
            // If the position ends up in a different chunk when offset by the normal load that chunk
            let ray_chunk_position = ChunkGrid::to_chunk_coordinates(ray.position + ray.normal);
            if ray_chunk_position != chunk_position {
                chunk_position = ray_chunk_position;
                let Some(ray_chunk) = level.get_chunk_grid().0.get(&chunk_position) else {
                    // Chunk not loaded so exit early
                    break None;
                };
                chunk = ray_chunk;
            }
            chunk.write().expect("Chunk rw poisoned").contents[Chunk::to_index(
                Chunk::to_block_coordinates((ray.position + ray.normal).floor().as_ivec3()),
            )] = Some(Block::new(Identifier::new(DEFAULT_NAMESPACE, "dirt")));
        }
        // Remove the block at the ray position
        else {
            chunk.write().expect("Chunk rw poisoned").contents[target_block_index] = None;
        }
        break Some(chunk_position);
    };

    // Rebuild modified chunk mesh (if a chunk was modified)
    if let Some(chunk_position) = rebuild {
        level.rebuild_mesh(chunk_position);
    }
}
