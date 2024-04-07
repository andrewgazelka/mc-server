use evenio::prelude::*;
use tracing::instrument;
use valence_protocol::text::IntoText;

use crate::{
    singleton::encoder::{Encoder, PacketMetadata},
    StatsEvent,
};

#[instrument(skip_all, level = "trace")]
pub fn tps_message(r: Receiver<StatsEvent>, encoder: Single<&mut Encoder>) {
    let StatsEvent {
        ms_per_tick_mean_1s,
        ms_per_tick_mean_5s,
        cpus,
    } = r.event;


    let mut message = format!("ms {ms_per_tick_mean_1s:05.2} {ms_per_tick_mean_5s:05.2}");

    for cpu in cpus.iter() {
        // format xx.xx%
        message.push_str(&format!(" {cpu:04.1}"));
    }

    let packet = valence_protocol::packets::play::OverlayMessageS2c {
        action_bar_text: message.into_cow_text(),
    };

    let encoder = encoder.0;

    encoder
        .append_round_robin(&packet, PacketMetadata::DROPPABLE)
        .unwrap();
}
