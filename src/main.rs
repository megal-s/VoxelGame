use bevy::{
    DefaultPlugins, app::App, color::palettes::css::WHITE, ecs::system::Commands, math::I16Vec3,
    platform::collections::HashMap, prelude::*, window::PrimaryWindow,
};
use noiz::{
    Noise, SampleableFor, SeedableNoise,
    cells::OrthoGrid,
    prelude::PerCell,
    rng::{Random, UNorm},
};

use crate::{
    chunk::{Block, BlockGrid, CHUNK_SIZE_F32, Chunk, ChunkGrid},
    game::camera_movement::MovableCamera,
};

mod chunk;
mod game;

#[derive(Default, Resource)]
struct ChunkWorld {
    chunk_grid: ChunkGrid,
    meshes: HashMap<IVec3, Handle<Mesh>>,
}

#[derive(Component)]
struct DebugPositionText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(game::camera_movement::CameraMovementPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update_debug_position_text)
        .run();
}

fn setup(
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
            speed: 20.,
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
    let mut noise = Noise::<PerCell<OrthoGrid, Random<UNorm, f32>>>::default();
    noise.set_seed(10);
    const MIN_CHUNK: i32 = -2;
    const MAX_CHUNK: i32 = 2;
    for x in MIN_CHUNK..=MAX_CHUNK {
        for y in MIN_CHUNK..=MAX_CHUNK {
            for z in MIN_CHUNK..=MAX_CHUNK {
                /*if x.abs() % 2 == 1 || y.abs() % 2 == 1 || z.abs() % 2 == 1 {
                    continue;
                }*/

                chunk_world
                    .chunk_grid
                    .generate_chunk(IVec3::new(x, y, z), &noise);
                /*
                let position = IVec3::new(x, y, z);
                let mut contents = BlockGrid::new();
                contents.set_area(I16Vec3::ZERO, I16Vec3::splat(31), Block(1));
                chunk_world.chunk_grid.chunks.insert(position, Chunk { position, contents });
                */
            }
        }
    }

    let chunk_meshes = game::chunk_mesh::create_chunk_meshes(&chunk_world.chunk_grid);
    let mesh_mat = materials.add(StandardMaterial::from_color(WHITE));
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
