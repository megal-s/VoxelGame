use std::{
    collections::VecDeque,
    fs,
    ops::DerefMut,
    sync::{Arc, Mutex, RwLock, Weak},
};

use bevy::{
    app::{App, Plugin, Update},
    asset::{Assets, Handle},
    color::Color,
    ecs::{
        entity::Entity,
        query::With,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res, ResMut, Single},
    },
    math::{IVec2, IVec3, Vec2, Vec2Swizzles},
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::{HashMap, HashSet},
    render::{
        camera::Camera,
        mesh::{Mesh, Mesh3d},
    },
    state::{condition::in_state, state::OnEnter},
    tasks::{AsyncComputeTaskPool, IoTaskPool},
    transform::components::Transform,
    utils::default,
};
use noiz::{Noise, SampleableFor, prelude::common_noise::Perlin, rng::NoiseRng};
use serde::Deserialize;

use crate::{
    GameSettings, GameState,
    atlas::AtlasManager,
    block::BlockAtlasManager,
    chunk::{self, Chunk, ChunkGrid},
};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_level)
            .add_systems(
                Update,
                (
                    mark_nearby_chunks_uninitialized,
                    finalize_chunk_generation,
                    handle_remesh_queue,
                    apply_ready_meshes,
                    remove_far_chunks,
                    cleanup_saved_chunks,
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

/// Resource from which all level data is defined and accessed
#[derive(Resource)]
struct Level {
    level_properties: LevelProperties,
    chunk_properties: ChunkProperties,
    mesh_properties: MeshProperties,
    bevy_properties: BevyProperties,
}

struct LevelProperties {
    id: String,
    seed: u32,
}

#[derive(Default)]
struct ChunkProperties {
    chunk_grid: ChunkGrid,
    chunk_states: Arc<RwLock<HashMap<IVec3, Mutex<ChunkGenerationState>>>>,
    removed: HashSet<IVec3>,
}

enum ChunkGenerationState {
    Uninitialized,
    Ready(Option<Chunk>),
    Removed,
}

#[derive(Default)]
struct MeshProperties {
    remesh: HashSet<IVec3>,
    mesh_states: Arc<RwLock<HashMap<IVec3, Mutex<ChunkMeshState>>>>,
}

enum ChunkMeshState {
    Unmeshed,
    Ready(Option<Mesh>),
}

struct BevyProperties {
    chunk_entities: HashMap<IVec3, Entity>,
    chunk_material: Handle<StandardMaterial>,
}

fn setup_level(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut block_atlas_manager: ResMut<BlockAtlasManager>,
) {
    let level = Level {
        level_properties: LevelProperties {
            id: "debug".to_owned(),
            seed: 0,
        },
        chunk_properties: ChunkProperties::default(),
        mesh_properties: MeshProperties::default(),
        bevy_properties: BevyProperties {
            chunk_entities: Default::default(),
            chunk_material: materials.add(StandardMaterial {
                base_color_texture: Some(
                    Arc::make_mut(&mut block_atlas_manager.0)
                        .atlas_texture()
                        .expect("Block atlas not yet built"),
                ),
                base_color: Color::WHITE,
                ..default()
            }),
        },
    };
    fs::create_dir_all(format!("save/{}/chunk", level.level_properties.id))
        .expect("Failed to create save directory");
    commands.insert_resource(level);
}

fn mark_nearby_chunks_uninitialized(
    level: Res<Level>,
    game_settings: Res<GameSettings>,
    camera_query: Single<&Transform, With<Camera>>,
) {
    let Ok(mut chunk_states) = level.chunk_properties.chunk_states.try_write() else {
        return;
    };

    let camera_position = ChunkGrid::to_chunk_coordinates(camera_query.translation);
    let render_distance = IVec2::new(
        game_settings.horizontal_render_distance,
        game_settings.vertical_render_distance,
    );
    let min = camera_position - render_distance.xyx();
    let max = camera_position + render_distance.xyx();

    // In future this should be derived from the biome
    let noise = Noise::<Perlin> {
        seed: NoiseRng(level.level_properties.seed),
        frequency: 1. / chunk::SIZE_F32,
        ..Default::default()
    };

    let task_pool = AsyncComputeTaskPool::get();
    for x in min.x..max.x {
        for y in min.y..max.y {
            for z in min.z..max.z {
                let position = IVec3::new(x, y, z);

                if chunk_states.contains_key(&position)
                    || level.chunk_properties.chunk_grid.0.contains_key(&position)
                    || level.chunk_properties.removed.contains(&position)
                {
                    continue;
                }

                chunk_states.insert(position, Mutex::new(ChunkGenerationState::Uninitialized));
                task_pool
                    .spawn(create_chunk(
                        level.chunk_properties.chunk_states.clone(),
                        level.level_properties.id.clone(),
                        position,
                        noise,
                    ))
                    .detach();
            }
        }
    }
}

async fn create_chunk(
    chunk_states: Arc<RwLock<HashMap<IVec3, Mutex<ChunkGenerationState>>>>,
    file_path: String,
    position: IVec3,
    noise: impl SampleableFor<Vec2, f32>,
) {
    let chunk = 'load: {
        let path = format!(
            "save/{}/chunk/{}_{}_{}.json",
            file_path, position.x, position.y, position.z
        );
        if let Ok(serialized_chunk) = fs::read_to_string(path) {
            let mut deserializer = serde_json::Deserializer::from_str(&serialized_chunk);
            match Chunk::deserialize(&mut deserializer) {
                Ok(mut deserialized_chunk) => {
                    deserialized_chunk.position = position;
                    break 'load deserialized_chunk;
                }
                Err(error) => {
                    eprintln!("Failed to deserialize chunk at {position}: {error:?}")
                }
            }
        }

        Chunk::generate(position, &noise)
    };

    let chunk_states = chunk_states.read().expect("Chunk states rw poisoned");
    let Some(state_mutex) = chunk_states.get(&position) else {
        return;
    };
    let mut state = state_mutex.lock().expect("Chunk state mutex poisoned");
    if !matches!(*state, ChunkGenerationState::Uninitialized) {
        return;
    }
    *state = ChunkGenerationState::Ready(Some(chunk));
}

fn finalize_chunk_generation(mut level: ResMut<Level>) {
    let finished_chunks = {
        let Ok(mut chunk_states) = level.chunk_properties.chunk_states.try_write() else {
            return;
        };
        let finished_chunks = chunk_states
            .iter()
            .filter_map(|(position, state)| {
                let Ok(mut state) = state.try_lock() else {
                    return None;
                };
                let ChunkGenerationState::Ready(chunk) = state.deref_mut() else {
                    return None;
                };
                Some((*position, chunk.take()?))
            })
            .collect::<Vec<(IVec3, Chunk)>>();
        for (position, _) in finished_chunks.iter() {
            chunk_states.remove(position);
        }
        finished_chunks
    };
    for (position, chunk) in finished_chunks {
        if level.chunk_properties.removed.contains(&position) {
            continue;
        }
        level
            .chunk_properties
            .chunk_grid
            .0
            .insert(position, Arc::new(chunk));
        level.mesh_properties.remesh.insert(position);
    }
}

fn handle_remesh_queue(mut level: ResMut<Level>, block_manager: Res<BlockAtlasManager>) {
    // Arc clone needed so that remesh_queue can be drained while write lock is in scope
    let mesh_states = level.mesh_properties.mesh_states.clone();
    let Ok(mut mesh_states) = mesh_states.try_write() else {
        return;
    };

    let mesh_states_lock = level.mesh_properties.mesh_states.clone();
    let task_pool = AsyncComputeTaskPool::get();
    for position in level.mesh_properties.remesh.drain().collect::<Vec<IVec3>>() {
        mesh_states.insert(position, Mutex::new(ChunkMeshState::Unmeshed));
        let Some(chunk) = level.chunk_properties.chunk_grid.0.get(&position) else {
            continue;
        };
        task_pool
            .spawn(remesh_chunk(
                mesh_states_lock.clone(),
                Arc::downgrade(chunk),
                Arc::downgrade(&block_manager.0),
                position,
            ))
            .detach();
    }
}

async fn remesh_chunk(
    mesh_states: Arc<RwLock<HashMap<IVec3, Mutex<ChunkMeshState>>>>,
    chunk: Weak<Chunk>,
    atlas_manager: Weak<AtlasManager>,
    position: IVec3,
) {
    let Some(mesh) = chunk::mesh::build_mesh(chunk, atlas_manager) else {
        return;
    };

    let mesh_states = mesh_states.read().expect("Mesh states rw poisoned");
    let Some(state_mutex) = mesh_states.get(&position) else {
        return;
    };
    let mut state = state_mutex.lock().expect("Mesh state mutex poisoned");
    if !matches!(*state, ChunkMeshState::Unmeshed) {
        return;
    }
    *state = ChunkMeshState::Ready(mesh);
}

fn apply_ready_meshes(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let finished_meshes = {
        let Ok(mut mesh_states) = level.mesh_properties.mesh_states.try_write() else {
            return;
        };
        let finished_meshes = mesh_states
            .iter()
            .filter_map(|(position, state)| {
                let Ok(mut state) = state.try_lock() else {
                    return None;
                };
                let ChunkMeshState::Ready(mesh) = state.deref_mut() else {
                    return None;
                };
                Some((*position, mesh.take()))
            })
            .collect::<Vec<(IVec3, Option<Mesh>)>>();
        for (position, _) in finished_meshes.iter() {
            mesh_states.remove(position);
        }
        finished_meshes
    };
    let removed_meshes = finished_meshes
        .iter()
        .map(|(position, _)| *position)
        .collect::<Vec<IVec3>>();
    for (position, mesh) in finished_meshes {
        if let Some(entity) = level.bevy_properties.chunk_entities.get(&position) {
            let mut entity = commands.entity(*entity);
            match mesh {
                Some(mesh) => entity.insert(Mesh3d(meshes.add(mesh))),
                None => entity.remove::<Mesh3d>(),
            };
            continue;
        }

        let mut entity = commands.spawn((
            MeshMaterial3d(level.bevy_properties.chunk_material.clone()),
            Transform::from_xyz(
                position.x as f32 * chunk::SIZE_F32,
                position.y as f32 * chunk::SIZE_F32,
                position.z as f32 * chunk::SIZE_F32,
            ),
        ));
        if let Some(mesh) = mesh {
            entity.insert(Mesh3d(meshes.add(mesh)));
        };
        level
            .bevy_properties
            .chunk_entities
            .insert(position, entity.id());
    }
    let mesh_states = level.mesh_properties.mesh_states.clone();
    AsyncComputeTaskPool::get()
        .spawn(async move {
            let mut mesh_states = mesh_states.write().expect("Mesh states rw poisoned");
            for position in removed_meshes {
                mesh_states.remove(&position);
            }
        })
        .detach();
}

fn remove_far_chunks(
    mut level: ResMut<Level>,
    game_settings: Res<GameSettings>,
    camera_query: Single<&Transform, With<Camera>>,
) {
    let camera_position = ChunkGrid::to_chunk_coordinates(camera_query.translation);
    let render_distance = IVec2::new(
        game_settings.horizontal_render_distance,
        game_settings.vertical_render_distance,
    );
    let far_chunks = level
        .chunk_properties
        .chunk_grid
        .0
        .extract_if(|position, _| {
            let diff = (position - camera_position).abs();
            diff.x > render_distance.x || diff.y > render_distance.y || diff.z > render_distance.x
        })
        .collect::<Vec<(IVec3, Arc<Chunk>)>>();

    let task_pool = IoTaskPool::get();
    for (position, chunk) in far_chunks {
        if level.chunk_properties.removed.contains(&position) {
            continue;
        }

        let chunk = match Arc::try_unwrap(chunk) {
            Ok(chunk) => chunk,
            Err(chunk) => {
                // Safe because we just removed this key from the map
                unsafe {
                    level
                        .chunk_properties
                        .chunk_grid
                        .0
                        .insert_unique_unchecked(position, chunk);
                }
                continue;
            }
        };

        level.chunk_properties.removed.insert(position);
        level.mesh_properties.remesh.remove(&position);

        task_pool
            .spawn(save_chunk(
                level.chunk_properties.chunk_states.clone(),
                level.mesh_properties.mesh_states.clone(),
                level.level_properties.id.clone(),
                chunk,
            ))
            .detach();
    }
}

async fn save_chunk(
    chunk_states: Arc<RwLock<HashMap<IVec3, Mutex<ChunkGenerationState>>>>,
    mesh_states: Arc<RwLock<HashMap<IVec3, Mutex<ChunkMeshState>>>>,
    file_path: String,
    chunk: Chunk,
) {
    mesh_states
        .write()
        .expect("Mesh states rw poisoned")
        .remove(&chunk.position);
    chunk_states
        .write()
        .expect("Chunk states rw poisoned")
        .insert(chunk.position, Mutex::new(ChunkGenerationState::Removed));

    match serde_json::to_string(&chunk) {
        Ok(serialized_chunk) => {
            fs::write(
                format!(
                    "save/{}/chunk/{}_{}_{}.json",
                    file_path, chunk.position.x, chunk.position.y, chunk.position.z
                ),
                serialized_chunk,
            )
            .expect("Failed to write chunk");
        }
        Err(error) => eprintln!("Failed to serialize chunk at {}: {error:?}", chunk.position),
    }
}

fn cleanup_saved_chunks(mut commands: Commands, mut level: ResMut<Level>) {
    let removed_chunks = {
        let Ok(mut chunk_states) = level.chunk_properties.chunk_states.try_write() else {
            return;
        };

        chunk_states
            .extract_if(|_, state| {
                let Ok(state) = state.try_lock() else {
                    return false;
                };
                matches!(*state, ChunkGenerationState::Removed)
            })
            .collect::<Vec<(IVec3, Mutex<ChunkGenerationState>)>>()
    };

    for (position, _) in removed_chunks {
        level.chunk_properties.removed.remove(&position);
        if let Some(entity) = level.bevy_properties.chunk_entities.remove(&position) {
            commands.entity(entity).despawn();
        }
    }
}
