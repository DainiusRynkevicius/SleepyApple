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

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
}

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMCopyAssertionsStatus(assertions: *mut CFDictionaryRef) -> i32;
}

fn is_macos_locked() -> bool {
    unsafe {
        let dict_ptr = CGSessionCopyCurrentDictionary();
        if dict_ptr.is_null() {
            return false;
        }

        let dict: CFDictionary = CFDictionary::wrap_under_create_rule(dict_ptr);
        let key = CFString::from_static_string("CGSSessionScreenIsLocked");

        match dict.find(key.as_CFTypeRef().cast()) {
            Some(val_ptr) => {
                let bool_ref: CFBoolean = CFBoolean::wrap_under_get_rule(val_ptr.cast());
                return bool_ref.into();
            }
            None => false,
        }
    }
}

pub fn prevent_user_idle_system_sleep() -> Result<bool, &'static str> {
    unsafe {
        let mut dict_ref: CFDictionaryRef = ptr::null();
        let result = IOPMCopyAssertionsStatus(&mut dict_ref);

        if result != 0 || dict_ref.is_null() {
            return Err("IOPMCopyAssertionsStatus failed");
        }

        let dict =
            CFDictionary::<CFString, CFTypeRef>::wrap_under_create_rule(dict_ref as *const _);

        let key = CFString::new("PreventUserIdleSystemSleep");

        let value = dict.find(&key).ok_or("Key not found")?;

        let number = CFNumber::wrap_under_get_rule(*value as *const _);

        number
            .to_i64()
            .map(|v| v == 1)
            .ok_or("Failed to convert CFNumber")
    }
}

fn main() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::default())
        .expect("Failed to initialize tracing.");
    info!("Starting Sleeping Apple...");
    let mut screen_off_start: Option<Instant> = None;

    loop {
        let display = CGDisplay::main();
        let display_sleeping = display.is_asleep();
        let device_locked = is_macos_locked();
        let idle_prevented = prevent_user_idle_system_sleep().unwrap_or_else(|e| {
            error!("An Error occurred retrieving idle sleep status: {}", e);
            false
        });
        // if device_locked {
        //     info!("Device locked");
        // }
        // if display_sleeping {
        //     info!("Device display sleeping.");
        // }
        // if idle_prevented {
        //     info!("Idle sleep prevented.");
        // }
        if display_sleeping && (!idle_prevented || device_locked) {
            match screen_off_start {
                None => {
                    info!("Sleep Timer started.");
                    screen_off_start = Some(Instant::now());
                }
                Some(start) => {
                    if start.elapsed() >= Duration::from_secs(SLEEP_AFTER_SECONDS) {
                        info!("Sleeping...");
                        Command::new("pmset")
                            .arg("sleepnow")
                            .output()
                            .expect("Failed to execute sleep.");

                        screen_off_start = None;
                        thread::sleep(Duration::from_secs(10));
                    }
                }
            }
        } else {
            if screen_off_start.is_some() {
                info!("Sleep canceled.");
                screen_off_start = None;
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}
