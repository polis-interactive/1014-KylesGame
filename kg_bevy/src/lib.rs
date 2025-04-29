
mod kg_core;
pub use kg_core::{
    dying_enter, dying_exit, dying_update_death_throes,
    home_update_wait_for_player_interaction,
    in_game_exit_core,
    running_update_enemy_position, running_update_player_position, running_update_try_spawn_enemies,
    startup_core,
    GameState, InGameState, InputEvent
};

cfg_if::cfg_if! {
    if #[cfg(any(feature = "web", feature = "std"))] {
        mod kg_std;
        pub use kg_std::{
            on_add_enemy,
            startup_std,
            update_handle_keyboard, update_handle_resize,
            update_render_camera, update_render_tiles
        };
    }
}
    
