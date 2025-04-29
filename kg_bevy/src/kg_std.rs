
use bevy::{color::palettes::css::{DIM_GREY, RED}, prelude::*, window::WindowResized};

use crate::kg_core::{DirectionType, Enemy, EntityColor, InputEvent, Player, Position, ShowDeathThroes, BOARD_SIZE};

/* COMPONENTS */


#[derive(Component)]
#[require(EntityColor, Position, Mesh2d, MeshMaterial2d<ColorMaterial>, Transform)]
pub struct Tile;

impl Tile {
    fn new(
        entity_color: &EntityColor, position: &Position, window_size: &Vec2, window_extent: &Vec2,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) -> (Tile, Mesh2d, MeshMaterial2d<ColorMaterial>, Transform) {
        let (width, height, x, y) = Tile::rect(position, window_size, window_extent);
        (
            Tile,
            Mesh2d(meshes.add(Rectangle::new(width, height))),
            MeshMaterial2d(materials.add(entity_color.to_color())),
            Transform::from_translation(Vec3::new(
                x,
                y,
                0.0,
            )),
        )
    }

    fn rect(position: &Position, window_size: &Vec2, window_extent: &Vec2) -> (f32, f32, f32, f32) {
        let Vec2 { x: width, y: height} = window_size / BOARD_SIZE as f32;
        let x = width * (position.x as f32) - window_extent.x + width * 0.5;
        let y = height * (position.y as f32) - window_extent.y + height * 0.5;
        (width, height, x, y)
    }
}


#[derive(Component)]
#[require(Mesh2d, MeshMaterial2d<ColorMaterial>, Transform)]
pub struct GridLine {
    is_vertical: bool,
    index: u8
}

impl GridLine {
    fn new(
        is_vertical: bool, index: u8, window_size: &Vec2, window_extent: &Vec2,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) -> (GridLine, Mesh2d, MeshMaterial2d<ColorMaterial>, Transform) {
        let grid_line = GridLine { is_vertical, index };
        let (width, height, x, y) = grid_line.rect(window_size, window_extent);
        (
            grid_line,
            Mesh2d(meshes.add(Rectangle::new(width, height))),
            MeshMaterial2d(materials.add(Color::from(DIM_GREY))),
            Transform::from_translation(Vec3::new(
                x,
                y,
                0.0,
            )),
        )
    }
    fn rect(&self, window_size: &Vec2, window_extent: &Vec2) -> (f32, f32, f32, f32) {
        let width = if self.is_vertical {
            1.0
        } else {
            window_size.x
        };
        let height = if self.is_vertical {
            window_size.y
        } else {
            1.0
        };
        let x = if self.is_vertical {
            window_size.x / BOARD_SIZE as f32 * self.index as f32 - window_extent.x
        } else {
            0.0
        };
        let y = if self.is_vertical {
            0.0
        } else {
            window_size.y / BOARD_SIZE as f32 * self.index as f32 - window_extent.y
        };
        (width, height, x, y)
    }
}

/* SYSTEMS */

pub fn startup_std(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window: Single<&Window>,
    // todo: this is jank
    player: Single<(Entity, &EntityColor, &Position), With<Player>>
) {

    commands.spawn(Camera2d);

    let window_size = window.resolution.size();
    let window_extent = window_size.map(|f| f * 0.5);

    for i in 1..BOARD_SIZE {
        // vertical bar
        commands.spawn(GridLine::new(true, i, &window_size, &window_extent, &mut meshes, &mut materials));
        // horizontal bar
        commands.spawn(GridLine::new(false, i, &window_size, &window_extent, &mut meshes, &mut materials));
    }

    commands.entity(player.0).insert(Tile::new(
        &player.1, &player.2, &window_size, &window_extent, &mut meshes, &mut materials
    ));

}

pub fn update_handle_resize(
    mut grid_lines: Query<(&GridLine, &Mesh2d, &mut Transform), (With<GridLine>, Without<Tile>)>,
    mut tiles: Query<(&Position, &Mesh2d, &mut Transform), With<Tile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut resize_reader: EventReader<WindowResized>,
) {
    for e in resize_reader.read() {
        let window_size = Vec2::new(e.width, e.height);
        let window_extent = window_size.map(|f| f * 0.5);
        grid_lines.iter_mut().for_each(|(grid_line, mesh2d, mut transform)| {
            let (width, height, x, y)  = grid_line.rect(&window_size, &window_extent);
            if let Some(mesh) = meshes.get_mut(&mesh2d.0) {
                *mesh = Rectangle::new(width, height).into();
            }
            transform.translation = Vec3::new(x, y, 0.0);
        });
        tiles.iter_mut().for_each(|(position, mesh2d, mut transform)| {
            let (width, height, x, y) = Tile::rect(position, &window_size, &window_extent);
            if let Some(mesh) = meshes.get_mut(&mesh2d.0) {
                *mesh = Rectangle::new(width, height).into();
            }
            transform.translation = Vec3::new(x, y, 0.0);
        });
    }
}


pub fn update_handle_keyboard(
    mut input_event: EventWriter<InputEvent>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let event = match [
        KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight
    ].map(|key_code| keys.pressed(key_code)) {
        [true, _, true, _] => Some(DirectionType::UpLeft),
        [true, _, _, true] => Some(DirectionType::UpRight),
        [true, _, _, _] => Some(DirectionType::Up),
        [_, true, true, _] => Some(DirectionType::DownLeft),
        [_, true, _, true] => Some(DirectionType::DownRight),
        [_, true, _, _] => Some(DirectionType::Down),
        [_, _, true, _] => Some(DirectionType::Left),
        [_, _, _, true] => Some(DirectionType::Right),
        _ => None
    };
    if let Some(e) = event {
        input_event.write(InputEvent(e));
    }
}

pub fn update_render_tiles(
    mut tiles: Query<(&Position, &EntityColor, &mut Transform, &MeshMaterial2d<ColorMaterial>), With<Tile>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window: Single<&Window>,
) {
    let window_size = window.resolution.size();
    let window_extent = window_size.map(|f| f * 0.5);
    tiles.iter_mut().for_each(|(position, entity_color, mut transform, material)| {
        let (_, _, x, y) = Tile::rect(position, &window_size, &window_extent);
        transform.translation = Vec3::new(x, y, 0.0);
        if let Some(m) = materials.get_mut(material.id()) {
            *m = entity_color.to_color().into()
        }
    });
}

pub fn update_render_camera(
    show_death_throes: Res<ShowDeathThroes>,
    mut camera: Single<&mut Camera>,
) {
    if show_death_throes.get_value() {
        camera.clear_color = ClearColorConfig::Custom(RED.into());
    } else {
        camera.clear_color = ClearColorConfig::default();

    }
}

pub fn on_add_enemy(
    trigger: Trigger<OnAdd, Enemy>,
    mut commands: Commands,
    query: Query<(&EntityColor, &Position), With<Enemy>>,
    window: Single<&Window>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window_size = window.resolution.size();
    let window_extent = window_size.map(|f| f * 0.5);
    let enemy_entity = query.get(trigger.target()).unwrap();
    commands.entity(trigger.target()).insert(Tile::new(
        enemy_entity.0, enemy_entity.1, &window_size, &window_extent, &mut meshes, &mut materials
    ));
}