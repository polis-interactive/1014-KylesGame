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
use defmt::{info, unwrap};
use embassy_executor::{Executor, Spawner};
use embassy_rp::adc::{Adc, Channel, Config, InterruptHandler as AdcInterruptHandler};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output, Pull};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use kg_core::KgCorePlugin;
use kg_embedded::{thumbstick_task, EmbassyPlugin, KgEmbeddedPlugin, Thumbstick, LED_COUNT};
use portable_atomic::AtomicBool;
use rand::RngCore;
use core::sync::atomic::Ordering;
use core::time::Duration as CoreDuration;
use core::mem::MaybeUninit;
use static_cell::StaticCell;
use embedded_alloc::LlffHeap as Heap;
use getrandom::Error as GetRandomError;
use defmt_rtt as _;

#[global_allocator]
static HEAP: Heap = Heap::empty();
const HEAP_SIZE: usize = 150 * 1024;

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


#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), GetRandomError> {
    let buf = unsafe {
        // fill the buffer with zeros
        core::ptr::write_bytes(dest, 0, len);
        // create mutable byte slice
        core::slice::from_raw_parts_mut(dest, len)
    };
    let mut rng = RoscRng;
    rng.try_fill_bytes(buf).map_err(|_| {
        GetRandomError::new_custom(1)
    })
}


#[cortex_m_rt::entry]
fn main() -> ! {

    info!("Initialize heap");

    {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    info!("Initialize peripherals");

    let p = embassy_rp::init(Default::default());
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);

    let led = Output::new(p.PIN_25, Level::Low);

    let adc = Adc::new(p.ADC, Irqs, Config::default());
    let vrx_pin = Channel::new_pin(p.PIN_26, Pull::None);
    let vry_pin = Channel::new_pin(p.PIN_27, Pull::None);

    let thumbstick = Thumbstick::new(adc, vrx_pin, vry_pin);

    let program = PioWs2812Program::new(&mut common);
    let lights = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_12, &program);

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
    executor0.run(|spawner| unwrap!(spawner.spawn(core0_task(led))));
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

#[embassy_executor::task]
async fn core0_task(led: Output<'static>) {
    
    info!("Initialize bevy");

    info!("free={}", HEAP.free());
    

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
        .add_systems(PreStartup, || {
            info!("free={}", HEAP.free());
        })
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
        })
        .run()
    ;
}


#[embassy_executor::task]
async fn core1_task(
    spawner: Spawner, thumbstick: Thumbstick, _lights: PioWs2812<'static, PIO0, 0, LED_COUNT>
) {
    info!("Initialize I/O");
    info!("free={}", HEAP.free());
    spawner.must_spawn(thumbstick_task(thumbstick));
    info!("free={}", HEAP.free());
}