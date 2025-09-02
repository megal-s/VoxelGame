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
    math::IVec3,
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
use noiz::{Noise, prelude::common_noise::Perlin, rng::NoiseRng};

use crate::{
    blocks::BlockManagerResource,
    chunk::{CHUNK_SIZE_F32, ChunkGrid},
    game::chunk_mesh::rebuild_chunk_mesh,
};

pub struct ChunkLoaderPlugin;

impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        //Add resource here, then add systems to interact with it
        app.add_systems(OnEnter(crate::GameState::InGame), setup_level)
            .add_systems(
                Update,
                (
                    generate_nearby_chunks,
                    generate_chunk_meshes,
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
    loaded_chunks: Arc<Mutex<HashSet<IVec3>>>,
    chunk_grid: Arc<Mutex<ChunkGrid>>,
    chunk_entities: HashMap<IVec3, Entity>,
    chunk_queue: Arc<Mutex<Vec<IVec3>>>,
    chunk_mesh_queue: Arc<Mutex<Vec<(IVec3, Mesh)>>>,
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
            loaded_chunks: Default::default(),
            chunk_grid: Default::default(),
            chunk_entities: Default::default(),
            chunk_queue: Default::default(),
            chunk_mesh_queue: Default::default(),
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
    camera_query: Single<&Transform, With<Camera>>, //in future change this to be the player
) {
    const GENERATION_DISTANCE: i32 = 2;
    let camera_position = ChunkGrid::to_chunk_coordinates(camera_query.into_inner().translation);
    let noise = level.noise;
    let loaded_chunks = level.loaded_chunks.clone();
    let chunk_queue = level.chunk_queue.clone();
    let chunk_grid = level.chunk_grid.clone();
    //AsyncComputeTaskPool::get().spawn(async move {
    for x in camera_position.x - GENERATION_DISTANCE..=camera_position.x + GENERATION_DISTANCE {
        for y in camera_position.y - GENERATION_DISTANCE..=camera_position.y + GENERATION_DISTANCE {
            for z in
                camera_position.z - GENERATION_DISTANCE..=camera_position.z + GENERATION_DISTANCE
            {
                let position = IVec3::new(x, y, z);
                if loaded_chunks.lock().expect("Loaded chunk hashset mutex poisoned").contains(&position) {
                    continue;
                }
                //let noise = noise;
                let chunk_queue = chunk_queue.clone();
                let chunk_grid = chunk_grid.clone();
                loaded_chunks.lock().expect("Loaded chunk hashset mutex poisoned").insert(position);
                AsyncComputeTaskPool::get()
                    .spawn(async move {
                        let chunk = ChunkGrid::generate_or_load_chunk(position, &noise);
                        chunk_grid.lock().expect("Chunk grid mutex was poisoned").chunks.insert(position, chunk);
                        chunk_queue
                            .lock()
                            .expect("Chunk queue mutex poisoned")
                            .push(position);
                    })
                    .detach();
            }
        }
    }
    //}).detach();
}

fn generate_chunk_meshes(level: Res<Level>, block_manager: ResMut<BlockManagerResource>) {
    let Ok(mut chunk_queue) = level.chunk_queue.try_lock() else {
        return;
    };

    while !chunk_queue.is_empty() {
        let Some(chunk_position) = chunk_queue.pop() else {
            continue;
        };

        let chunk_grid = level.chunk_grid.clone();
        let block_manager = block_manager.clone();
        let chunk_mesh_queue = level.chunk_mesh_queue.clone();
        AsyncComputeTaskPool::get()
            .spawn(async move {
                let chunk_grid = chunk_grid.lock().expect("Chunk grid mutex poisoned");
                let Some(mesh) = rebuild_chunk_mesh(
                    &chunk_grid,
                    &block_manager.lock().expect("Block manager mutex poisoned"),
                    &chunk_grid.chunks[&chunk_position],
                ) else {
                    return;
                };
                chunk_mesh_queue
                    .lock()
                    .expect("Chunk mesh queue mutex poisoned")
                    .push((chunk_position, mesh));
            })
            .detach();
    }
}

fn update_chunk_entities(
    mut commands: Commands,
    mut level: ResMut<Level>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut spawned_entities = HashMap::new();
    {
        let Ok(mut chunk_mesh_queue) = level.chunk_mesh_queue.try_lock() else {
            return;
        };

        while !chunk_mesh_queue.is_empty() {
            let Some((position, mesh)) = chunk_mesh_queue.pop() else {
                continue;
            };

            if level.chunk_entities.contains_key(&position)
                || spawned_entities.contains_key(&position)
            {
                continue;
            }

            spawned_entities.insert(
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
    level.chunk_entities.extend(spawned_entities);
}
