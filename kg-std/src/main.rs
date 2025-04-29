
use bevy::prelude::*;
use kg_core::{
    GameState, InGameState, KgCorePlugin
};
use kg_std::KgStdPlugin;

fn main() {
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins)
        .add_plugins(KgCorePlugin)
        .add_plugins(KgStdPlugin);

    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            use bevy::dev_tools::states::log_transitions;
            app.add_systems(Update, (log_transitions::<GameState>, log_transitions::<InGameState>));
        }
    }

    app.run();
}
