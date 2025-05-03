
#![no_std]

extern crate alloc;
use bevy::{app::PluginGroupBuilder, prelude::*};
use kg_core::GenericRand;

mod embassy_plugin;
pub use embassy_plugin::EmbassyPlugin;

use embassy_rp::clocks::RoscRng;
use lights::KgLightsPlugin;
use rand::SeedableRng;
use rand::rngs::SmallRng;

mod thumbstick;
use thumbstick::KgThumbstickPlugin;
pub use thumbstick::{Thumbstick, thumbstick_task};

mod lights;
pub use lights::{LED_COUNT, lighting_task};

#[derive(Default)]
struct KgRandPlugin;

impl Plugin for KgRandPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(GenericRand (SmallRng::from_rng(RoscRng).unwrap()))
        ;
    }
}

pub struct KgEmbeddedPlugin;

impl PluginGroup for KgEmbeddedPlugin {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(KgRandPlugin)
            .add(KgThumbstickPlugin)
            .add(KgLightsPlugin)
    }
}