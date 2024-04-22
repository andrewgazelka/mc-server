use std::{alloc::Allocator, fmt::Debug};

use bumpalo::Bump;
use evenio::{entity::EntityId, event::Event};
use glam::Vec3;
use valence_protocol::Hand;

use crate::{
    components::FullEntityPose,
    net::{Server, MAX_PACKET_SIZE},
};

/// Initialize a Minecraft entity (like a zombie) with a given pose.
#[derive(Event)]
pub struct InitEntity {
    /// The pose of the entity.
    pub pose: FullEntityPose,
}

#[derive(Event)]
pub struct PlayerInit {
    pub entity: EntityId,

    /// The name of the player i.e., `Emerald_Explorer`.
    pub username: Box<str>,
    pub uuid: uuid::Uuid,
    pub pose: FullEntityPose,
}

/// Sent whenever a player joins the server.
#[derive(Event)]
pub struct PlayerJoinWorld {
    /// The [`EntityId`] of the player.
    #[event(target)]
    pub target: EntityId,
}

/// An event that is sent whenever a player is kicked from the server.
#[derive(Event)]
pub struct KickPlayer {
    /// The [`EntityId`] of the player.
    #[event(target)] // Works on tuple struct fields as well.
    pub target: EntityId,
    /// The reason the player was kicked.
    pub reason: String,
}

/// An event that is sent whenever a player swings an arm.
#[derive(Event)]
pub struct SwingArm {
    /// The [`EntityId`] of the player.
    #[event(target)]
    pub target: EntityId,
    /// The hand the player is swinging.
    pub hand: Hand,
}

#[derive(Event)]
pub struct AttackEntity {
    /// The [`EntityId`] of the player.
    #[event(target)]
    pub target: EntityId,
    /// The location of the player that is hitting.
    pub from_pos: Vec3,
}

/// An event to kill all minecraft entities (like zombies, skeletons, etc). This will be sent to the equivalent of
/// `/killall` in the game.
#[derive(Event)]
pub struct KillAllEntities;

/// An event when server stats are updated.
#[derive(Event)]
pub struct StatsEvent<'a, 'b> {
    /// The number of milliseconds per tick in the last second.
    pub ms_per_tick_mean_1s: f64,
    /// The number of milliseconds per tick in the last 5 seconds.
    pub ms_per_tick_mean_5s: f64,

    pub scratch: &'b mut BumpScratch<'a>,
}

// todo: REMOVE
#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "this will be removed in the future"
)]
unsafe impl<'a, 'b> Send for StatsEvent<'a, 'b> {}
unsafe impl<'a, 'b> Sync for StatsEvent<'a, 'b> {}

// todo: naming? this seems bad
#[derive(Debug)]
pub struct Scratch<A: Allocator = std::alloc::Global> {
    inner: Vec<u8, A>,
}

impl Scratch {
    #[must_use]
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl Default for Scratch {
    fn default() -> Self {
        Self::new()
    }
}

/// Nice for getting a buffer that can be used for intermediate work
///
/// Guarantees:
/// - every single time [`ScratchBuffer::obtain`] is called, the buffer will be cleared before returning
/// - the buffer has capacity of at least `MAX_PACKET_SIZE`
pub trait ScratchBuffer: sealed::Sealed + Debug {
    type Allocator: Allocator;
    fn obtain(&mut self) -> &mut Vec<u8, Self::Allocator>;
}

mod sealed {
    pub trait Sealed {}
}

impl<A: Allocator + Debug> sealed::Sealed for Scratch<A> {}

impl<A: Allocator + Debug> ScratchBuffer for Scratch<A> {
    type Allocator = A;

    fn obtain(&mut self) -> &mut Vec<u8, Self::Allocator> {
        &mut self.inner
    }
}

pub type BumpScratch<'a> = Scratch<&'a Bump>;

impl<'a> From<&'a Bump> for BumpScratch<'a> {
    fn from(bump: &'a Bump) -> Self {
        Self {
            inner: Vec::with_capacity_in(MAX_PACKET_SIZE, bump),
        }
    }
}

impl<'a> BumpScratch<'a> {
    pub fn obtain(&mut self) -> &mut Vec<u8, &'a Bump> {
        // clear scratch before we get
        self.inner.clear();
        &mut self.inner
    }
}

// todo: why need two life times?
#[derive(Event)]
pub struct Gametick<'a, 'b> {
    pub bump: &'a Bump,
    pub scratch: &'b mut BumpScratch<'a>,
}

// todo: REMOVE
#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "this will be removed in the future"
)]
unsafe impl<'a, 'b> Send for Gametick<'a, 'b> {}
unsafe impl<'a, 'b> Sync for Gametick<'a, 'b> {}

/// An event that is sent when it is time to send packets to clients.
#[derive(Event)]
pub struct Egress<'a> {
    pub server: &'a mut Server,
}
