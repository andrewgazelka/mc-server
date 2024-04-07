use std::sync::atomic::Ordering;

use evenio::prelude::*;
use tracing::instrument;
use valence_protocol::{
    packets::play,
    text::{Color, IntoText},
};

use crate::{global::Global, singleton::player_lookup::PlayerUuidLookup, KickPlayer, Player, Uuid};

#[instrument(skip_all)]
pub fn player_kick(
    r: Receiver<KickPlayer, (EntityId, &mut Player, &Uuid)>,
    global: Single<&Global>,
    lookup: Single<&mut PlayerUuidLookup>,
    mut s: Sender<Despawn>,
) {
    let (id, player, uuid) = r.query;

    lookup.0.remove(&uuid.0);

    let reason = &r.event.reason;

    let io = &mut player.packets;

    let reason = reason.into_text().color(Color::RED);

    // if we can't send ignore
    let _ = io.writer.send_packet(&play::DisconnectS2c {
        reason: reason.into(),
    });

    // todo: also handle disconnecting without kicking, io socket being closed, etc

    global.0.shared.player_count.fetch_sub(1, Ordering::Relaxed);

    s.send(Despawn(id));
}
