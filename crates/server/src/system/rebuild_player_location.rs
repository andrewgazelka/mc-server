use bvh::{Bvh, TrivialHeuristic};
use evenio::{
    entity::EntityId,
    event::Receiver,
    fetch::{Fetcher, Single},
    query::{Query, With},
};

use crate::{
    singleton::player_location_lookup::{LookupData, PlayerLocationLookup},
    FullEntityPose, Gametick, Player,
};

#[derive(Query, Debug)]
pub(crate) struct EntityQuery<'a> {
    id: EntityId,
    pose: &'a FullEntityPose,
    _player: With<&'static Player>,
}

pub fn rebuild_player_location(
    _: Receiver<Gametick>,
    lookup: Single<&mut PlayerLocationLookup>,
    entities: Fetcher<EntityQuery>,
) {
    let mut elements: Vec<_> = entities
        .iter()
        .map(|query| LookupData {
            id: query.id,
            aabb: query.pose.bounding.into(),
        })
        .collect();

    let lookup = lookup.0;
    
    let bvh = Bvh::build::<TrivialHeuristic>(&mut elements);

    lookup.inner = bvh;
}
