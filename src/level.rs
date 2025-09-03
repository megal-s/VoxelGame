use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

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
    platform::collections::{HashMap, HashSet},
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
                    save_discarded_chunks,
                    removed_chunk_entities,
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
    pub loaded_chunks: HashSet<IVec3>, // public so that the debug text can read it
    discarded_chunks: Arc<Mutex<Vec<IVec3>>>,
    remesh_queue: Arc<Mutex<VecDeque<IVec3>>>,
    complete_meshes: Arc<Mutex<Vec<(IVec3, Mesh)>>>,
    saved_chunks: Arc<Mutex<Vec<(IVec3, Entity)>>>,
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
            loaded_chunks: Default::default(),
            discarded_chunks: Default::default(),
            remesh_queue: Default::default(),
            complete_meshes: Default::default(),
            saved_chunks: Default::default(),
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
    mut level: ResMut<Level>,
    settings: Res<GameSettings>,
    camera_query: Single<&Transform, With<Camera>>,
) {
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
                if level.loaded_chunks.contains(&position) {
                    continue;
                }
                level.loaded_chunks.insert(position);

                let chunk_grid = level.chunk_grid.clone();
                let remesh_queue = level.remesh_queue.clone();
                let noise = level.noise;
                AsyncComputeTaskPool::get()
                    .spawn(generate_chunk(chunk_grid, remesh_queue, position, noise))
                    .detach();
            }
        }
    }
}

async fn generate_chunk(
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    remesh_queue: Arc<Mutex<VecDeque<IVec3>>>,
    position: IVec3,
    noise: impl SampleableFor<Vec2, f32>,
) {
    let chunk = ChunkGrid::generate_or_load_chunk(position, &noise);
    chunk_grid
        .lock()
        .expect("Chunk grid mutex was poisoned")
        .chunks
        .insert(position, chunk);
    remesh_queue
        .lock()
        .expect("Remesh queue mutex was poisoned")
        .push_back(position);
}

fn build_chunk_meshes(level: Res<Level>, block_manager_resource: Res<BlockManagerResource>) {
    let Ok(mut remesh_queue) = level.remesh_queue.try_lock() else {
        return;
    };

    while !remesh_queue.is_empty() {
        let Some(position) = remesh_queue.pop_front() else {
            break;
        };

        let chunk_grid = level.chunk_grid.clone();
        let block_manager = (*block_manager_resource).clone();
        let complete_meshes = level.complete_meshes.clone();
        AsyncComputeTaskPool::get()
            .spawn(build_chunk_mesh(
                chunk_grid,
                block_manager,
                complete_meshes,
                position,
            ))
            .detach();
    }
}

async fn build_chunk_mesh(
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    block_manager: Arc<Mutex<BlockManager>>,
    complete_meshes: Arc<Mutex<Vec<(IVec3, Mesh)>>>,
    position: IVec3,
) {
    let chunk_grid = chunk_grid.lock().expect("Chunk grid mutex poisoned");
    let Some(chunk) = chunk_grid.chunks.get(&position) else {
        return;
    };
    let Some(mesh) = meshing::rebuild_chunk_mesh(
        &chunk_grid,
        &block_manager.lock().expect("Block manager mutex poisoned"),
        chunk,
    ) else {
        return;
    };
    complete_meshes
        .lock()
        .expect("Completed meshes mutex poisoned")
        .push((position, mesh));
}

fn update_chunk_entities(
    mut commands: Commands,
    level: Res<Level>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(mut complete_meshes) = level.complete_meshes.try_lock() else {
        return;
    };

    let Ok(mut chunk_entities) = level.chunk_entities.try_lock() else {
        return;
    };

    while !complete_meshes.is_empty() {
        let Some((position, mesh)) = complete_meshes.pop() else {
            break;
        };

        if let Some(entity) = chunk_entities.get(&position) {
            commands.entity(*entity).insert(Mesh3d(meshes.add(mesh)));
            continue;
        }

        chunk_entities.insert(
            position,
            commands
                .spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(level.block_material.clone()),
                    Transform::from_xyz(
                        position.x as f32 * CHUNK_SIZE_F32,
                        position.y as f32 * CHUNK_SIZE_F32,
                        position.z as f32 * CHUNK_SIZE_F32,
                    ),
                ))
                .id(),
        );
    }
}

fn discard_far_chunks(
    level: Res<Level>,
    settings: Res<GameSettings>,
    camera_query: Single<&Transform, With<Camera>>,
) {
    let Ok(chunk_grid) = level.chunk_grid.try_lock() else {
        return;
    };
    let Ok(mut discarded_chunks) = level.discarded_chunks.try_lock() else {
        return;
    };

    let camera_position = ChunkGrid::to_chunk_coordinates(camera_query.into_inner().translation);
    let far_chunks = chunk_grid
        .chunks
        .keys()
        .filter_map(|position| {
            let diff = (position - camera_position).abs();
            if (diff.x <= settings.horizontal_render_distance
                && diff.y <= settings.vertical_render_distance
                && diff.z <= settings.horizontal_render_distance)
                || discarded_chunks.contains(position)
            {
                return None;
            }
            Some(*position)
        })
        .collect::<Vec<IVec3>>();
    discarded_chunks.extend(far_chunks);
}

fn save_discarded_chunks(level: Res<Level>) {
    let Ok(mut discarded_chunks) = level.discarded_chunks.try_lock() else {
        return;
    };

    while !discarded_chunks.is_empty() {
        let Some(position) = discarded_chunks.pop() else {
            break;
        };

        let chunk_grid = level.chunk_grid.clone();
        let chunk_entities = level.chunk_entities.clone();
        let saved_chunks = level.saved_chunks.clone();
        AsyncComputeTaskPool::get()
            .spawn(save_chunk(
                chunk_grid,
                chunk_entities,
                saved_chunks,
                position,
            ))
            .detach();
    }
}

async fn save_chunk(
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    chunk_entities: Arc<Mutex<HashMap<IVec3, Entity>>>,
    saved_chunks: Arc<Mutex<Vec<(IVec3, Entity)>>>,
    position: IVec3,
) {
    let entity = chunk_entities
        .lock()
        .expect("Chunk entities mutex poisoned")
        .remove(&position);
    chunk_grid
        .lock()
        .expect("Chunk grid mutex poisoned")
        .chunks
        .remove(&position);
    if entity.is_none() {
        return;
    }
    saved_chunks
        .lock()
        .expect("Saved chunks mutex poisoned")
        .push((position, entity.unwrap()));
}

fn removed_chunk_entities(mut level: ResMut<Level>, mut commands: Commands) {
    let mut removed_chunks = Vec::new();
    {
        let Ok(mut saving_chunks) = level.saved_chunks.try_lock() else {
            return;
        };

        while !saving_chunks.is_empty() {
            let Some((position, entity)) = saving_chunks.pop() else {
                break;
            };

            removed_chunks.push(position);
            commands.entity(entity).despawn();
        }
    }
    for position in removed_chunks {
        level.loaded_chunks.remove(&position);
    }
}
