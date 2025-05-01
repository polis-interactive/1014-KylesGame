
#![no_std]

extern crate alloc;
use bevy::prelude::*;
use kg_core::CoreSet;

mod embassy_plugin;
pub use embassy_plugin::EmbassyPlugin;

mod thumbstick;
use thumbstick::{ThumbstickChannel, update_proxy_thumbstick};
pub use thumbstick::{Thumbstick, thumbstick_task};

mod lights;
pub use lights::LED_COUNT;

pub struct KgEmbeddedPlugin;

impl Plugin for KgEmbeddedPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .insert_resource(ThumbstickChannel::new())
            .add_systems(Update, update_proxy_thumbstick.before(CoreSet))
        ;
    }
}