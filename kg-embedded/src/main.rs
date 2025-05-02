//! This example shows how to send messages between the two cores in the RP2040 chip.
//!
//! The LED on the RP Pico W board is connected differently. See wifi_blinky.rs.

#![no_std]
#![no_main]

extern crate alloc;

use bevy::app::{PanicHandlerPlugin, ScheduleRunnerPlugin};
use bevy::diagnostic::FrameCountPlugin;
use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimePlugin;
use defmt::{error, info, unwrap, warn, Format};
use embassy_executor::{Executor, Spawner};
use embassy_futures::block_on;
use embassy_rp::adc::{Adc, Channel, Config, InterruptHandler as AdcInterruptHandler};
use embassy_rp::bind_interrupts;
use embassy_rp::flash::{Async, Flash, ERASE_SIZE};
use embassy_rp::gpio::{Level, Output, Pull};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{FLASH, PIO0};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embedded_storage_async::nor_flash::MultiwriteNorFlash;
use kg_core::{GameState, InGameState, KgCorePlugin};
use kg_embedded::{lighting_task, thumbstick_task, EmbassyPlugin, KgEmbeddedPlugin, Thumbstick, LED_COUNT};
use portable_atomic::AtomicBool;
use sequential_storage::erase_all;
use sequential_storage::map::store_item;
use sequential_storage::{map::fetch_item, cache::NoCache};
use core::ops::Range;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration as CoreDuration;
use core::mem::MaybeUninit;
use static_cell::StaticCell;
use embedded_alloc::LlffHeap as Heap;
use defmt_rtt as _;

#[global_allocator]
static HEAP: Heap = Heap::empty();
const HEAP_SIZE: usize = 150 * 1024;

const FLASH_SIZE: u32 = 2 * 1024 * 1024;
const UFLASH_SIZE: usize = FLASH_SIZE as usize;
const USIZE_SIZE: usize = core::mem::size_of::<usize>();
static MIN_HEAP_SIZE: AtomicUsize = AtomicUsize::new(HEAP_SIZE);


async fn load_store<E: defmt::Format>(
    flash: &mut impl MultiwriteNorFlash<Error = E>,
    flash_range: &Range<u32>,
) {
    let mut data_buffer = [0; 32];
    let fetched = fetch_item::<u8, &[u8], _>(
        flash,
        flash_range.clone(),
        &mut NoCache::new(),
        &mut data_buffer,
        &0u8,
    ).await;
    if let Ok(Some(raw_buffer)) = fetched {
        if let Ok(buffer) = TryInto::<[u8; USIZE_SIZE]>::try_into(raw_buffer) {
            MIN_HEAP_SIZE.store(usize::from_ne_bytes(buffer), Ordering::Relaxed);
            return;
        } else {
            warn!("Persisted store is either the wrong format or corrupted");
        }
    } else if let Err(e) = fetched {
        error!("Persisted store is corrupted: {:?}", e);
    } else {
        warn!("No data in the persisted store");
    }
    let _ = erase_all(flash, flash_range.clone()).await;
}

async fn write_store<E: defmt::Format>(
    flash: &mut impl MultiwriteNorFlash<Error = E>,
    flash_range: &Range<u32>,
  ) {
    let to_store = MIN_HEAP_SIZE.load(Ordering::Relaxed).to_ne_bytes();
    let mut data_buffer: [u8; 32] = [0; 32];
    let stored = store_item(
      flash,
      flash_range.clone(),
      &mut NoCache::new(),
      &mut data_buffer,
      &0u8,
      &to_store.as_slice(),
    ).await;
    if let Err(e) = stored {
      error!("Failed to persist store to disk with err: {:?}", e);
    }
  }
  


#[panic_handler] // built-in ("core") attribute
fn core_panic(info: &core::panic::PanicInfo) -> ! {
    static PANICKED: AtomicBool = AtomicBool::new(false);

    cortex_m::interrupt::disable();

    // Guard against infinite recursion, just in case.
    if !PANICKED.load(Ordering::Relaxed) {
        PANICKED.store(true, Ordering::Relaxed);

        info!("{:?}", info);
    }

    info!("free={}", HEAP.free());

    const SHCSR: *mut u32 = 0xE000ED24usize as _;
    const USGFAULTENA: usize = 18;

    unsafe {
        let mut shcsr = core::ptr::read_volatile(SHCSR);
        shcsr &= !(1 << USGFAULTENA);
        core::ptr::write_volatile(SHCSR, shcsr);
    }

    cortex_m::asm::udf();
}

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => AdcInterruptHandler;
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});


#[cortex_m_rt::entry]
fn main() -> ! {

    info!("Initialize heap");

    {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    info!("Initialize peripherals");

    let p = embassy_rp::init(Default::default());


    let flash = Flash::<_, Async, UFLASH_SIZE>::new(p.FLASH, p.DMA_CH1);
    let flash_range_start = (flash.capacity() - 4 * ERASE_SIZE) as u32;
    let flash_range_end = flash.capacity() as u32;
    let map_flash_range = flash_range_start..flash_range_end;


    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);

    let led = Output::new(p.PIN_25, Level::Low);

    let adc = Adc::new(p.ADC, Irqs, Config::default());
    let vrx_pin = Channel::new_pin(p.PIN_26, Pull::None);
    let vry_pin = Channel::new_pin(p.PIN_27, Pull::None);

    let thumbstick = Thumbstick::new(adc, vrx_pin, vry_pin);

    let program = PioWs2812Program::new(&mut common);
    let lights = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_19, &program);

    info!("Initialize I/O core");

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| unwrap!(spawner.spawn(core1_task(spawner, thumbstick, lights))));
        },
    );

    info!("Initialize bevy core");

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| unwrap!(spawner.spawn(core0_task(led, flash, map_flash_range))));
}

#[derive(Resource)]
pub struct LedResource {
    pub last_flop: Instant,
    pub led: Output<'static>
}

static TIME_TO_FLOP: CoreDuration = CoreDuration::from_millis(500);

impl LedResource {
    fn should_flop(&self, now: &Instant) -> bool {
        *now - self.last_flop >= TIME_TO_FLOP
    }

    fn flop(&mut self, now: Instant) {
        self.last_flop = now;
        self.led.toggle();
    }
}

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        pub fn log_transitions<S: States + Format>(mut transitions: EventReader<StateTransitionEvent<S>>) {
            // State internals can generate at most one event (of type) per frame.
            let Some(transition) = transitions.read().last() else {
                return;
            };
            let name = core::any::type_name::<S>();
            let StateTransitionEvent { exited, entered } = transition;
            match (exited, entered) {
                (Some(o), Some(n)) => info!("{} transition: {:?} => {:?}", name, o, n),
                (_, Some(n)) => info!("{} transition: N/A => {:?}", name, n),
                (Some(o), _) => info!("{} transition: {:?} => N/A", name, o),
                _ => info!("{} transition: idk...", name)
            }  
            info!("Heap At transition: free={}", HEAP.free());
        }
    }
}


// really need to try and get bevy running on core 1
#[derive(Resource)]
struct FlashResource {
    last_check: Instant,
    flash: Flash<'static, FLASH, Async, UFLASH_SIZE>,
    flash_range: Range<u32>
}

static TIME_TO_WRITE: CoreDuration = CoreDuration::from_secs(3);

impl FlashResource {
    fn could_write(&self, now: &Instant) -> bool {
        *now - self.last_check >= TIME_TO_WRITE
    }
    fn try_write(&mut self, now: Instant) {
        let now_free = HEAP.free();
        let last_free = MIN_HEAP_SIZE.load(Ordering::Relaxed);
        if now_free < last_free {
            MIN_HEAP_SIZE.store(now_free, Ordering::Relaxed);
            cfg_if::cfg_if! {
                if #[cfg(not(debug_assertions))] {
                    block_on(write_store(&mut self.flash, &self.flash_range));
                }
            }
            info!("Updated max heap used: total={} free={}", HEAP_SIZE, now_free);
        } else {

            info!("Have not updated max heap used: total={} free={}", HEAP_SIZE, last_free);
        }
        self.last_check = now;
    }
}

#[embassy_executor::task]
async fn core0_task(
    led: Output<'static>,
    mut flash: Flash<'static, FLASH, Async, UFLASH_SIZE>,
    flash_range: Range<u32>
) {
    
    info!("Initialize bevy");

    load_store(&mut flash, &flash_range).await;
    info!("Max heap used: total={} free={}", HEAP_SIZE, MIN_HEAP_SIZE.load(Ordering::Relaxed));

    info!("PreBevy free={}", HEAP.free());

    let mut app = App::new();
    app
        .add_plugins((
            EmbassyPlugin,
            (
                PanicHandlerPlugin,
                FrameCountPlugin,
                TimePlugin,
                ScheduleRunnerPlugin::run_loop(CoreDuration::from_millis(33)),
                StatesPlugin
            ),
            KgEmbeddedPlugin,
            KgCorePlugin
        ))
        .insert_resource(
            LedResource {
                last_flop: Instant::now(),
                led
            }
        )
        .add_systems(Update, |mut led_resource: ResMut<LedResource>| {
            let now = Instant::now();
            if led_resource.should_flop(&now) {
                led_resource.flop(now);
            }
        });

    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            app
                .add_systems(PreStartup, || {
                    info!("PreStartup free={}", HEAP.free());
                })
                .add_systems(PostStartup, || {
                    info!("PostStartup free={}", HEAP.free());
                })
                .add_systems(Update, (log_transitions::<GameState>, log_transitions::<InGameState>))
            ;
        }
    }

    app
        .insert_resource(FlashResource { last_check: Instant::now(), flash, flash_range })
        .add_systems(Update, |mut flash_resource: ResMut<FlashResource>| {
            let now = Instant::now();
            if flash_resource.could_write(&now) {
                flash_resource.try_write(now);
            }
        })
    ;

    app.run();
}


#[embassy_executor::task]
async fn core1_task(
    spawner: Spawner,
    thumbstick: Thumbstick,
    lights: PioWs2812<'static, PIO0, 0, LED_COUNT>,
) {
    info!("Initialize I/O");
    spawner.must_spawn(lighting_task(lights));
    spawner.must_spawn(thumbstick_task(thumbstick));
}
