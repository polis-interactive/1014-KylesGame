
mod time;
pub use time::*;

use bevy::app::plugin_group;

plugin_group! {
    pub struct EmbassyPlugin {
        :EmbassyTimePlugin
    }
}
