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
        blocks::BlockManager,
        chunk::{BlockGrid, CHUNK_BLOCK_COUNT, CHUNK_SIZE_I32, Chunk, ChunkGrid},
    };

    pub fn rebuild_chunk_meshes(
        chunk_grid: &ChunkGrid,
        block_manager: &BlockManager,
    ) -> HashMap<IVec3, Mesh> {
        let mut meshes = HashMap::new();
        for chunk in chunk_grid.chunks.values() {
            let Some(mesh) = rebuild_chunk_mesh(chunk_grid, block_manager, chunk) else {
                continue;
            };
            meshes.insert(chunk.position, mesh);
        }
        meshes
    }

    /// Will return [None] if there are no blocks to render
    pub fn rebuild_chunk_mesh(
        chunk_grid: &ChunkGrid,
        block_manager: &BlockManager,
        chunk: &Chunk,
    ) -> Option<Mesh> {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut uv_0 = Vec::new();
        let mut indices_offset = 0;

        for index in 0..CHUNK_BLOCK_COUNT {
            let Some(position) = BlockGrid::to_block_coordinates_from_index(index) else {
                continue;
            };
            let Some(block) = chunk.contents.get(position) else {
                continue;
            };

            let atlas_rect = block_manager.atlas_location_or_error(&block.0);

            let block_position =
                IVec3::new(position.x as i32, position.y as i32, position.z as i32)
                    + chunk.position * CHUNK_SIZE_I32;
            let (x, y, z) = (position.x as f32, position.y as f32, position.z as f32);

            // TOP FACE
            if chunk_grid.get_block(block_position + IVec3::Y).is_none() {
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
                    indices_offset,
                    indices_offset + 3,
                    indices_offset + 1,
                    indices_offset + 1,
                    indices_offset + 3,
                    indices_offset + 2,
                ]);
                uv_0.extend_from_slice(&[
                    [atlas_rect.min.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.max.y],
                    [atlas_rect.min.x, atlas_rect.max.y],
                ]);

                indices_offset += 4;
            }
            // BOTTOM FACE
            if chunk_grid.get_block(block_position - IVec3::Y).is_none() {
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
                    indices_offset,
                    indices_offset + 1,
                    indices_offset + 3,
                    indices_offset + 1,
                    indices_offset + 2,
                    indices_offset + 3,
                ]);
                uv_0.extend_from_slice(&[
                    [atlas_rect.min.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.max.y],
                    [atlas_rect.min.x, atlas_rect.max.y],
                ]);
                indices_offset += 4;
            }
            // RIGHT FACE
            if chunk_grid.get_block(block_position + IVec3::X).is_none() {
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
                    indices_offset,
                    indices_offset + 3,
                    indices_offset + 1,
                    indices_offset + 1,
                    indices_offset + 3,
                    indices_offset + 2,
                ]);
                uv_0.extend_from_slice(&[
                    [atlas_rect.min.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.max.y],
                    [atlas_rect.min.x, atlas_rect.max.y],
                ]);
                indices_offset += 4;
            }
            // LEFT FACE
            if chunk_grid.get_block(block_position - IVec3::X).is_none() {
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
                    indices_offset,
                    indices_offset + 1,
                    indices_offset + 3,
                    indices_offset + 1,
                    indices_offset + 2,
                    indices_offset + 3,
                ]);
                uv_0.extend_from_slice(&[
                    [atlas_rect.min.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.max.y],
                    [atlas_rect.min.x, atlas_rect.max.y],
                ]);
                indices_offset += 4;
            }
            // BACK FACE
            if chunk_grid.get_block(block_position + IVec3::Z).is_none() {
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
                    indices_offset,
                    indices_offset + 3,
                    indices_offset + 1,
                    indices_offset + 1,
                    indices_offset + 3,
                    indices_offset + 2,
                ]);
                uv_0.extend_from_slice(&[
                    [atlas_rect.min.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.max.y],
                    [atlas_rect.min.x, atlas_rect.max.y],
                ]);
                indices_offset += 4;
            }
            // FRONT FACE
            if chunk_grid.get_block(block_position - IVec3::Z).is_none() {
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
                    indices_offset,
                    indices_offset + 1,
                    indices_offset + 3,
                    indices_offset + 1,
                    indices_offset + 2,
                    indices_offset + 3,
                ]);
                uv_0.extend_from_slice(&[
                    [atlas_rect.min.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.min.y],
                    [atlas_rect.max.x, atlas_rect.max.y],
                    [atlas_rect.min.x, atlas_rect.max.y],
                ]);
                indices_offset += 4;
            }
        }

        if indices.is_empty() {
            return None;
        }

        Some(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv_0)
            .with_inserted_indices(Indices::U32(indices)),
        )
    }
}

mod chunk_file {}

mod world_generation {}
