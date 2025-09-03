use bevy::{DefaultPlugins, app::App, ecs::system::Commands, prelude::*, window::PrimaryWindow};
use bevy_asset_loader::prelude::*;
use blocks::Block;

use crate::{
    blocks::BlockManagerResource,
    chunk::{BlockGrid, ChunkGrid},
    game::camera_movement::MovableCamera,
    level::Level,
};

mod blocks;
mod chunk;
mod game;
mod level;
mod meshing;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    AssetLoading,
    CreateAtlas,
    InGame,
}

#[derive(AssetCollection, Resource)]
struct BlockAssets {
    #[asset(path = "Error.png")]
    error: Handle<Image>,
    #[asset(path = "Stone.png")]
    stone: Handle<Image>,
    #[asset(path = "Dirt.png")]
    dirt: Handle<Image>,
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
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(game::camera_movement::CameraMovementPlugin)
        .add_plugins(level::ChunkLoaderPlugin)
        .init_resource::<BlockManagerResource>()
        .insert_resource(GameSettings::default())
        .init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::CreateAtlas)
                .load_collection::<BlockAssets>(),
        )
        .add_systems(OnEnter(GameState::CreateAtlas), setup_blocks)
        .add_systems(OnEnter(GameState::InGame), setup)
        .add_systems(
            Update,
            update_debug_text.run_if(in_state(GameState::InGame)),
        )
        .run();
}

fn setup(mut commands: Commands, window_query: Single<&mut Window, With<PrimaryWindow>>) {
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
            ..default()
        },
    ));
}

fn update_debug_text(
    level: Res<Level>,
    camera_query: Single<&Transform, With<Camera>>,
    text_query: Single<&mut Text, With<DebugText>>,
) {
    let camera_position = camera_query.into_inner().translation;
    let int_camera_position = IVec3::new(
        camera_position.x as i32,
        camera_position.y as i32,
        camera_position.z as i32,
    );
    text_query.into_inner().0 = format!(
        "Raw   x/y/z: {}\nBlock x/y/z: {} ({})\nChunk x/y/z: {}\n\nChunk Count: {}",
        camera_position,
        int_camera_position,
        BlockGrid::to_block_coordinates(int_camera_position),
        ChunkGrid::to_chunk_coordinates(camera_position),
        level.loaded_chunks.len(),
    );
}

fn setup_blocks(
    mut commands: Commands,
    block_assets: Res<crate::BlockAssets>,
    textures: ResMut<Assets<Image>>,
    block_manager: Res<BlockManagerResource>,
) {
    let mut block_manager = block_manager.into_inner().lock().unwrap();

    block_manager.set_error_texture(block_assets.error.clone());
    block_manager.add_block(Block::new("stone"), block_assets.stone.clone());
    block_manager.add_block(Block::new("dirt"), block_assets.dirt.clone());

    block_manager.rebuild_atlas(textures.into_inner());

    commands.set_state(crate::GameState::InGame);
}
