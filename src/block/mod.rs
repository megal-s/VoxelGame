use std::sync::Arc;

use bevy::{
    asset::Handle,
    ecs::resource::Resource,
    image::Image,
    math::{BVec3, IVec3, Vec3, Vec3Swizzles},
};
use bevy_asset_loader::asset_collection::AssetCollection;
use serde::{Deserialize, Serialize};

use crate::{Identifier, atlas::AtlasManager};

#[derive(AssetCollection, Resource)]
pub struct BlockAssets {
    #[asset(path = "Error.png")]
    pub error: Handle<Image>,
    #[asset(path = "Stone.png")]
    pub stone: Handle<Image>,
    #[asset(path = "Dirt.png")]
    pub dirt: Handle<Image>,
}

#[derive(Default, Resource)]
pub struct BlockAtlasManager(pub Arc<AtlasManager>);

#[derive(Clone, Serialize, Deserialize)]
pub struct Block {
    pub identifier: Identifier,
}

impl Block {
    pub fn new(identifier: Identifier) -> Self {
        Self { identifier }
    }
}

pub struct BlockRaycast {
    origin: Vec3,
    direction: Vec3,
    step: IVec3,
    next_position: IVec3,
    bound_difference: Vec3,
    next_bounds: Vec3,
    bounds_mask: BVec3,
}

impl BlockRaycast {
    pub fn from_origin_to_target(origin: Vec3, target: Vec3) -> Self {
        Self::from_origin_in_direction(origin, (target - origin).normalize_or_zero())
    }

    pub fn from_origin_in_direction(origin: Vec3, mut direction: Vec3) -> Self {
        let block_origin = origin.floor();

        if direction.x == 0. {
            direction.x = 0.00001;
        }
        if direction.y == 0. {
            direction.y = 0.00001;
        }
        if direction.z == 0. {
            direction.z = 0.00001;
        }
        direction = direction.normalize();

        let step = direction.signum();
        let bounds_difference = (Vec3::ONE / direction).abs();
        let next_bounds = (step * (block_origin - origin) + (step * 0.5) + 0.5) * bounds_difference;

        Self {
            origin,
            direction,
            step: step.as_ivec3(),
            next_position: block_origin.as_ivec3(),
            bound_difference: bounds_difference,
            next_bounds,
            bounds_mask: BVec3::FALSE,
        }
    }

    pub fn step(&mut self) {
        self.bounds_mask = self
            .next_bounds
            .cmple(self.next_bounds.yzx().min(self.next_bounds.zxy()));
        self.next_position += self.step * IVec3::from(self.bounds_mask);
        self.next_bounds += self.bound_difference * Vec3::from(self.bounds_mask);
    }

    pub fn query_distance(&self) -> f32 {
        ((self.next_bounds - self.bound_difference) * Vec3::from(self.bounds_mask)).element_sum()
    }

    pub fn query_position(&self) -> Vec3 {
        self.origin + (self.direction * self.query_distance())
    }

    pub fn query_normal(&self) -> IVec3 {
        -self.step * IVec3::from(self.bounds_mask)
    }
}
