use std::borrow::Cow;

use evenio::prelude::*;
use tracing::instrument;
use valence_protocol::{packets::play::{
    scoreboard_display_s2c::ScoreboardPosition,
    scoreboard_objective_update_s2c::{ObjectiveMode, ObjectiveRenderType},
}, text::IntoText, Text, VarInt};
use valence_protocol::packets::play::BundleSplitterS2c;
use valence_protocol::packets::play::scoreboard_player_update_s2c::ScoreboardPlayerUpdateAction;

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
    // let value: Cow<str> = Cow::Borrowed("yo");

    let encoder = encoder.0;
    
    let text = " - Score: ".into_text()
        + Text::score(
            "*",
            "skibidi",
            None,
            // Some(value),
        );

    let text2 = " - Score with custom value: ".into_text()
        + Text::score("*", "skibidi", Some("value".into()));


    let packet = valence_protocol::packets::play::ScoreboardObjectiveUpdateS2c {
        objective_name: "skibidi",
        mode: ObjectiveMode::Create {
            objective_display_name: "yooooo".into_cow_text(),
            render_type: ObjectiveRenderType::Integer,
        },
    };
    encoder.append_round_robin(&packet, PacketMetadata::DROPPABLE).unwrap();

    let packet = valence_protocol::packets::play::ScoreboardDisplayS2c {
        position: ScoreboardPosition::Sidebar,
        score_name: "skibidi",
    };
    encoder.append_round_robin(&packet, PacketMetadata::DROPPABLE).unwrap();
    
    // let score = fastrand::i32(1..100);

    encoder.append_round_robin(&BundleSplitterS2c, PacketMetadata::DROPPABLE).unwrap();
    for (idx, cpu) in cpus.iter().enumerate() {
        let num = idx + 1;
        
        let percent = format!("{cpu:.0}");
        
        
        
        let packet = valence_protocol::packets::play::ScoreboardPlayerUpdateS2c {
            entity_name: &percent,
            action: ScoreboardPlayerUpdateAction::Update {
                objective_name: "skibidi",
                objective_score: VarInt(num as i32),
            }
        };
        encoder.append_round_robin(&packet, PacketMetadata::DROPPABLE).unwrap();
    }
    encoder.append_round_robin(&BundleSplitterS2c, PacketMetadata::DROPPABLE).unwrap();
    
}
