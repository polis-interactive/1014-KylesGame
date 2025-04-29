
use bevy::{
    dev_tools::states::log_transitions,
    prelude::*
};
use bevy_rand::{
    plugin::EntropyPlugin,
    prelude::WyRand
};
use kg_bevy::{
    dying_enter, dying_exit, dying_update_death_throes,
    home_update_wait_for_player_interaction,
    in_game_exit_core,
    running_update_enemy_position, running_update_player_position, running_update_try_spawn_enemies,
    on_add_enemy,
    startup_std,
    update_handle_keyboard, update_handle_resize, update_render_camera, update_render_tiles,
    startup_core,
    GameState, InGameState, InputEvent
};


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_sub_state::<InGameState>()
        .add_event::<InputEvent>()
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .add_systems(Startup, (
            startup_core,
            startup_std.after(startup_core)
        ))
        .add_systems(OnEnter(InGameState::Dying), dying_enter)
        .add_systems(Update, (
            update_handle_resize,
            update_handle_keyboard,
            (
                home_update_wait_for_player_interaction
            ).run_if(in_state(GameState::Home)),
            (
                running_update_try_spawn_enemies,
                running_update_enemy_position,
                running_update_player_position
            ).run_if(in_state(InGameState::Running)),
            (
                dying_update_death_throes
            ).run_if(in_state(InGameState::Dying)),
            update_render_tiles,
            update_render_camera
        ))
        .add_systems(OnExit(InGameState::Dying), dying_exit)
        .add_systems(OnExit(GameState::InGame), in_game_exit_core)
        .add_systems(Update, (log_transitions::<GameState>, log_transitions::<InGameState>))
        .add_observer(on_add_enemy)
        .run();
}
