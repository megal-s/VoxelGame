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
    core_pipeline::core_3d::Camera3d,
    ecs::{
        component::Component,
        query::With,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res, ResMut, Single},
    },
    image::Image,
    input::{ButtonInput, keyboard::KeyCode},
    math::IVec3,
    pbr::AmbientLight,
    render::camera::{Camera, PerspectiveProjection, Projection},
    state::{
        app::AppExtStates,
        commands::CommandsStatesExt,
        condition::in_state,
        state::{OnEnter, States},
    },
    text::TextLayout,
    transform::components::Transform,
    ui::{Node, PositionType, Val, widget::Text},
    window::{PrimaryWindow, Window},
};
use bevy_asset_loader::loading_state::{
    LoadingState, LoadingStateAppExt, config::ConfigureLoadingState,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockAssets, BlockAtlasManager, BlockRaycast},
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins) // TODO; replace with only those needed
        .add_plugins(camera_control::CameraMovementPlugin)
        .add_plugins(level::LevelPlugin)
        .insert_resource(GameSettings::default())
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

fn setup_world(mut commands: Commands, window_query: Single<&mut Window, With<PrimaryWindow>>) {
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
            speed: 60.,
            sensitivity: 0.002,
        },
        Projection::from(PerspectiveProjection {
            fov: 90_f32.to_radians(),
            ..Default::default()
        }),
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
        Text::new("[Arrow Keys]: Change render distance\n[E]: Place block\n[Q]: Remove block"),
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
    camera_query: Single<&Transform, With<Camera>>,
    text_query: Single<&mut Text, With<DebugText>>,
) {
    let camera_position = camera_query.translation;
    let int_camera_position = IVec3::new(
        camera_position.x as i32,
        camera_position.y as i32,
        camera_position.z as i32,
    );
    text_query.into_inner().0 = format!(
        "Raw   x/y/z: {}\nBlock x/y/z: {} ({})\nChunk x/y/z: {}\n\nRender Distance: [h:{}, v:{}]",
        camera_position,
        int_camera_position,
        Chunk::to_block_coordinates(int_camera_position),
        ChunkGrid::to_chunk_coordinates(camera_position),
        settings.horizontal_render_distance,
        settings.vertical_render_distance
    );
}

fn handle_debug_input(
    mut level: ResMut<Level>,
    mut settings: ResMut<GameSettings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Single<&Transform, With<Camera>>,
) {
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
    let mut block_interaction = None;
    if keyboard_input.just_pressed(KeyCode::KeyE) {
        block_interaction = Some(true);
    }
    if keyboard_input.just_pressed(KeyCode::KeyQ) {
        block_interaction = Some(false);
    }

    let Some(block_interaction) = block_interaction else {
        return;
    };

    let mut chunk_position = ChunkGrid::to_chunk_coordinates(camera_query.translation);
    let Some(mut chunk) = level.get_chunk_grid().0.get(&chunk_position) else {
        return;
    };
    let mut raycast = BlockRaycast::from_origin_in_direction(
        camera_query.translation,
        camera_query.forward().normalize(),
    );
    let rebuild = loop {
        let position = raycast.query_position();
        {
            let current_chunk_position = ChunkGrid::to_chunk_coordinates(position);
            if current_chunk_position != chunk_position {
                let Some(c) = level.get_chunk_grid().0.get(&current_chunk_position) else {
                    break None;
                };
                chunk_position = current_chunk_position;
                chunk = c;
            }
        }

        let index = Chunk::to_index(Chunk::to_block_coordinates(position.floor().as_ivec3()));
        if chunk.read().expect("Chunk rw poisoned").contents[index].is_none() {
            raycast.step();
            continue;
        }
        if block_interaction {
            chunk.write().expect("Chunk rw poisoned").contents[index] =
                Some(Block::new(Identifier::new(DEFAULT_NAMESPACE, "dirt")));
        } else {
            chunk.write().expect("Chunk rw poisoned").contents[index] = None;
        }
        break Some(position);
    };

    if let Some(position) = rebuild {
        level.rebuild_mesh(ChunkGrid::to_chunk_coordinates(position));
    }
}
