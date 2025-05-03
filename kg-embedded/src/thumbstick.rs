use defmt::info;
use embassy_rp::adc::{Adc, Async, Channel as AdcChannel};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel as SyncChannel, Receiver};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use kg_core::{CoreSet, DirectionType, InputEvent};
use bevy::prelude::*;

const THUMBSTICK_CHANNEL_SIZE: usize = 10;
static THUMBSTICK_CHANNEL: SyncChannel<CriticalSectionRawMutex, DirectionType, THUMBSTICK_CHANNEL_SIZE> = SyncChannel::new();
const THUMBSTICK_TICK_RATE_IN_HZ: u64 = 30;

static THUMBSTICK_START_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub struct Thumbstick {
    adc: Adc<'static, Async>,
    vrx_pin: AdcChannel<'static>,
    vry_pin: AdcChannel<'static>,
}

impl Thumbstick {
    pub fn new(adc: Adc<'static, Async>, vrx_pin: AdcChannel<'static>, vry_pin: AdcChannel<'static>) -> Self {
        Self {
            adc, vrx_pin, vry_pin
        }
    }
    pub async fn read(&mut self) -> Option<DirectionType> {
        let vrx = self.adc.read(&mut self.vrx_pin).await.unwrap();
        let vry = self.adc.read(&mut self.vry_pin).await.unwrap();
        // consider more than a quarter move to be in that direction
        let x_direction = if vrx < 256 {
            Some(false)
        } else if vrx >= 768 {
            Some(true)
        } else {
            None
        };
        let y_direction = if vry < 256 {
            Some(false)
        } else if vry >= 768 {
            Some(true)
        } else {
            None
        };
        match (x_direction, y_direction) {
            (Some(false), Some(false)) => Some(DirectionType::UpLeft),
            (Some(false), Some(true)) => Some(DirectionType::UpRight),
            (Some(false), _) => Some(DirectionType::Up),
            (Some(true), Some(false)) => Some(DirectionType::DownLeft),
            (Some(true), Some(true)) => Some(DirectionType::DownRight),
            (Some(true), _) => Some(DirectionType::Down),
            (_, Some(false)) => Some(DirectionType::Left),
            (_, Some(true)) => Some(DirectionType::Right),
            _ => None
        }
    }
}


#[embassy_executor::task]
pub async fn thumbstick_task(mut thumbstick: Thumbstick) {
    info!("Thumstick waiting for bevy to startup");
    THUMBSTICK_START_SIGNAL.wait().await;
    let sender = THUMBSTICK_CHANNEL.sender();
    let mut ticker = Ticker::every(Duration::from_hz(THUMBSTICK_TICK_RATE_IN_HZ));
    info!("Runnng thumbstick");
    loop {
        if let Some(direction) = thumbstick.read().await {
            sender.send(direction).await;
        }
        ticker.next().await;
    }
}

#[derive(Resource)]
struct ThumbstickChannel(
    Receiver<'static, CriticalSectionRawMutex, DirectionType, THUMBSTICK_CHANNEL_SIZE>
);

impl ThumbstickChannel {
    fn new() -> Self {
        Self(THUMBSTICK_CHANNEL.receiver())
    }
    fn receive(&mut self) -> Option<DirectionType> {
        if let Ok(direction) = self.0.try_receive() {
            self.0.clear();
            Some(direction)
        } else {
            None
        }
    }
}

fn update_proxy_thumbstick(
    mut channel: ResMut<ThumbstickChannel>,
    mut event_writer: EventWriter<InputEvent>,
) {
    if let Some(event) = channel.receive() {
        event_writer.write(InputEvent(event));
    }
}

#[derive(Default)]
pub struct KgThumbstickPlugin;

impl Plugin for KgThumbstickPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ThumbstickChannel::new())
            .add_systems(PostStartup, || {
                // todo: maybe this should be pubsub? Alhtough on this project, won't be any more uses i dont think
                THUMBSTICK_START_SIGNAL.signal(());
            })
            .add_systems(Update, update_proxy_thumbstick.before(CoreSet))
        ;
    }
}