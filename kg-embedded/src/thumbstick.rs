use defmt::info;
use embassy_rp::adc::{Adc, Async, Channel as AdcChannel};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel as SyncChannel, Receiver};
use embassy_time::{Duration, Ticker};
use kg_core::{DirectionType, InputEvent};
use bevy::prelude::*;

const THUMBSTICK_CHANNEL_SIZE: usize = 10;
static THUMBSTICK_CHANNEL: SyncChannel<CriticalSectionRawMutex, DirectionType, THUMBSTICK_CHANNEL_SIZE> = SyncChannel::new();
const THUMBSTICK_TICK_RATE_IN_HZ: u64 = 30;

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
pub struct ThumbstickChannel(
    Receiver<'static, CriticalSectionRawMutex, DirectionType, THUMBSTICK_CHANNEL_SIZE>
);

impl ThumbstickChannel {
    pub fn new() -> Self {
        Self(THUMBSTICK_CHANNEL.receiver())
    }
    pub fn receive(&mut self) -> Option<DirectionType> {
        if let Ok(direction) = self.0.try_receive() {
            self.0.clear();
            Some(direction)
        } else {
            None
        }
    }
}

pub fn update_proxy_thumbstick(
    mut channel: ResMut<ThumbstickChannel>,
    mut event_writer: EventWriter<InputEvent>,
) {
    if let Some(event) = channel.receive() {
        event_writer.write(InputEvent(event));
    }
}
