//todo: better chunk rendering
//  - textures (stitch atlases at runtime)
//  - vertex colors
//  - models
//todo: chunk saving
//  - when out of render distance
//todo: chunk loading
//  - when in render distance or when forced
//todo: world generation

pub mod camera_movement {
    use std::f32::consts::FRAC_PI_2;

    use bevy::{
        input::{ButtonInput, keyboard::KeyCode, mouse::AccumulatedMouseMotion},
        math::{EulerRot, Quat, Vec2},
        prelude::*,
        time::Time,
        transform::components::Transform,
    };

    #[derive(Component)]
    #[require(Camera3d)]
    pub struct MovableCamera {
        pub speed: f32,
        pub sensitivity: f32,
    }

    pub struct CameraMovementPlugin;

    impl Plugin for CameraMovementPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Update, control_camera);
        }
    }

    fn axis(a: bool, b: bool) -> f32 {
        if a && b {
            return 0.;
        }
        if a {
            return 1.;
        }
        if b {
            return -1.;
        }
        0.
    }

    fn control_camera(
        time: Res<Time>,
        mouse_motion: Res<AccumulatedMouseMotion>,
        keyboard_input: Res<ButtonInput<KeyCode>>,
        camera_query: Single<(&mut Transform, &MovableCamera)>,
    ) {
        let (mut transform, movable_camera) = camera_query.into_inner();

        let forward = transform.forward().normalize();
        let left = transform.left().normalize();
        let up = transform.up().normalize();
        transform.translation += (forward
            * axis(
                keyboard_input.pressed(KeyCode::KeyW),
                keyboard_input.pressed(KeyCode::KeyS),
            )
            + left
                * axis(
                    keyboard_input.pressed(KeyCode::KeyA),
                    keyboard_input.pressed(KeyCode::KeyD),
                )
            + up * axis(
                keyboard_input.pressed(KeyCode::Space),
                keyboard_input.pressed(KeyCode::ShiftLeft),
            ))
            * movable_camera.speed
            * time.delta_secs();

        if mouse_motion.delta == Vec2::ZERO {
            return;
        }
        let (mut yaw, mut pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        yaw += -mouse_motion.delta.x * movable_camera.sensitivity;

        const PITCH_MAX: f32 = FRAC_PI_2 - 0.01;
        pitch = (pitch - mouse_motion.delta.y * movable_camera.sensitivity)
            .clamp(-PITCH_MAX, PITCH_MAX);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

pub mod chunk_mesh {
    use bevy::{
        asset::RenderAssetUsages,
        math::IVec3,
        platform::collections::HashMap,
        render::mesh::{Indices, Mesh, PrimitiveTopology},
    };

    use crate::{
        chunk::{Block, BlockGrid, CHUNK_BLOCK_COUNT, ChunkGrid},
        textures::BlockTextureAtlas,
    };

    fn is_block_air(block: &Block) -> bool {
        block.0 == 0
    }

    //TODO: tidy up this code
    pub fn create_chunk_meshes(
        chunk_grid: &ChunkGrid,
        block_atlas: &BlockTextureAtlas,
    ) -> HashMap<IVec3, Mesh> {
        let mut meshes = HashMap::new();
        for chunk in chunk_grid.chunks.values() {
            let mut positions = Vec::new();
            let mut normals = Vec::new();
            let mut indices = Vec::new();
            let mut uv_0 = Vec::new();

            let mut indicies_offset = 0;
            for index in 0..CHUNK_BLOCK_COUNT {
                let Some(position) = BlockGrid::to_block_coordinates_from_index(index) else {
                    continue;
                };

                if chunk.contents.get(position).is_none_or(is_block_air) {
                    continue;
                }

                let atlas_rect = block_atlas
                    .layout
                    .textures
                    .get(chunk.contents.get(position).unwrap().0 as usize)
                    .unwrap_or_else(|| block_atlas.layout.textures.first().unwrap());

                let block_position =
                    IVec3::new(position.x as i32, position.y as i32, position.z as i32);

                // replace usage of these with the position var definied above
                let (x, y, z) = (
                    block_position.x as f32,
                    block_position.y as f32,
                    block_position.z as f32,
                );

                // TOP FACE
                if chunk_grid
                    .get_block(block_position + IVec3::Y)
                    .is_none_or(is_block_air)
                {
                    positions.extend_from_slice(&[
                        [x + -0.5, y + 0.5, z + -0.5],
                        [x + 0.5, y + 0.5, z + -0.5],
                        [x + 0.5, y + 0.5, z + 0.5],
                        [x + -0.5, y + 0.5, z + 0.5],
                    ]);
                    normals.extend_from_slice(&[
                        [0.0, 1.0, 0.0],
                        [0.0, 1.0, 0.0],
                        [0.0, 1.0, 0.0],
                        [0.0, 1.0, 0.0],
                    ]);
                    indices.extend_from_slice(&[
                        indicies_offset,
                        indicies_offset + 3,
                        indicies_offset + 1,
                        indicies_offset + 1,
                        indicies_offset + 3,
                        indicies_offset + 2,
                    ]);
                    uv_0.extend_from_slice(&[
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                    ]);

                    indicies_offset += 4;
                }
                // BOTTOM FACE
                if chunk_grid
                    .get_block(block_position - IVec3::Y)
                    .is_none_or(is_block_air)
                {
                    positions.extend_from_slice(&[
                        [x + -0.5, y + -0.5, z + -0.5],
                        [x + 0.5, y + -0.5, z + -0.5],
                        [x + 0.5, y + -0.5, z + 0.5],
                        [x + -0.5, y + -0.5, z + 0.5],
                    ]);
                    normals.extend_from_slice(&[
                        [0.0, -1.0, 0.0],
                        [0.0, -1.0, 0.0],
                        [0.0, -1.0, 0.0],
                        [0.0, -1.0, 0.0],
                    ]);
                    indices.extend_from_slice(&[
                        indicies_offset,
                        indicies_offset + 1,
                        indicies_offset + 3,
                        indicies_offset + 1,
                        indicies_offset + 2,
                        indicies_offset + 3,
                    ]);
                    uv_0.extend_from_slice(&[
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                    ]);
                    indicies_offset += 4;
                }
                // RIGHT FACE
                if chunk_grid
                    .get_block(block_position + IVec3::X)
                    .is_none_or(is_block_air)
                {
                    positions.extend_from_slice(&[
                        [x + 0.5, y + -0.5, z + -0.5],
                        [x + 0.5, y + -0.5, z + 0.5],
                        [x + 0.5, y + 0.5, z + 0.5],
                        [x + 0.5, y + 0.5, z + -0.5],
                    ]);
                    normals.extend_from_slice(&[
                        [1.0, 0.0, 0.0],
                        [1.0, 0.0, 0.0],
                        [1.0, 0.0, 0.0],
                        [1.0, 0.0, 0.0],
                    ]);
                    indices.extend_from_slice(&[
                        indicies_offset,
                        indicies_offset + 3,
                        indicies_offset + 1,
                        indicies_offset + 1,
                        indicies_offset + 3,
                        indicies_offset + 2,
                    ]);
                    uv_0.extend_from_slice(&[
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                    ]);
                    indicies_offset += 4;
                }
                // LEFT FACE
                if chunk_grid
                    .get_block(block_position - IVec3::X)
                    .is_none_or(is_block_air)
                {
                    positions.extend_from_slice(&[
                        [x + -0.5, y + -0.5, z + -0.5],
                        [x + -0.5, y + -0.5, z + 0.5],
                        [x + -0.5, y + 0.5, z + 0.5],
                        [x + -0.5, y + 0.5, z + -0.5],
                    ]);
                    normals.extend_from_slice(&[
                        [-1.0, 0.0, 0.0],
                        [-1.0, 0.0, 0.0],
                        [-1.0, 0.0, 0.0],
                        [-1.0, 0.0, 0.0],
                    ]);
                    indices.extend_from_slice(&[
                        indicies_offset,
                        indicies_offset + 1,
                        indicies_offset + 3,
                        indicies_offset + 1,
                        indicies_offset + 2,
                        indicies_offset + 3,
                    ]);
                    uv_0.extend_from_slice(&[
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                    ]);
                    indicies_offset += 4;
                }
                // BACK FACE
                if chunk_grid
                    .get_block(block_position + IVec3::Z)
                    .is_none_or(is_block_air)
                {
                    positions.extend_from_slice(&[
                        [x + -0.5, y + -0.5, z + 0.5],
                        [x + -0.5, y + 0.5, z + 0.5],
                        [x + 0.5, y + 0.5, z + 0.5],
                        [x + 0.5, y + -0.5, z + 0.5],
                    ]);
                    normals.extend_from_slice(&[
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                    ]);
                    indices.extend_from_slice(&[
                        indicies_offset,
                        indicies_offset + 3,
                        indicies_offset + 1,
                        indicies_offset + 1,
                        indicies_offset + 3,
                        indicies_offset + 2,
                    ]);
                    uv_0.extend_from_slice(&[
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                    ]);
                    indicies_offset += 4;
                }
                // FRONT FACE
                if chunk_grid
                    .get_block(block_position - IVec3::Z)
                    .is_none_or(is_block_air)
                {
                    positions.extend_from_slice(&[
                        [x + -0.5, y + -0.5, z + -0.5],
                        [x + -0.5, y + 0.5, z + -0.5],
                        [x + 0.5, y + 0.5, z + -0.5],
                        [x + 0.5, y + -0.5, z + -0.5],
                    ]);
                    normals.extend_from_slice(&[
                        [0.0, 0.0, -1.0],
                        [0.0, 0.0, -1.0],
                        [0.0, 0.0, -1.0],
                        [0.0, 0.0, -1.0],
                    ]);
                    indices.extend_from_slice(&[
                        indicies_offset,
                        indicies_offset + 1,
                        indicies_offset + 3,
                        indicies_offset + 1,
                        indicies_offset + 2,
                        indicies_offset + 3,
                    ]);
                    uv_0.extend_from_slice(&[
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.min.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.max.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                        [
                            (atlas_rect.min.x as f32) / (block_atlas.layout.size.x as f32),
                            (atlas_rect.max.y as f32) / (block_atlas.layout.size.y as f32),
                        ],
                    ]);
                    indicies_offset += 4;
                }
            }

            if indices.is_empty() {
                continue;
            }

            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv_0)
            .with_inserted_indices(Indices::U32(indices));

            meshes.insert(chunk.position, mesh);
        }
        meshes
    }
}

mod chunk_file {}

mod world_generation {}
