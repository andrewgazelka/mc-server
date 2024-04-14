use evenio::{
    component::Component,
    entity::EntityId,
    event::{Event, Receiver},
    fetch::Single,
    query::Query,
};

use crate::{
    global::Global,
    packets::voicechat::{Codec, Msg},
};

#[derive(Event)]
struct InitVoiceChat {
    #[event(target)]
    player: EntityId,
}

#[derive(Component)]
struct VoiceChatGlobal {
    port: u16,
    mtu_size: usize,
    voice_chat_distance: f64,
    keep_alive: u16,

    /// todo: what is this
    groups_enabled: bool,

    voice_host: String,
    allow_recording: bool,
}

#[derive(Query)]
pub struct PlayerQuery<'a> {
    id: EntityId,
    player: &'a mut crate::Player,
    encoder: &'a mut crate::Encoder,
    uuid: &'a crate::Uuid,
}

pub fn voice_chat(r: Receiver<InitVoiceChat, PlayerQuery>, global: Single<&VoiceChatGlobal>) {
    let PlayerQuery {
        id,
        player,
        encoder,
        uuid,
    } = r.query;

    let uuid = uuid.0;
    let secret = uuid::Uuid::new_v4();

    let pkt = crate::packets::voicechat::SecretVoiceChatS2c {
        secret,
        server_port: global.port as i32,
        player_uuid: uuid,
        codec: Codec::VoIp,
        mtu_size: global.mtu_size as i32,
        voice_chat_distance: global.voice_chat_distance,
        keep_alive: global.keep_alive as i32,
        groups_enabled: false,
        voice_host: &global.voice_host,
        allow_recording: global.allow_recording,
    }
    .to_plugin_message();
    
    encoder.append_packet(&pkt).unwrap();
}
