
use defmt::{info, warn};
use embassy_rp::{peripherals::PIO0, pio_programs::ws2812::PioWs2812};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    mutex::Mutex,
    signal::Signal
};
use kg_core::{CoreSet, EntityColor, Position, ShowDeathThroes, BOARD_SIZE};
use bevy::prelude::*;
use smart_leds::RGB8;

/* todo: board size should be some multiple of physical board size? need to make this more generic */

const UBOARD_SIZE: usize = BOARD_SIZE as usize;
pub const LED_COUNT: usize = UBOARD_SIZE * UBOARD_SIZE;
static LED_READY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

type LedBuffer = [RGB8; LED_COUNT];
type LedGrid = [[RGB8; UBOARD_SIZE]; UBOARD_SIZE];
type SharedLedGrid = Mutex<CriticalSectionRawMutex, LedGrid>;
static SHARED_LED_GRID: SharedLedGrid = Mutex::new([[RGB8 { r: 0, g: 0, b: 0}; UBOARD_SIZE]; UBOARD_SIZE]);

#[allow(dead_code)]
enum BoardOrientation {
    Orient0,
    Orient90,
    Orient180,
    Orient270
}

impl BoardOrientation {
    fn get_transformation(self) -> impl Fn(usize, usize, &mut (usize, usize)) -> () {
        // not sure why I need to type position and not x / y...
        match self {
            BoardOrientation::Orient0 => |x, y, position: &mut(usize, usize)| {
                position.0 = y;
                position.1 = UBOARD_SIZE - 1 - x;
            },
            BoardOrientation::Orient90 => |x, y, position: &mut(usize, usize)| {
                position.0 = x;
                position.1 = y;
            },
            BoardOrientation::Orient180 => |x, y, position: &mut(usize, usize)| {
                position.0 = UBOARD_SIZE - 1 - y;
                position.1 = x;
            },
            BoardOrientation::Orient270 => |x, y, position: &mut(usize, usize)| {
                position.0 = UBOARD_SIZE - 1 - x;
                position.1 = UBOARD_SIZE - 1 - y;
            },
        }
    }
}

type GridBufferMap = [(usize, usize); LED_COUNT];

// todo: need to support multiple boards
fn build_grid_buffer_map(orientation: BoardOrientation) -> GridBufferMap {
    let mut grid_buffer_map = [(0, 0); LED_COUNT];
    let transform_postion = orientation.get_transformation();
    for (i, position) in grid_buffer_map.iter_mut().enumerate() {
        let y_pos = i.div_euclid(UBOARD_SIZE);
        let is_odd_y_pos = y_pos.rem_euclid(2) != 0;
        let x_pos = if is_odd_y_pos {
            UBOARD_SIZE - 1 - i.rem_euclid(UBOARD_SIZE)
        } else {
            i.rem_euclid(UBOARD_SIZE)
        };
        transform_postion(x_pos, y_pos, position);
    }
    grid_buffer_map
}

fn process_lights(grid: &LedGrid, buffer: &mut LedBuffer, map: &GridBufferMap) {
    for (i, (x, y)) in map.iter().enumerate() {
        buffer[i].clone_from(&grid[*x][*y]);
    }
}


#[embassy_executor::task]
pub async fn lighting_task(mut lights: PioWs2812<'static, PIO0, 0, LED_COUNT>) {
    info!("Initializing lighting");
    let mut led_grid = [[RGB8 { r: 0, g: 0, b: 0 }; UBOARD_SIZE]; UBOARD_SIZE];
    let mut led_buffer = [RGB8 { r: 0, g: 0, b: 0 }; LED_COUNT];
    let grid_buffer_map= build_grid_buffer_map(BoardOrientation::Orient0);
    lights.write(&led_buffer).await;
    info!("Running lighting");
    loop {
        LED_READY_SIGNAL.wait().await;
        {
            let shared_led_grid = SHARED_LED_GRID.lock().await;
            led_grid.clone_from(&shared_led_grid);
        }
        process_lights(&led_grid, &mut led_buffer, &grid_buffer_map);
        lights.write(&led_buffer).await;
    }
}

fn update_render_proxy(
    entities: Query<(&Position, &EntityColor)>,
    show_death_throes: Res<ShowDeathThroes>,
) {
    if let Ok(mut led_grid) = SHARED_LED_GRID.try_lock() {
        let base_color = if show_death_throes.get_value() {
            RGB8 { r: 255, g: 0, b: 0 }
        } else {
            RGB8 { r: 0, g: 0, b: 0 }
        };
        for led_row in led_grid.iter_mut() {
            for led in led_row.iter_mut() {
                led.clone_from(&base_color);
            }
        }
        entities.iter().for_each(|(position, entity_color)| {
            if position.out_of_bounds() {
                info!(" We are somehow out of bounds? ({:?}, {:?})", position.0.x, position.0.y);
                return;
            }
            led_grid[position.0.x as usize][position.0.y as usize].clone_from(&entity_color.into()); 
        });
        LED_READY_SIGNAL.signal(());
    } else {
        warn!("Couldn't obtain led lock...");
    }
}

#[derive(Default)]
pub struct KgLightsPlugin;

impl Plugin for KgLightsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, update_render_proxy.after(CoreSet))
        ;
    }
}