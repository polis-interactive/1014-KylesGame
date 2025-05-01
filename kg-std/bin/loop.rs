use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(
                ScheduleRunnerPlugin::run_loop(
                    Duration::from_secs_f64(1.0 / 30.0)
                )
            )
        )
        .add_systems(Update, hello_world_system)
        .run()
    ;
}

const EWRAM_END: usize = 0x0204_0000;
const IWRAM_END: usize = 0x0300_8000;

fn hello_world_system() {
    
    println!("{:?}, {:?}", IWRAM_END, EWRAM_END);
}