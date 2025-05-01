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
use defmt::*;
use embassy_executor::Executor;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use core::time::Duration as CoreDuration;
use core::mem::MaybeUninit;
use static_cell::StaticCell;
use embedded_alloc::LlffHeap as Heap;
use {defmt_rtt as _, panic_probe as _};

use kg_embedded::EmbassyPlugin;



#[global_allocator]
static HEAP: Heap = Heap::empty();
const HEAP_SIZE: usize = 100 * 1024;

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
const CHANNEL_SIZE: usize = 10;
static CHANNEL: Channel<CriticalSectionRawMutex, (), CHANNEL_SIZE> = Channel::new();
static TIME_TO_FLOP: CoreDuration = CoreDuration::from_millis(500);


#[derive(Resource)]
struct EmbeddedManager {
    pub last_flop: Instant,
    pub sender: Sender<'static, CriticalSectionRawMutex, (), CHANNEL_SIZE>
}

impl EmbeddedManager {
    fn new() -> Self {
        Self {
            last_flop: Instant::now(),
            sender: CHANNEL.sender()
        }
    }

    fn should_flop(&self, now: &Instant) -> bool {
        *now - self.last_flop >= TIME_TO_FLOP
    }

    fn flop(&mut self, now: Instant) {
        self.last_flop = now;
        self.sender.try_send(()).unwrap();
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {

    {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    let p = embassy_rp::init(Default::default());
    let led = Output::new(p.PIN_25, Level::Low);

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| unwrap!(spawner.spawn(core1_task(led))));
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| unwrap!(spawner.spawn(core0_task())));
}

#[embassy_executor::task]
async fn core0_task() {
    info!("Starting bevy on core 0");
    info!("free={}", HEAP.free());
    let mut app = App::new();
    app
        .add_plugins((
            EmbassyPlugin,
            PanicHandlerPlugin,
            FrameCountPlugin,
            TimePlugin,
            ScheduleRunnerPlugin::run_loop(CoreDuration::from_millis(33)),
            StatesPlugin,
        ))
        .insert_resource(EmbeddedManager::new())
        .add_systems(Update, |mut manager: ResMut<EmbeddedManager>| {
            let now = Instant::now();
            if manager.should_flop(&now) {
                manager.flop(now);
            }
        })
        .run()
    ;
}


#[embassy_executor::task]
async fn core1_task(mut led: Output<'static>) {
    info!("Starting embassy from core 1");
    info!("free={}", HEAP.free());
    let receiver = CHANNEL.receiver();
    loop {
        receiver.receive().await;
        led.toggle();
    }
}