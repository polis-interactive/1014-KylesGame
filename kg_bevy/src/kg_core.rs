
use core::time::Duration;
use core::ops::Not;

use bevy::{
    color::palettes::css::WHITE,
    prelude::*
};
use bevy_rand::{global::GlobalEntropy, prelude::WyRand};
use rand::Rng;

cfg_if::cfg_if! {
    if #[cfg(feature = "web")] {
        use web_time::Instant;
    } else if #[cfg(feature = "std")] {
        use std::time::Instant;
    } else {
        use embassy_time::Instant;
    }
}



/* CONST */

pub const BOARD_SIZE: u8 = 7;
pub const PLAYER_MOVE_SPEED: Duration = Duration::from_millis(75);
pub const ENEMY_MOVE_SPEED: Duration = Duration::from_millis(250);
pub const DEATH_THROWS_SPEED: Duration = Duration::from_millis(250);
pub const DEATH_THROWS_COUNT: usize = 5;
pub const MAX_ENEMIES: usize = 1;

/* STATES */

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum GameState {
    #[default]
    Home,
    InGame,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(GameState = GameState::InGame)]
#[states(scoped_entities)]
pub enum InGameState {
    #[default]
    Running,
    Dying,
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub enum DirectionType {
    #[default]
    Up,
    UpLeft,
    Left,
    DownLeft,
    Down,
    DownRight,
    Right,
    UpRight
}

impl Not for DirectionType {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            DirectionType::Up => DirectionType::Down,
            DirectionType::UpLeft => DirectionType::DownRight,
            DirectionType::Left => DirectionType::Right,
            DirectionType::DownLeft => DirectionType::UpRight,
            DirectionType::Down => DirectionType::Up,
            DirectionType::DownRight => DirectionType::UpLeft,
            DirectionType::Right => DirectionType::Left,
            DirectionType::UpRight => DirectionType::DownLeft,
        }
    }
}

impl DirectionType {
    pub fn is_complex(&self) -> bool {
        match self {
            Self::Up | Self::Left | Self::Down | Self::Right => false,
            _ => true
        }
    }

    // Favors vertical movement?
    pub fn choose_flopped_direction(&self, other: &Self) -> Self {
        match (self, other) {
            (DirectionType::UpLeft, DirectionType::Up) | (DirectionType::DownLeft, DirectionType::Down) => DirectionType::Left,
            (DirectionType::UpLeft, _) => DirectionType::Up,
            (DirectionType::DownLeft, _) => DirectionType::Down,
            (DirectionType::UpRight, DirectionType::Up) | (DirectionType::DownRight, DirectionType::Down) => DirectionType::Right,
            (DirectionType::UpRight, _) => DirectionType::Up,
            _ => DirectionType::Down
        }
    }

    pub fn choose_simple_random(rng: &mut impl Rng) -> Self {
        match rng.random_range(0..4) {
            0 => DirectionType::Up,
            1 => DirectionType::Left,
            2 => DirectionType::Down,
            _ => DirectionType::Right
        }
    }

    pub fn choose_new_random(rng: &mut impl Rng, last: &Self) -> Self {
        let choice = Self::choose_simple_random(rng);
        if &choice != last {
            choice
        } else {
            Self::choose_simple_random(rng)
        }
    }
}


/* EVENTS */


#[derive(Event)]
pub struct InputEvent(pub DirectionType);


/* COMPONENTS */

#[derive(Component, Default, Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

impl Position {
    pub fn new (x: u8, y: u8) -> Self {
        Position { x, y }
    }

    pub fn new_square(pos: u8) -> Self {
        let mut position = Self::default();
        position.update_square(pos);
        position
    }

    pub fn update_square(&mut self, position: u8) {
        self.x = position;
        self.y = position;
    }

    pub fn can_move(&self, direction: &DirectionType) -> bool {
        match direction {
            DirectionType::Up => self.y < BOARD_SIZE - 1,
            DirectionType::Left => self.x > 0,
            DirectionType::Down => self.y > 0,
            _ => self.x < BOARD_SIZE - 1,
        }
    }

    pub fn update_with_direction(&mut self, direction: DirectionType) {
        match direction {
            DirectionType::Up => {
                self.y = (self.y + 1).min(BOARD_SIZE - 1);
            }
            DirectionType::UpLeft => {
                self.x = self.x.saturating_sub(1);
                self.y = (self.y + 1).min(BOARD_SIZE - 1);
            }
            DirectionType::Left => {
                self.x = self.x.saturating_sub(1);
            }
            DirectionType::DownLeft => {
                self.x = self.x.saturating_sub(1);
                self.y = self.y.saturating_sub(1);
            }
            DirectionType::Down => {
                self.y = self.y.saturating_sub(1);
            },
            DirectionType::DownRight => {
                self.x = (self.x + 1).min(BOARD_SIZE - 1);
                self.y = self.y.saturating_sub(1);
            },
            DirectionType::Right => {
                self.x = (self.x + 1).min(BOARD_SIZE - 1);
            },
            DirectionType::UpRight => {
                self.x = (self.x + 1).min(BOARD_SIZE - 1);
                self.y = (self.y + 1).min(BOARD_SIZE - 1);
            },
        }
    }

    pub fn eq(&self, other: &Self) -> bool {
        self == other
    }

    pub fn random_enemy_position(direction: DirectionType, rng: &mut impl Rng) -> Self {
        let random_cell = rng.random_range(0..BOARD_SIZE);
        match direction {
            DirectionType::Up => Position::new(random_cell, 0),
            DirectionType::Left => Position::new(BOARD_SIZE, random_cell ),
            DirectionType::Down => Position::new(random_cell, BOARD_SIZE ),
            _ => Position::new(0, random_cell ),
        }
    }
}

#[derive(Component, Default, Clone, Debug)]
pub struct Direction(pub DirectionType);

impl Direction {
    pub fn new(direction: DirectionType) -> Self {
        Direction(direction)
    }
    pub fn choose_next_player_direction(&mut self, direction: DirectionType) {
        if !direction.is_complex() {
            self.0 = direction;
            return;
        }
        self.0 = direction.choose_flopped_direction(&self.0);
    }
}


#[derive(Component, Default, Clone)]
pub enum EntityColor {
    Hue(u8),
    #[default]
    White
}

impl EntityColor {
    pub fn to_color(&self) -> Color {
        match self {
            EntityColor::Hue(h) => {
                Hsva::hsv((*h as f32) / 256.0 * 360.0, 1.0, 1.0).into()
            },
            EntityColor::White => WHITE.into(),
        }
    }

    pub fn random_enemy_hue(rng: &mut impl Rng) -> Self {
        let mut choice: u8 = rng.random();
        if choice.abs_diff(Player::default_hue()) < 5 {
            choice = choice.wrapping_add(128);
        }
        EntityColor::Hue(choice)
    }
}

#[derive(Clone)]
pub struct Timer {
    instant: Instant,
    duration: Duration
}

impl Default for Timer {
    fn default() -> Self {
        Timer::new(Duration::default())
    }
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            instant: Instant::now(),
            duration
        }
    }

    pub fn can_update(&self) -> bool {
        Instant::now() - self.instant > self.duration
    }

    pub fn reset(&mut self) {
        self.instant = Instant::now();
    }
}

#[derive(Component, Clone, Default)]
pub struct EntityTimer(Timer);

impl EntityTimer {
    pub fn new(duration: Duration) -> Self {
        EntityTimer(Timer::new(duration))
    }
    pub fn can_update(&self) -> bool {
        self.0.can_update()
    }
    pub fn reset(&mut self) {
        self.0.reset();
    }
}

#[derive(Component, Clone)]
#[require(EntityColor, Position, EntityTimer, Direction)]
pub struct Player;

impl Player {
    pub fn default_hue() -> u8 {
        85
    }
    pub fn default_position() -> u8 {
       BOARD_SIZE.div_euclid(2)
    }
    pub fn new() -> (Player, EntityColor, Position, EntityTimer) {
        (
            Player,
            EntityColor::Hue(Player::default_hue()),
            Position::new_square(Player::default_position()),
            EntityTimer::new(PLAYER_MOVE_SPEED.clone())
        )
    }
}

#[derive(Component, Clone)]
#[require(EntityColor, Position, EntityTimer, Direction)]
pub struct Enemy;

impl Enemy {
    pub fn new(rng: &mut impl Rng, last_enemy_direction: &mut DirectionType) -> (Enemy, EntityColor, Position, EntityTimer, Direction) {
        let direction = DirectionType::choose_new_random(rng, last_enemy_direction);
        *last_enemy_direction = direction;
        (
            Enemy,
            EntityColor::random_enemy_hue(rng),
            Position::random_enemy_position(direction, rng),
            EntityTimer::new(ENEMY_MOVE_SPEED.clone()),
            Direction::new(direction)
        )
    }
}

#[derive(Component)]
#[require(EntityTimer)]
pub struct DeathThroes(pub usize);

impl Default for DeathThroes {
    fn default() -> Self {
        Self(0)
    }
}

impl DeathThroes {
    fn new() -> (DeathThroes, EntityTimer) {
        (
            Self::default(),
            EntityTimer::new(DEATH_THROWS_SPEED.clone())
        )
    }
    fn can_update(&mut self) -> bool {
        self.0 += 1;
        return self.0 < DEATH_THROWS_COUNT
    }
}

/* RESOURCES */

#[derive(Resource, Default)]
pub struct LastEnemyDirection(pub DirectionType);


#[derive(Resource)]
pub struct ShowDeathThroes(bool);

impl Default for ShowDeathThroes {
    fn default() -> Self {
        Self(false)
    }
}

impl ShowDeathThroes {
    pub fn turn_on(&mut self) {
        self.0 = true;
    }
    pub fn reset(&mut self) {
        self.0 = false;
    }
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }
    pub fn get_value(&self) -> bool {
        self.0
    }
}


/* SYSTEMS */

pub fn startup_core(
    mut commands: Commands,
) {
    commands.spawn(Player::new());
    commands.insert_resource(LastEnemyDirection::default());
    commands.insert_resource(ShowDeathThroes::default());
}

pub fn home_update_wait_for_player_interaction(
    mut input_event: EventReader<InputEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(_) = input_event.read().next() {
        next_state.set(GameState::InGame);
    }
    for _ in input_event.read() {}
}

pub fn running_update_try_spawn_enemies(
    mut commands: Commands,
    mut last_enemy_direction: ResMut<LastEnemyDirection>,
    enemies: Query<(), With<Enemy>>,
    mut rng: GlobalEntropy<WyRand>
) {
    if enemies.iter().count() < MAX_ENEMIES {
        commands.spawn(Enemy::new(rng.as_mut(), &mut last_enemy_direction.0));
    }
}

pub fn running_update_enemy_position(
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut Position, &mut EntityTimer, &Direction), (With<Enemy>, Without<Player>)>,
    player_position: Single<&Position, With<Player>>,
    mut next_state: ResMut<NextState<InGameState>>,
) {
    for (entity, mut position, mut timer, direction) in enemies.iter_mut() {
        if !timer.can_update() {
            continue;
        }
        if !position.can_move(&direction.0) {
            commands.entity(entity).despawn();
            continue;
        }
        position.update_with_direction(direction.0);
        if position.eq(&player_position) {
            position.update_with_direction(!direction.0);
            next_state.set(InGameState::Dying);
            return;
        }
        timer.reset();
    }
}

pub fn running_update_player_position(
    mut input_event: EventReader<InputEvent>,
    player: Single<(&mut Position, &mut Direction, &mut EntityTimer), (With<Player>, Without<Enemy>)>,
    enemy_positions: Query<&Position, With<Enemy>>,
    mut next_state: ResMut<NextState<InGameState>>,
) {
    if let Some(event) = input_event.read().next() {
        let (mut position, mut direction, mut move_timer) = player.into_inner();
        if move_timer.can_update() {
            direction.choose_next_player_direction(event.0);
            position.update_with_direction(direction.0);
            for enemy_position in enemy_positions {
                if position.eq(enemy_position) {
                    position.update_with_direction(!direction.0);
                    next_state.set(InGameState::Dying);
                    return;
                }
            }
            move_timer.reset();
        }
    }
    for _ in input_event.read() {}
}

pub fn dying_enter(
    mut commands: Commands,
    mut show_death_throes: ResMut<ShowDeathThroes>
) {
    show_death_throes.turn_on();
    commands.spawn(DeathThroes::new());
}

pub fn dying_update_death_throes(
    death_throes_bundle: Single<(&mut DeathThroes, &mut EntityTimer)>,
    mut show_death_throes: ResMut<ShowDeathThroes>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let (mut death_throes, mut entity_timer) = death_throes_bundle.into_inner();
    if entity_timer.can_update() {
        if !death_throes.can_update() {
            next_state.set(GameState::Home);
            return;
        }
        show_death_throes.toggle();
        entity_timer.reset();
    }
}

pub fn dying_exit(
    mut commands: Commands,
    mut show_death_throes: ResMut<ShowDeathThroes>,
    death_throes: Single<Entity, With<DeathThroes>>
) {
    show_death_throes.reset();
    commands.entity(death_throes.into_inner()).despawn();
}


pub fn in_game_exit_core(
    mut commands: Commands,
    enemies: Query<Entity, With<Enemy>>,
    mut player_position: Single<&mut Position, With<Player>>,
) {
    for enemy in enemies {
        commands.entity(enemy).despawn();
    }
    player_position.update_square(Player::default_position());
}