use std::sync::Arc;

use bevy::{asset::Handle, ecs::resource::Resource, image::Image, math::Vec3};
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

pub struct BlockRay {
    pub position: Vec3,
    step: Vec3,
    delta: Vec3,
    bound: Vec3,
    pub normal: Vec3,
}

impl BlockRay {
    pub fn from_origin_to_target(origin: Vec3, target: Vec3) -> Self {
        Self::from_origin_in_direction(origin, (target - origin).normalize_or_zero())
    }

    pub fn from_origin_in_direction(origin: Vec3, mut direction: Vec3) -> Self {
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
        let delta = step / direction;

        let floored_origin = origin.floor();
        let bound = Vec3::new(
            Self::max(origin.x, floored_origin.x, step.x, direction.x),
            Self::max(origin.y, floored_origin.y, step.y, direction.y),
            Self::max(origin.z, floored_origin.z, step.z, direction.z),
        );

        Self {
            position: floored_origin,
            step,
            delta,
            bound,
            normal: Vec3::ZERO,
        }
    }

    fn max(x: f32, fx: f32, s: f32, d: f32) -> f32 {
        //(if d > 0. {x.ceil()-x} else {x-x.floor()}) / d.abs()
        ((fx + (if s > 0. { 1. } else { 0. })) - x) / d
    }

    pub fn step(&mut self) {
        if self.bound.x < self.bound.y && self.bound.x < self.bound.z {
            self.position.x += self.step.x;
            self.bound.x += self.delta.x;
            self.normal = Vec3::X * -self.step;
            return;
        }
        if self.bound.y < self.bound.z {
            self.position.y += self.step.y;
            self.bound.y += self.delta.y;
            self.normal = Vec3::Y * -self.step;
            return;
        }
        self.position.z += self.step.z;
        self.bound.z += self.delta.z;
        self.normal = Vec3::Z * -self.step;
    }
}
