#![no_std]

mod kg_core;
pub use kg_core::{
    CoreSet, DirectionType, EntityColor, InputEvent, Position, GenericRand, ShowDeathThroes, BOARD_SIZE,
    KgCorePlugin, GameState, InGameState
};
