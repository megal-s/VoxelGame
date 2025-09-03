use std::sync::{Arc, Mutex};

use bevy::{
    app::{Plugin, Update},
    asset::{Assets, Handle},
    color::Color,
    ecs::{
        entity::Entity,
        query::With,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res, ResMut, Single},
    },
    math::{IVec3, Vec2},
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::HashMap,
    render::{
        camera::Camera,
        mesh::{Mesh, Mesh3d},
    },
    state::{condition::in_state, state::OnEnter},
    tasks::AsyncComputeTaskPool,
    transform::components::Transform,
    utils::default,
};
use noiz::{Noise, SampleableFor, prelude::common_noise::Perlin, rng::NoiseRng};

use crate::{
    GameSettings,
    blocks::{BlockManager, BlockManagerResource},
    chunk::{CHUNK_SIZE_F32, ChunkGrid},
    meshing,
};

pub struct ChunkLoaderPlugin;

impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        //Add resource here, then add systems to interact with it
        app.add_systems(OnEnter(crate::GameState::InGame), setup_level)
            .add_systems(
                Update,
                (
                    discard_far_chunks,
                    cleanup_saved_chunks,
                    generate_nearby_chunks,
                    build_chunk_meshes,
                    update_chunk_entities,
                )
                    .chain()
                    .run_if(in_state(crate::GameState::InGame)),
            );
    }
}

#[derive(Resource)]
pub struct Level {
    noise: Noise<Perlin>,
    block_material: Handle<StandardMaterial>,
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    chunk_entities: Arc<Mutex<HashMap<IVec3, Entity>>>,
    load_info: LevelLoadInfo,
}

#[derive(Default)]
struct LevelLoadInfo {
    chunk_states: Arc<Mutex<HashMap<IVec3, ChunkLoadState>>>,
}

enum ChunkLoadState {
    Uninitialized,
    Reading, // Intended for once we're loading chunks from disk
    Generating,
    Generated, // May be renamed once chunks are able to be created via means other than generating
    MeshBuilding,
    MeshReady(Option<Mesh>),
    Saving,
    Saved(Entity),
}

impl Level {
    fn new(seed: u32, material: Handle<StandardMaterial>) -> Self {
        Self {
            noise: Noise::<Perlin> {
                seed: NoiseRng(seed),
                frequency: 1. / CHUNK_SIZE_F32,
                ..Default::default()
            },
            block_material: material,
            chunk_grid: Default::default(),
            chunk_entities: Default::default(),
            load_info: Default::default(),
        }
    }
}

fn setup_level(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    block_manager: Res<BlockManagerResource>,
) {
    let block_manager = block_manager.lock().expect("Block manager mutex poisoned");
    commands.insert_resource(Level::new(
        0,
        materials.add(StandardMaterial {
            base_color_texture: Some(block_manager.atlas_texture().expect("Atlas is not built")),
            base_color: Color::WHITE,
            ..default()
        }),
    ));
}

fn generate_nearby_chunks(
    level: Res<Level>,
    settings: Res<GameSettings>,
    camera_query: Single<&Transform, With<Camera>>,
) {
    let Ok(mut chunk_states) = level.load_info.chunk_states.try_lock() else {
        return;
    };

    let Ok(chunk_grid) = level.chunk_grid.try_lock() else {
        return;
    };

    let camera_position = ChunkGrid::to_chunk_coordinates(camera_query.into_inner().translation);

    for x in camera_position.x - settings.horizontal_render_distance
        ..=camera_position.x + settings.horizontal_render_distance
    {
        for y in camera_position.y - settings.vertical_render_distance
            ..=camera_position.y + settings.vertical_render_distance
        {
            for z in camera_position.z - settings.horizontal_render_distance
                ..=camera_position.z + settings.horizontal_render_distance
            {
                let position = IVec3::new(x, y, z);
                if chunk_states.contains_key(&position) || chunk_grid.chunks.contains_key(&position)
                {
                    continue;
                }

                chunk_states.insert(position, ChunkLoadState::Uninitialized);

                let chunk_grid = level.chunk_grid.clone();
                let chunk_states = level.load_info.chunk_states.clone();
                let noise = level.noise;
                AsyncComputeTaskPool::get()
                    .spawn(generate_chunk(chunk_grid, chunk_states, position, noise))
                    .detach();
            }
        }
    }
}

async fn generate_chunk(
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    chunk_states: Arc<Mutex<HashMap<IVec3, ChunkLoadState>>>,
    position: IVec3,
    noise: impl SampleableFor<Vec2, f32>,
) {
    {
        let mut chunk_states = chunk_states.lock().expect("Chunk states mutex poisoned");
        if let Some(state) = chunk_states.get(&position)
            && let &ChunkLoadState::Uninitialized = state
        {
            chunk_states.insert(position, ChunkLoadState::Generating);
        } else {
            return;
        };
    }

    let chunk = ChunkGrid::generate_or_load_chunk(position, &noise);
    chunk_grid
        .lock()
        .expect("Chunk grid mutex poisoned")
        .chunks
        .insert(position, chunk);
    chunk_states
        .lock()
        .expect("Chunk states mutex poisoned")
        .insert(position, ChunkLoadState::Generated);
}

fn build_chunk_meshes(level: Res<Level>, block_manager: Res<BlockManagerResource>) {
    let Ok(chunk_states) = level.load_info.chunk_states.try_lock() else {
        return;
    };

    let generated_chunks = chunk_states
        .iter()
        .filter_map(|(position, state)| match state {
            ChunkLoadState::Generated => Some(*position),
            _ => None,
        });

    for position in generated_chunks {
        let chunk_grid = level.chunk_grid.clone();
        let chunk_states = level.load_info.chunk_states.clone();
        let block_manager = block_manager.clone();
        AsyncComputeTaskPool::get()
            .spawn(build_chunk_mesh(
                chunk_grid,
                chunk_states,
                block_manager,
                position,
            ))
            .detach();
    }
}

async fn build_chunk_mesh(
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    chunk_states: Arc<Mutex<HashMap<IVec3, ChunkLoadState>>>,
    block_manager: Arc<Mutex<BlockManager>>,
    position: IVec3,
) {
    {
        let mut chunk_states = chunk_states.lock().expect("Chunk states mutex poisoned");
        if !matches!(
            chunk_states.get(&position),
            Some(&ChunkLoadState::Generated)
        ) {
            return;
        }
        chunk_states.insert(position, ChunkLoadState::MeshBuilding);
    }

    let mesh = {
        let chunk_grid = chunk_grid.lock().expect("Chunk grid mutex poisoned");
        let Some(chunk) = chunk_grid.chunks.get(&position) else {
            eprintln!("Chunk not found during mesh generation for chunk {position}!");
            return;
        };
        meshing::rebuild_chunk_mesh(
            &chunk_grid,
            &block_manager.lock().expect("Block manager mutex poisoned"),
            chunk,
        )
    };

    let mut chunk_states = chunk_states.lock().expect("Chunk states mutex poisoned");

    if let Some(state) = chunk_states.get(&position)
        && let &ChunkLoadState::MeshBuilding = state
    {
        chunk_states.insert(position, ChunkLoadState::MeshReady(mesh));
    }
}

fn update_chunk_entities(
    mut commands: Commands,
    level: Res<Level>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(mut chunk_states) = level.load_info.chunk_states.try_lock() else {
        return;
    };

    let Ok(mut chunk_entities) = level.chunk_entities.try_lock() else {
        return;
    };

    let completed_meshes = chunk_states
        .iter_mut()
        .filter_map(|(position, state)| match state {
            ChunkLoadState::MeshReady(mesh) => Some((*position, mesh.take())),
            _ => None,
        })
        .collect::<Vec<(IVec3, Option<Mesh>)>>();

    for (position, mesh) in completed_meshes {
        if let Some(entity) = chunk_entities.get(&position) {
            let mut entity = commands.entity(*entity);
            match mesh {
                Some(mesh) => entity.insert(Mesh3d(meshes.add(mesh))),
                None => entity.remove::<Mesh3d>(),
            };
            continue;
        }

        let mut entity = commands.spawn((
            MeshMaterial3d(level.block_material.clone()),
            Transform::from_xyz(
                position.x as f32 * CHUNK_SIZE_F32,
                position.y as f32 * CHUNK_SIZE_F32,
                position.z as f32 * CHUNK_SIZE_F32,
            ),
        ));
        if let Some(mesh) = mesh {
            entity.insert(Mesh3d(meshes.add(mesh)));
        };

        chunk_entities.insert(position, entity.id());
        chunk_states.remove(&position);
    }
}

fn discard_far_chunks(
    level: Res<Level>,
    settings: Res<GameSettings>,
    camera_query: Single<&Transform, With<Camera>>,
) {
    let Ok(mut chunk_states) = level.load_info.chunk_states.try_lock() else {
        return;
    };

    let far_chunks = {
        let Ok(chunk_grid) = level.chunk_grid.try_lock() else {
            return;
        };

        let camera_position =
            ChunkGrid::to_chunk_coordinates(camera_query.into_inner().translation);
        chunk_grid
            .chunks
            .keys()
            .filter_map(|position| {
                let diff = (position - camera_position).abs();
                if (diff.x <= settings.horizontal_render_distance
                    && diff.y <= settings.vertical_render_distance
                    && diff.z <= settings.horizontal_render_distance)
                    || chunk_states.contains_key(position)
                {
                    return None;
                }
                Some(*position)
            })
            .collect::<Vec<IVec3>>()
    };

    for position in far_chunks {
        chunk_states.insert(position, ChunkLoadState::Saving);

        let chunk_grid = level.chunk_grid.clone();
        let chunk_entities = level.chunk_entities.clone();
        let chunk_states = level.load_info.chunk_states.clone();
        AsyncComputeTaskPool::get()
            .spawn(save_chunk(
                chunk_grid,
                chunk_entities,
                chunk_states,
                position,
            ))
            .detach();
    }
}

async fn save_chunk(
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    chunk_entities: Arc<Mutex<HashMap<IVec3, Entity>>>,
    chunk_states: Arc<Mutex<HashMap<IVec3, ChunkLoadState>>>,
    position: IVec3,
) {
    chunk_grid
        .lock()
        .expect("Chunk grid mutex poisoned")
        .chunks
        .remove(&position);
    let entity = chunk_entities
        .lock()
        .expect("Chunk entities mutex poisoned")
        .remove(&position);
    let mut chunk_states = chunk_states.lock().expect("Chunk states mutex poisoned");
    match entity {
        Some(entity) => chunk_states.insert(position, ChunkLoadState::Saved(entity)),
        None => chunk_states.remove(&position),
    };
}

fn cleanup_saved_chunks(mut commands: Commands, level: Res<Level>) {
    let Ok(mut chunk_states) = level.load_info.chunk_states.try_lock() else {
        return;
    };

    let saved_chunks = chunk_states
        .iter()
        .filter_map(|(position, state)| match state {
            ChunkLoadState::Saved(entity) => Some((*position, *entity)),
            _ => None,
        })
        .collect::<Vec<(IVec3, Entity)>>();

    for (position, entity) in saved_chunks {
        chunk_states.remove(&position);
        commands.entity(entity).despawn();
    }
}
