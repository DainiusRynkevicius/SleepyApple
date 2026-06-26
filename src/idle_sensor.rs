use std::ptr;
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMCopyAssertionsStatus(assertions: *mut CFDictionaryRef) -> i32;
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
            .map(|v| v > 0)
            .ok_or("Failed to convert CFNumber")
    }
}