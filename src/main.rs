use bevy::{
    DefaultPlugins, app::App, ecs::system::Commands, math::I16Vec3, platform::collections::HashMap,
    prelude::*, window::PrimaryWindow,
};
use bevy_asset_loader::prelude::*;
use blocks::BlockManager;
use chunk::{Block, Chunk};
use noiz::{
    Noise, Sampleable, SampleableFor, ScalableNoise, SeedableNoise,
    cells::OrthoGrid,
    prelude::{PerCell, common_noise::Perlin},
    rng::{Random, UNorm},
};

use crate::{
    chunk::{BlockGrid, CHUNK_SIZE_F32, ChunkGrid},
    game::camera_movement::MovableCamera,
};

mod blocks;
mod chunk;
mod game;

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

#[derive(Default, Resource)]
struct ChunkWorld {
    chunk_grid: ChunkGrid,
    meshes: HashMap<IVec3, Handle<Mesh>>,
}

#[derive(Component)]
struct DebugPositionText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(game::camera_movement::CameraMovementPlugin)
        .init_resource::<BlockManager>()
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
            update_debug_position_text.run_if(in_state(GameState::InGame)),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    block_manager: Res<BlockManager>,
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
            speed: 60.,
            sensitivity: 0.002,
        },
        Projection::from(PerspectiveProjection {
            fov: 90_f32.to_radians(),
            ..Default::default()
        }),
    ));

    // Setup chunks
    // Temporary code, generation will be added later
    let mut chunk_world = ChunkWorld::default();

    let mut noise = Noise::<Perlin>::default();
    noise.set_seed(10);
    noise.set_period(CHUNK_SIZE_F32);
    const MIN_CHUNK: i32 = -2;
    const MAX_CHUNK: i32 = 2;
    for x in MIN_CHUNK..=MAX_CHUNK {
        for y in MIN_CHUNK..=MAX_CHUNK {
            for z in MIN_CHUNK..=MAX_CHUNK {
                chunk_world
                    .chunk_grid
                    .generate_chunk(IVec3::new(x, y, z), &noise);
            }
        }
    }

    // let mut contents = BlockGrid::new();
    // contents
    //     .set(I16Vec3::new(0, 0, 0), Block("stone".to_string()))
    //     .unwrap();
    // contents
    //     .set(I16Vec3::new(1, 0, 0), Block("dirt".to_string()))
    //     .unwrap();
    // contents
    //     .set(I16Vec3::new(2, 0, 0), Block("doesn't exist".to_string()))
    //     .unwrap();

    // chunk_world.chunk_grid.chunks.insert(
    //     IVec3::default(),
    //     Chunk {
    //         position: IVec3::default(),
    //         contents,
    //     },
    // );

    let chunk_meshes =
        game::chunk_mesh::rebuild_chunk_meshes(&chunk_world.chunk_grid, &block_manager);

    let mesh_mat = materials.add(StandardMaterial {
        base_color_texture: Some(block_manager.atlas_texture().expect("Atlas is not built")),
        base_color: Color::WHITE,
        ..default()
    });

    for (chunk_position, mesh) in chunk_meshes {
        let mesh_handle = meshes.add(mesh);
        chunk_world
            .meshes
            .insert(chunk_position, mesh_handle.clone());
        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(mesh_mat.clone()),
            Transform::from_xyz(
                chunk_position.x as f32 * CHUNK_SIZE_F32,
                chunk_position.y as f32 * CHUNK_SIZE_F32,
                chunk_position.z as f32 * CHUNK_SIZE_F32,
            ),
        ));
    }

    commands.insert_resource(chunk_world);

    // Debug info
    commands.spawn((
        DebugPositionText,
        Text::new("Raw   x/y/z: ?\nBlock x/y/z: ? (?)\nChunk x/y/z: ?"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        },
    ));
}

fn update_debug_position_text(
    camera_query: Single<&Transform, With<Camera>>,
    text_query: Single<&mut Text, With<DebugPositionText>>,
) {
    let camera_position = camera_query.into_inner().translation;
    let int_camera_position = IVec3::new(
        camera_position.x as i32,
        camera_position.y as i32,
        camera_position.z as i32,
    );
    text_query.into_inner().0 = format!(
        "Raw   x/y/z: {}\nBlock x/y/z: {} ({})\nChunk x/y/z: {}",
        camera_position,
        int_camera_position,
        BlockGrid::to_block_coordinates(int_camera_position),
        ChunkGrid::to_chunk_coordinates(camera_position),
    );
}

fn setup_blocks(
    mut commands: Commands,
    block_assets: Res<crate::BlockAssets>,
    textures: ResMut<Assets<Image>>,
    block_manager: ResMut<BlockManager>,
) {
    let block_manager = block_manager.into_inner();

    block_manager.set_error_texture(block_assets.error.clone());
    block_manager.add_block("stone", block_assets.stone.clone());
    block_manager.add_block("dirt", block_assets.dirt.clone());

    block_manager.rebuild_atlas(textures.into_inner());

    commands.set_state(crate::GameState::InGame);
}
