use evenio::prelude::*;
use tracing::{info, instrument};

use crate::{Gametick, Player, SHARED};

#[instrument(skip_all, level = "trace")]
pub fn clean_up_io(
    _r: Receiver<Gametick>,
    mut io_entities: Fetcher<(EntityId, &mut Player)>,

    mut s: Sender<Despawn>,
) {
    for (id, player) in &mut io_entities {
        if player.packets.writer.is_closed() {
            info!("player {} disconnected", player.name);
            SHARED.player_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            s.send(Despawn(id));
        }
    }
}
