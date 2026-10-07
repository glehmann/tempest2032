//! Level-entry, level-exit, Flipper-capture, and glider-death animation sequences.

use crate::web::WORLD_UNIT;

pub(crate) mod flipper_capture;
pub(crate) mod glider_death;
pub(crate) mod level_entry;
pub(crate) mod level_exit;

// The original viewpoint stops at -400, measured from webz-80 at the rim.
const RETREAT_END: f32 = (110.0 - 80.0 + 400.0) * WORLD_UNIT;
