use bevy::{
    math::{I16Vec3, IVec2, IVec3, Vec2, Vec3},
    platform::collections::HashMap,
    utils::default,
};
use noiz::{
    Noise, Sampleable, SampleableFor, ScalableNoise, SeedableNoise,
    cells::OrthoGrid,
    curves::Smoothstep,
    prelude::{MixCellGradients, QuickGradients, SNormToUNorm, common_noise::Perlin},
};

// Could probably be nicer
pub const CHUNK_SIZE_I16: i16 = 32; // due to i16 having a max value of 32768, this value must not exceed 32
pub const CHUNK_SIZE_I32: i32 = 32;
pub const CHUNK_SIZE_F32: f32 = 32.;
pub const CHUNK_SIZE_USIZE: usize = 32;
pub const CHUNK_BLOCK_COUNT: usize = CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE;

#[derive(Default)]
pub struct ChunkGrid {
    pub chunks: HashMap<IVec3, Chunk>,
}

impl ChunkGrid {
    pub fn to_chunk_coord(raw_coordinate: f32) -> i32 {
        (raw_coordinate / CHUNK_SIZE_F32).floor() as i32
    }

    pub fn to_chunk_coordinates(raw_coordinates: Vec3) -> IVec3 {
        IVec3::new(
            Self::to_chunk_coord(raw_coordinates.x),
            Self::to_chunk_coord(raw_coordinates.y),
            Self::to_chunk_coord(raw_coordinates.z),
        )
    }

    pub fn to_chunk_coordinates_from_ivec3(raw_coordinates: IVec3) -> IVec3 {
        IVec3::new(
            Self::to_chunk_coord(raw_coordinates.x as f32),
            Self::to_chunk_coord(raw_coordinates.y as f32),
            Self::to_chunk_coord(raw_coordinates.z as f32),
        )
    }

    pub fn get_block(&self, position: IVec3) -> Option<&Block> {
        self.chunks
            .get(&Self::to_chunk_coordinates_from_ivec3(position))?
            .contents
            .get(BlockGrid::to_block_coordinates(position))
    }

    pub fn get_block_mut(&mut self, position: IVec3) -> Option<&mut Block> {
        self.chunks
            .get_mut(&Self::to_chunk_coordinates_from_ivec3(position))?
            .contents
            .get_mut(BlockGrid::to_block_coordinates(position))
    }

    pub fn set_block(&mut self, position: IVec3, block: Block) -> Option<()> {
        self.chunks
            .get_mut(&Self::to_chunk_coordinates_from_ivec3(position))?
            .contents
            .set(BlockGrid::to_block_coordinates(position), block)
    }

    pub fn set_blocks_in_area(&mut self, start: IVec3, end: IVec3, block: Block) -> Option<()> {
        let chunk_start = Self::to_chunk_coordinates_from_ivec3(start);
        let chunk_end = Self::to_chunk_coordinates_from_ivec3(end);

        for x in chunk_start.x..=chunk_end.x {
            for y in chunk_start.y..=chunk_end.y {
                for z in chunk_start.z..=chunk_end.z {
                    let chunk = self.chunks.get_mut(&IVec3::new(x, y, z))?;

                    let start_x = if chunk_start.x < x {
                        0
                    } else {
                        BlockGrid::to_block_coord(start.x)
                    };
                    let start_y = if chunk_start.y < y {
                        0
                    } else {
                        BlockGrid::to_block_coord(start.y)
                    };
                    let start_z = if chunk_start.z < z {
                        0
                    } else {
                        BlockGrid::to_block_coord(start.z)
                    };
                    let end_x = if chunk_end.x > x {
                        CHUNK_SIZE_I16 - 1
                    } else {
                        BlockGrid::to_block_coord(end.x)
                    };
                    let end_y = if chunk_end.y > y {
                        CHUNK_SIZE_I16 - 1
                    } else {
                        BlockGrid::to_block_coord(end.y)
                    };
                    let end_z = if chunk_end.z > z {
                        CHUNK_SIZE_I16 - 1
                    } else {
                        BlockGrid::to_block_coord(end.z)
                    };

                    chunk.contents.set_area(
                        I16Vec3::new(start_x, start_y, start_z),
                        I16Vec3::new(end_x, end_y, end_z),
                        block,
                    )?;
                }
            }
        }

        Some(())
    }

    pub fn generate_or_load_chunk(&mut self, position: IVec3) {
        // check if chunk file exists and load it if it does
        //self.generate_chunk(position);
    }

    pub fn generate_chunk(&mut self, position: IVec3, noise: &impl SampleableFor<Vec2, f32>) {
        // definitely a better way of doing this, just trying to get something working for now
        let mut chunk_contents = BlockGrid::new();
        for x in 0..CHUNK_SIZE_I32 {
            let raw_x = position.x * CHUNK_SIZE_I32 + x;
            for z in 0..CHUNK_SIZE_I32 {
                let raw_z = position.z * CHUNK_SIZE_I32 + z;
                let a: f32 = noise.sample(Vec2::new(raw_x as f32, raw_z as f32));
                let height = (a * 10.) as i32;
                chunk_contents.set_area(
                    I16Vec3::new(x as i16, 0, z as i16),
                    I16Vec3::new(x as i16, height as i16, z as i16),
                    Block(1),
                );
            }
        }
        self.chunks.insert(
            position,
            Chunk {
                position,
                contents: chunk_contents,
            },
        );
    }
}

pub struct Chunk {
    pub position: IVec3,
    pub contents: BlockGrid,
}

pub struct BlockGrid {
    blocks: Box<[Block; CHUNK_BLOCK_COUNT]>,
}

impl BlockGrid {
    pub fn new() -> Self {
        Self {
            blocks: Box::new([Block(0); CHUNK_BLOCK_COUNT]),
        }
    }

    pub fn to_block_coord(raw_coordinate: i32) -> i16 {
        let block_coord = raw_coordinate % CHUNK_SIZE_I32;
        if block_coord >= 0 {
            return block_coord as i16;
        }
        (block_coord + CHUNK_SIZE_I32) as i16
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
        if i16_index < 0 {
            return None;
        }
        Some(I16Vec3::new(
            i16_index % CHUNK_SIZE_I16,
            i16_index / CHUNK_SIZE_I16 % CHUNK_SIZE_I16,
            i16_index / (CHUNK_SIZE_I16 * CHUNK_SIZE_I16),
        ))
    }

    pub fn to_index(block_coordinates: I16Vec3) -> Option<usize> {
        if block_coordinates.x < 0 || block_coordinates.x >= CHUNK_SIZE_I16 {
            return None;
        }
        if block_coordinates.y < 0 || block_coordinates.y >= CHUNK_SIZE_I16 {
            return None;
        }
        if block_coordinates.z < 0 || block_coordinates.z >= CHUNK_SIZE_I16 {
            return None;
        }
        Some(
            (block_coordinates.x
                + block_coordinates.y * CHUNK_SIZE_I16
                + block_coordinates.z * CHUNK_SIZE_I16 * CHUNK_SIZE_I16) as usize,
        )
    }

    pub fn get(&self, position: I16Vec3) -> Option<&Block> {
        self.blocks.get(Self::to_index(position)?)
    }

    pub fn get_mut(&mut self, position: I16Vec3) -> Option<&mut Block> {
        self.blocks.get_mut(Self::to_index(position)?)
    }

    pub fn set(&mut self, position: I16Vec3, block: Block) -> Option<()> {
        self.blocks[Self::to_index(position)?] = block;
        Some(())
    }

    pub fn set_area(&mut self, start: I16Vec3, end: I16Vec3, block: Block) -> Option<()> {
        Self::to_index(start)?;
        Self::to_index(end)?;
        for x in start.x..=end.x {
            for y in start.y..=end.y {
                for z in start.z..=end.z {
                    self.blocks[Self::to_index(I16Vec3::new(x, y, z)).unwrap()] = block;
                }
            }
        }
        Some(())
    }
}

#[derive(Default, Clone, Copy)]
pub struct Block(pub i32);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn raw_coord_to_chunk_coord() {
        // Individual coordinates
        assert_eq!(ChunkGrid::to_chunk_coord(0.), 0);
        assert_eq!(ChunkGrid::to_chunk_coord(CHUNK_SIZE_F32 - 1.), 0);
        assert_eq!(ChunkGrid::to_chunk_coord(CHUNK_SIZE_F32), 1);
        assert_eq!(ChunkGrid::to_chunk_coord(-1.), -1);
        assert_eq!(ChunkGrid::to_chunk_coord(-CHUNK_SIZE_F32), -1);
        assert_eq!(ChunkGrid::to_chunk_coord(-CHUNK_SIZE_F32 - 1.), -2);

        // Floating point coordinate sets
        assert_eq!(ChunkGrid::to_chunk_coordinates(Vec3::ZERO), IVec3::ZERO);
        assert_eq!(
            ChunkGrid::to_chunk_coordinates(Vec3::splat(CHUNK_SIZE_F32 - 1.)),
            IVec3::ZERO
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates(Vec3::splat(CHUNK_SIZE_F32)),
            IVec3::ONE
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates(Vec3::NEG_ONE),
            IVec3::NEG_ONE
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates(Vec3::splat(-CHUNK_SIZE_F32)),
            IVec3::NEG_ONE
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates(Vec3::splat(-CHUNK_SIZE_F32 - 1.)),
            IVec3::splat(-2)
        );

        // Integer coordinate sets
        assert_eq!(
            ChunkGrid::to_chunk_coordinates_from_ivec3(IVec3::ZERO),
            IVec3::ZERO
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates_from_ivec3(IVec3::splat(CHUNK_SIZE_I32 - 1)),
            IVec3::ZERO
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates_from_ivec3(IVec3::splat(CHUNK_SIZE_I32)),
            IVec3::ONE
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates_from_ivec3(IVec3::NEG_ONE),
            IVec3::NEG_ONE
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates_from_ivec3(IVec3::splat(-CHUNK_SIZE_I32)),
            IVec3::NEG_ONE
        );
        assert_eq!(
            ChunkGrid::to_chunk_coordinates_from_ivec3(IVec3::splat(-CHUNK_SIZE_I32 - 1)),
            IVec3::splat(-2)
        );
    }

    #[test]
    fn raw_coord_to_grid_coord() {
        // Individual coordinates
        assert_eq!(BlockGrid::to_block_coord(0), 0);
        assert_eq!(BlockGrid::to_block_coord(1), 1);
        assert_eq!(BlockGrid::to_block_coord(-1), CHUNK_SIZE_I16 - 1);
        assert_eq!(BlockGrid::to_block_coord(CHUNK_SIZE_I32), 0);
        assert_eq!(BlockGrid::to_block_coord(-CHUNK_SIZE_I32), 0);

        // Coordinate sets
        assert_eq!(BlockGrid::to_block_coordinates(IVec3::ZERO), I16Vec3::ZERO);
        assert_eq!(BlockGrid::to_block_coordinates(IVec3::ONE), I16Vec3::ONE);
        assert_eq!(
            BlockGrid::to_block_coordinates(IVec3::NEG_ONE),
            I16Vec3::splat(CHUNK_SIZE_I16 - 1)
        );
        assert_eq!(
            BlockGrid::to_block_coordinates(IVec3::splat(CHUNK_SIZE_I32)),
            I16Vec3::ZERO
        );
        assert_eq!(
            BlockGrid::to_block_coordinates(IVec3::splat(-CHUNK_SIZE_I32)),
            I16Vec3::ZERO
        );
    }

    #[test]
    fn block_coord_indexing() {
        // Block coordinates into indicies
        assert_eq!(BlockGrid::to_index(I16Vec3::ZERO), Some(0));
        assert_eq!(BlockGrid::to_index(I16Vec3::X), Some(1));
        assert_eq!(BlockGrid::to_index(I16Vec3::Y), Some(CHUNK_SIZE_USIZE));
        assert_eq!(
            BlockGrid::to_index(I16Vec3::Z),
            Some(CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE)
        );
        assert_eq!(
            BlockGrid::to_index(I16Vec3::splat(CHUNK_SIZE_I16 - 1)),
            Some(i16::MAX as usize)
        );

        assert_eq!(BlockGrid::to_index(I16Vec3::MAX), None);
        assert_eq!(BlockGrid::to_index(I16Vec3::NEG_ONE), None);

        // Indicies into block coordinates
        assert_eq!(
            BlockGrid::to_block_coordinates_from_index(0),
            Some(I16Vec3::ZERO)
        );
        assert_eq!(
            BlockGrid::to_block_coordinates_from_index(1),
            Some(I16Vec3::X)
        );
        assert_eq!(
            BlockGrid::to_block_coordinates_from_index(CHUNK_SIZE_USIZE),
            Some(I16Vec3::Y)
        );
        assert_eq!(
            BlockGrid::to_block_coordinates_from_index(CHUNK_SIZE_USIZE * CHUNK_SIZE_USIZE),
            Some(I16Vec3::Z)
        );
        assert_eq!(
            BlockGrid::to_block_coordinates_from_index(i16::MAX as usize),
            Some(I16Vec3::splat(CHUNK_SIZE_I16 - 1))
        );

        assert_eq!(
            BlockGrid::to_block_coordinates_from_index(i16::MAX as usize + 1),
            None
        );
    }
}
