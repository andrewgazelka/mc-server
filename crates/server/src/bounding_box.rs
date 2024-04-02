use std::iter::Zip;

use bvh_arena::volumes::Aabb;
use evenio::{component::Component, entity::EntityId, fetch::Fetcher};
use fnv::FnvHashMap;
use smallvec::SmallVec;
use valence_protocol::math::{IVec2, Vec2, Vec3};

use crate::{EntityReaction, FullEntityPose};

type Storage = SmallVec<EntityId, 4>;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
struct Index2D {
    x: i32,
    z: i32,
}

#[derive(Component, Default)]
pub struct EntityBoundingBoxes {
    query: bvh_arena::Bvh<EntityId, Aabb<2>>,
}

#[derive(Copy, Clone, Debug)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    #[must_use]
    pub fn create(feet: Vec3, width: f32, height: f32) -> Self {
        let half_width = width / 2.0;

        let min = Vec3::new(feet.x - half_width, feet.y, feet.z - half_width);
        let max = Vec3::new(feet.x + half_width, feet.y + height, feet.z + half_width);

        Self { min, max }
    }

    fn collides(&self, other: Self) -> bool {
        let self_min = self.min.as_ref();
        let self_max = self.max.as_ref();

        let other_min = other.min.as_ref();
        let other_max = other.max.as_ref();

        // SIMD vectorized

        let mut collide = 0b1_u8;

        for i in 0..3 {
            collide &= u8::from(self_min[i] <= other_max[i]);
            collide &= u8::from(self_max[i] >= other_min[i]);
        }

        collide == 1
    }

    #[must_use]
    pub fn move_by(&self, offset: Vec3) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }
}

const fn idx(location: IVec2) -> Index2D {
    Index2D {
        x: location.x,
        z: location.y,
    }
}

const EMPTY_STORAGE: Storage = SmallVec::new();

pub struct Collisions {
    pub ids: SmallVec<EntityId, 4>,
    pub poses: SmallVec<FullEntityPose, 4>,
}

impl IntoIterator for Collisions {
    type IntoIter = Zip<smallvec::IntoIter<EntityId, 4>, smallvec::IntoIter<FullEntityPose, 4>>;
    type Item = (EntityId, FullEntityPose);

    fn into_iter(self) -> Self::IntoIter {
        self.ids.into_iter().zip(self.poses)
    }
}

pub struct CollisionContext {
    pub bounding: BoundingBox,
    pub id: EntityId,
}

impl EntityBoundingBoxes {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "https://github.com/andrewgazelka/hyperion/issues/73"
    )]
    pub fn insert(&mut self, bounding_box: BoundingBox, id: EntityId) {
        let min = [bounding_box.min.x, bounding_box.min.z];
        let max = [bounding_box.max.x, bounding_box.max.z];

        let aabb = Aabb::from_min_max(min, max);

        self.query.insert(id, aabb);
    }

    // todo: is there a better way to do this
    pub fn clear(&mut self) {
        self.query.clear();
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "https://github.com/andrewgazelka/hyperion/issues/74"
    )]
    pub fn get_all_collisions(&self, mut on_overlaping_pair: impl FnMut(EntityId, EntityId)) {
        self.query.for_each_overlaping_pair(|a, b| {
            on_overlaping_pair(*a, *b);
        });

        // let bounding = current.bounding;

        // let min2d = Vec2::new(bounding.min.x, bounding.min.z);
        // let max2d = Vec2::new(bounding.max.x, bounding.max.z);
        //
        // let start_x = min2d.x.floor() as i32;
        // let start_z = min2d.y.floor() as i32;
        //
        // let end_x = max2d.x.ceil() as i32;
        // let end_z = max2d.y.ceil() as i32;
        //
        // let mut collisions_ids = SmallVec::<EntityId, 4>::new();
        // let mut collisions_poses = SmallVec::<FullEntityPose, 4>::new();
        //
        // for x in start_x..=end_x {
        //     for z in start_z..=end_z {
        //         let coord = IVec2::new(x, z);
        //
        //         let Some(storage) = self.get(coord) else {
        //             continue;
        //         };
        //
        //         for &id in storage {
        //             if id == current.id {
        //                 continue;
        //             }
        //
        //             let Ok((_, other_pose, _)) = fetcher.get(id) else {
        //                 // the entity is probably expired / has been removed
        //                 continue;
        //             };
        //
        //             // todo: see which way ordering this has the most performance
        //             if bounding.collides(other_pose.bounding) && !collisions_ids.contains(&id) {
        //                 collisions_ids.push(id);
        //                 collisions_poses.push(*other_pose);
        //             }
        //         }
        //     }
        // }
        //
        // Collisions {
        //     ids: collisions_ids,
        //     poses: collisions_poses,
        // }
    }
}

// https://www.youtube.com/watch?v=3s7h2MHQtxc
// order 1 hilbert is 2x2   (2^1 x 2^1)
// order 2 hilbert is 4x4   (2^2 x 2^2)
// order 3 hilbert is 8x8   (2^3 x 2^3)
// ...
// order 10 hilbert is 1024x1024   (2^10 x 2^10)

// 1024x1024
