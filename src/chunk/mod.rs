use std::sync::{Arc, RwLock};

use bevy::{
    math::{I16Vec3, IVec3, Vec2, Vec3},
    platform::collections::HashMap,
    prelude::{Deref, DerefMut},
};
use noiz::SampleableFor;
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use serde_with::serde_as;

use crate::{DEFAULT_NAMESPACE, Identifier, block::Block};

pub mod mesh;

pub const SIZE_I16: i16 = 32;
pub const Z_INDEX_I16: i16 = SIZE_I16 * SIZE_I16;

pub const SIZE_I32: i32 = 32;

pub const SIZE_F32: f32 = 32.;

pub const SIZE_USIZE: usize = 32;
pub const Z_INDEX_USIZE: usize = SIZE_USIZE * SIZE_USIZE;
pub const CONTENTS_SIZE: usize = SIZE_USIZE * SIZE_USIZE * SIZE_USIZE;

#[derive(Default)]
pub struct ChunkGrid(pub HashMap<IVec3, Arc<RwLock<Chunk>>>);

impl ChunkGrid {
    pub fn to_chunk_coord(raw_coordinate: f32) -> i32 {
        (raw_coordinate / SIZE_F32).floor() as i32
    }

    pub fn to_chunk_coordinates(raw_coordinates: Vec3) -> IVec3 {
        IVec3::new(
            Self::to_chunk_coord(raw_coordinates.x),
            Self::to_chunk_coord(raw_coordinates.y),
            Self::to_chunk_coord(raw_coordinates.z),
        )
    }

    /// This will block the current thread due to a call to RwLock::write()<br>
    /// Using this function is not recommended unless you are <b>ONLY</b> setting one block
    pub fn set_block(&self, block_coordinates: IVec3, block: Option<Block>) -> Option<()> {
        self.0
            .get(&Self::to_chunk_coordinates(block_coordinates.as_vec3()))?
            .write()
            .expect("Chunk rw poisoned")
            .contents[Chunk::to_index(Chunk::to_block_coordinates(block_coordinates))] = block;
        Some(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Chunk {
    #[serde(skip)]
    pub position: IVec3,
    pub contents: SerializableChunkContents,
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        Self {
            position,
            contents: SerializableChunkContents::default(),
        }
    }

    // In future may be moved somewhere else and may potentially be split into multiple functions
    pub fn generate(position: IVec3, noise: &impl SampleableFor<Vec2, f32>) -> Self {
        let mut chunk = Self::new(position);

        for x in 0..SIZE_I32 {
            let raw_x = position.x * SIZE_I32 + x;
            for z in 0..SIZE_I32 {
                let raw_z = position.z * SIZE_I32 + z;
                let sample: f32 = noise.sample(Vec2::new(raw_x as f32, raw_z as f32));
                let height = (sample * 10.) as i32 + 2;
                if height < position.y * SIZE_I32 {
                    continue;
                }

                chunk.set_area(
                    I16Vec3::new(x as i16, 0, z as i16),
                    I16Vec3::new(
                        x as i16,
                        (height + position.y.abs() * SIZE_I32).min(SIZE_I32 - 1) as i16,
                        z as i16,
                    ),
                    &Block::new(Identifier::new(DEFAULT_NAMESPACE, "stone")),
                );
            }
        }

        chunk
    }

    pub fn to_block_coord(raw_coordinate: i32) -> i16 {
        let block_coord = raw_coordinate % SIZE_I32;
        if block_coord >= 0 {
            return block_coord as i16;
        }
        (block_coord + SIZE_I32) as i16
    }

    pub fn to_block_coordinates(raw_coordinates: IVec3) -> I16Vec3 {
        I16Vec3::new(
            Self::to_block_coord(raw_coordinates.x),
            Self::to_block_coord(raw_coordinates.y),
            Self::to_block_coord(raw_coordinates.z),
        )
    }

    pub fn to_block_coordinates_from_index(index: usize) -> Option<I16Vec3> {
        let i16_index = i16::try_from(index).ok()?;
        Some(I16Vec3::new(
            i16_index % SIZE_I16,
            i16_index / SIZE_I16 % SIZE_I16,
            i16_index / Z_INDEX_I16,
        ))
    }

    pub fn to_index(position: I16Vec3) -> usize {
        (position.x + position.y * SIZE_I16 + position.z * Z_INDEX_I16) as usize
    }

    pub fn set_area(&mut self, start: I16Vec3, end: I16Vec3, block: &Block) {
        for x in start.x..=end.x {
            for y in start.y..=end.y {
                let index_xy = x + y * SIZE_I16;
                for z in start.z..=end.z {
                    self.contents[(index_xy + z * Z_INDEX_I16) as usize] = Some(block.clone());
                }
            }
        }
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deref, DerefMut)]
pub struct SerializableChunkContents(
    #[serde_as(as = "Box<[Option<_>; CONTENTS_SIZE]>")] Box<[Option<Block>; CONTENTS_SIZE]>,
);

impl Default for SerializableChunkContents {
    fn default() -> Self {
        Self(Box::new([const { None }; CONTENTS_SIZE]))
    }
}

impl<'de> Deserialize<'de> for SerializableChunkContents {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct BlockVisitor;

        impl<'de> Visitor<'de> for BlockVisitor {
            type Value = SerializableChunkContents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(&format!("array of size {}", { CONTENTS_SIZE }))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut blocks = SerializableChunkContents::default();
                for i in 0..CONTENTS_SIZE {
                    let Some(block) = seq.next_element()? else {
                        break;
                    };
                    blocks[i] = block;
                }

                Ok(blocks)
            }
        }

        deserializer.deserialize_seq(BlockVisitor)
    }
}
