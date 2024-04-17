/// The absorption effect
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(packed)]
pub struct Absorption {
    /// This effect goes away on the tick with the value `end_tick`,
    pub end_tick: i64,
    /// The amount of health that is allocated to the absorption effect
    pub bonus_health: f32,
}

impl Default for Absorption {
    fn default() -> Self {
        Self {
            end_tick: 0,
            bonus_health: 0.0,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Regeneration {
    /// This effect goes away on the tick with the value `end_tick`.
    pub end_tick: i64,
}

impl Default for Regeneration {
    fn default() -> Self {
        Self { end_tick: 0 }
    }
}
