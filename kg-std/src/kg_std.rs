
use bevy::{
    color::palettes::css::{DIM_GREY, RED},
    platform::collections::HashMap,
    prelude::*,
    window::WindowResized
};

use kg_core::{CoreSet, DirectionType, EntityColor, InputEvent, Position, ShowDeathThroes, BOARD_SIZE};

/* COMPONENTS */


#[derive(Component)]
#[require(Mesh2d, MeshMaterial2d<ColorMaterial>, Transform)]
pub struct Tile(UVec2);

impl Tile {
    fn new(
        position: &UVec2,
        window_size: &Vec2,
        window_extent: &Vec2,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) -> (impl Bundle, AssetId<ColorMaterial>) {
        let tile = Tile(position.clone());
        let (width, height, x, y) = tile.rect(window_size, window_extent);
        let material = materials.add(Tile::default_color());
        let material_id = material.id();
        (
            (
                tile,
                Mesh2d(meshes.add(Rectangle::new(width, height))),
                MeshMaterial2d(material),
                Transform::from_translation(Vec3::new(
                    x,
                    y,
                    0.0,
                )),
            ),
            material_id
        )
    }

    fn default_color() -> Color {
        Color::srgb_u8(43, 44, 47)
    }

    fn rect(&self, window_size: &Vec2, window_extent: &Vec2) -> (f32, f32, f32, f32) {
        let Vec2 { x: width, y: height} = window_size / BOARD_SIZE as f32;
        let x = width * (self.0.x as f32) - window_extent.x + width * 0.5;
        let y = height * (self.0.y as f32) - window_extent.y + height * 0.5;
        (width, height, x, y)
    }
}

#[derive(Resource, Default)]
pub struct TileMaterialIndex {
    pub map: HashMap<UVec2, AssetId<ColorMaterial>>,
}

#[derive(Component)]
#[require(Mesh2d, MeshMaterial2d<ColorMaterial>, Transform)]
pub struct GridLine {
    is_vertical: bool,
    index: u32
}

impl GridLine {
    fn new(
        is_vertical: bool, index: u32, window_size: &Vec2, window_extent: &Vec2,
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
                1.0,
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
    window: Single<&Window>
) {

    commands.spawn(Camera2d);

    let window_size = window.resolution.size();
    let window_extent = window_size.map(|f| f * 0.5);

    let mut map = HashMap::new();

    for x in 0..=BOARD_SIZE  {

        for y in 0..=BOARD_SIZE {
            let position = UVec2 { x, y };
            let (tile_bundle, material_id) = Tile::new(
                &position, &window_size, &window_extent, &mut meshes, &mut materials
            );
            commands.spawn(tile_bundle);
            map.insert(position, material_id);
        }

        if x == 0 || x == BOARD_SIZE {
            continue;
        }

        // vertical bar
        commands.spawn(GridLine::new(true, x, &window_size, &window_extent, &mut meshes, &mut materials));
        // horizontal bar
        commands.spawn(GridLine::new(false, x, &window_size, &window_extent, &mut meshes, &mut materials));
    }

    commands.insert_resource(TileMaterialIndex{ map });

}

pub fn update_handle_resize(
    mut grid_lines: Query<(&GridLine, &Mesh2d, &mut Transform), (With<GridLine>, Without<Tile>)>,
    mut tiles: Query<(&Tile, &Mesh2d, &mut Transform), With<Tile>>,
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
            transform.translation = Vec3::new(x, y, 1.0);
        });
        tiles.iter_mut().for_each(|(tile, mesh2d, mut transform)| {
            let (width, height, x, y) = tile.rect(&window_size, &window_extent);
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

pub fn update_render_clear_tiles(
    tiles: Query<&MeshMaterial2d<ColorMaterial>, With<Tile>>,
    show_death_throes: Res<ShowDeathThroes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let base_color = if show_death_throes.get_value() {
        Color::from(RED)
    } else {
        Tile::default_color()
    };
    tiles.iter().for_each(|material| {
        if let Some(m) = materials.get_mut(material.id()) {
            *m = base_color.into()
        }
    });
}

pub fn update_render_populate_tiles(
    entities: Query<(&Position, &EntityColor)>,
    tile_material_index: ResMut<TileMaterialIndex>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    entities.iter().for_each(|(position, entity_color)| {
        let material_id = tile_material_index.map.get(&position.0).unwrap();
        if let Some(m) = materials.get_mut(*material_id) {
            *m = entity_color.to_color().into()
        }
    });
}


/* PLUGINS */

pub struct KgStdPlugin;

impl Plugin for KgStdPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup,
            startup_std.after(CoreSet)
        )
        .add_systems(Update, (
            update_handle_keyboard.before(CoreSet),
            update_handle_resize,
            (
                update_render_clear_tiles,
                update_render_populate_tiles
            ).chain().after(CoreSet)
        ))
        ;
    }
}