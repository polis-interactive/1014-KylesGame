use bevy::{
    platform::time::Instant,
    prelude::*,
};

use core::time::Duration as CoreDuration;
use embassy_time::Instant as EmbassyInstant;

#[derive(Default)]
pub struct EmbassyTimePlugin;

fn elapsed_time() -> CoreDuration  {
    CoreDuration::from_micros(EmbassyInstant::now().as_micros())
}

impl Plugin for EmbassyTimePlugin {
    fn build(&self, _: &mut App) {
        unsafe {
            Instant::set_elapsed(elapsed_time);
        }
    }

}