use evenio::{
    entity::EntityId,
    event::Receiver,
    fetch::{Fetcher, Single},
    rayon::prelude::*,
};
use tracing::instrument;

use crate::{
    bounding_box::{CollisionContext, EntityBoundingBoxes},
    EntityReaction, FullEntityPose, Gametick,
};

#[instrument(skip_all, name = "entity_detect_collisions")]
pub fn entity_detect_collisions(
    _: Receiver<Gametick>,
    entity_bounding_boxes: Single<&EntityBoundingBoxes>,
    poses_fetcher: Fetcher<(&FullEntityPose, &EntityReaction)>,
) {
    let entity_bounding_boxes = entity_bounding_boxes.0;

    entity_bounding_boxes.get_all_collisions(|a, b| {
        let (pose, reaction) = poses_fetcher.get(a).unwrap();
        let (other_pose, other_reaction) = poses_fetcher.get(b).unwrap();

        pose.apply_entity_collision(other_pose, unsafe { &mut *reaction.0.get() });
        other_pose.apply_entity_collision(pose, unsafe { &mut *other_reaction.0.get() });
    });
}
