mod display;
mod idle_sensor;
mod light_sensor;
mod lock_sensor;

use crate::display::DisplaySensor;
use crate::light_sensor::LightSensor;
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::display::CGDisplay;
use std::process::Command;
use std::time::{Duration, Instant};
use std::{ptr, thread};
use tracing::{error, info};

const SLEEP_AFTER_SECONDS: u64 = 5;
struct App {
    event_timestamps: EventTimestamps,
    sleep_timer: Option<Instant>,
    light_sensor: Option<LightSensor>,
    display_sensor: DisplaySensor,
}
#[derive(Default)]
struct EventTimestamps {
    lock: Option<Instant>,
    display_sleep: Option<Instant>,
    als: Option<Instant>,
    idle_prevented: Option<Instant>,
}

impl EventTimestamps {
    pub fn update_lock(&mut self, value: bool) {
        if self.lock.is_none() && value {
            info!("Mac locked");
            self.lock = Some(Instant::now());
        }
        if self.lock.is_some() && !value {
            info!("Mac unlocked");
            self.lock = None;
        }
    }
    pub fn update_display_sleep(&mut self, value: bool) {
        if self.display_sleep.is_none() && value {
            info!("Display sleeping");
            self.display_sleep = Some(Instant::now());
        }
        if self.display_sleep.is_some() && !value {
            info!("Display awake");
            self.display_sleep = None;
        }
    }
    pub fn update_als(&mut self, value: bool) {
        if self.als.is_none() && value {
            info!("Light sensor is dark");
            self.als = Some(Instant::now());
        }
        if self.als.is_some() && !value {
            info!("Light sensor is bright");
            self.als = None;
        }
    }
    pub fn update_idle_prevented(&mut self, value: bool) {
        if self.idle_prevented.is_none() && value {
            info!("Idle prevented");
            self.idle_prevented = Some(Instant::now());
        }
        if self.idle_prevented.is_some() && !value {
            info!("Idle no longer prevented");
            self.idle_prevented = None;
        }
    }
}
impl App {
    pub fn should_sleep(&self) -> bool {
        if let Some(lock) = self.event_timestamps.lock
            && let Some(display_sleep) = self.event_timestamps.display_sleep
        {
            let diff = lock.elapsed().abs_diff(display_sleep.elapsed());
            info!("Difference between lock and display sleep: {}", diff.as_millis());
            // User locks the mac. Then screen goes to sleep. This prevents the mac's screen going to sleep, which automatically locks the mac.
            if lock.elapsed() > display_sleep.elapsed() {
                return true;
            } else if self.event_timestamps.idle_prevented.is_none() {
                return true;
            }
        }

        false
    }
    pub fn run_tick(&mut self) {
        self.query_sensors();
        if self.should_sleep() {
            match self.sleep_timer {
                None => {
                    info!("Sleep timer started.");
                    self.sleep_timer = Some(Instant::now());
                }
                Some(start) => {
                    if start.elapsed() >= Duration::from_secs(SLEEP_AFTER_SECONDS) {
                        info!("Sleeping...");

                        Command::new("pmset")
                            .arg("sleepnow")
                            .output()
                            .expect("Failed to execute sleep.");

                        self.sleep_timer = None;
                        thread::sleep(Duration::from_secs(10));
                    }
                }
            }
        } else {
            if self.sleep_timer.is_some() {
                info!("Sleep canceled.");
                self.sleep_timer = None;
            }
        }
    }

    pub fn new() -> Self {
        let light_sensor = LightSensor::new().ok();
        Self {
            event_timestamps: Default::default(),
            sleep_timer: None,
            light_sensor,
            display_sensor: DisplaySensor::new(),
        }
    }

    pub fn query_sensors(&mut self){
        self.event_timestamps.update_lock(lock_sensor::is_macos_locked());
        if let Some(als) = &self.light_sensor{
            self.event_timestamps.update_als(als.value().map(|t| t == 0.0).unwrap_or(false))
        }
        self.event_timestamps.update_idle_prevented(idle_sensor::prevent_user_idle_system_sleep().unwrap_or(false));
        self.event_timestamps.update_display_sleep(self.display_sensor.sleeping());
    }
}

fn main() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::default())
        .expect("Failed to initialize tracing.");
    info!("Starting Sleeping Apple...");
    let mut app = App::new();


    loop {
        app.run_tick();

        thread::sleep(Duration::from_millis(100));
    }
}
