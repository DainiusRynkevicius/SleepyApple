use std::ffi::c_void;
use std::ptr;
use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation::base::{CFRelease, CFTypeRef};
use core_foundation::dictionary::{kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate, CFDictionaryRef};
use core_foundation::number::{kCFNumberIntType, CFNumberCreate};
use core_foundation::string::{kCFStringEncodingUTF8, CFStringCreateWithCString};
use thiserror::Error;

// Magic Numbers for Apple Internal Sensors
// Usage Page 0xFF00 = Apple Vendor
// Usage 0x0004 = Ambient Light Sensor
const HID_PAGE_APPLE_VENDOR: i32 = 0xFF00;
const HID_USAGE_APPLE_ALS: i32 = 0x0004;

const K_IOHID_EVENT_TYPE_ALS: i32 = 12;

// kIOHIDEventFieldAmbientLightSensorLevel = (Type << 16) | Index
// (12 << 16) | 0 = 786432
const K_IOHID_EVENT_FIELD_ALS_LEVEL: i32 = 786432;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDEventSystemClientCreate(allocator: CFTypeRef) -> CFTypeRef;

    fn IOHIDEventSystemClientSetMatching(client: CFTypeRef, match_dict: CFDictionaryRef);

    fn IOHIDEventSystemClientCopyServices(client: CFTypeRef) -> CFTypeRef; // Returns CFArrayRef

    fn IOHIDServiceClientCopyEvent(service: CFTypeRef, event_type: i32, options: i32, parent: i32) -> CFTypeRef;

    fn IOHIDEventGetFloatValue(event: CFTypeRef, field: i32) -> f64;
}

unsafe fn create_matching_dict(page: i32, usage: i32) -> CFDictionaryRef {
    unsafe {
        let key_page = CFStringCreateWithCString(ptr::null(), b"PrimaryUsagePage\0".as_ptr() as *const i8, kCFStringEncodingUTF8);
        let key_usage = CFStringCreateWithCString(ptr::null(), b"PrimaryUsage\0".as_ptr() as *const i8, kCFStringEncodingUTF8);

        let val_page = CFNumberCreate(ptr::null(), kCFNumberIntType, &page as *const _ as *const c_void);
        let val_usage = CFNumberCreate(ptr::null(), kCFNumberIntType, &usage as *const _ as *const c_void);

        let keys: Vec<*const c_void> = vec![key_page as _, key_usage as _];
        let values: Vec<*const c_void> = vec![val_page as _, val_usage as _];

        let dict = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            2,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );

        // Release temp objects (dict retains them)
        CFRelease(key_page as _);
        CFRelease(key_usage as _);
        CFRelease(val_page as _);
        CFRelease(val_usage as _);

        dict
    }
}

pub struct LightSensor{
    client: CFTypeRef,
    dict: CFDictionaryRef,
    services: CFArrayRef,
    service: *const c_void,
}

#[derive(Error, Debug, Copy, Clone)]
pub enum LightSensorCreateError{
    #[error("Failed to create IOHID Event Client")]
    FailCreateClient,
    #[error("Failed to find ALS")]
    FailToGetALS,
}

impl LightSensor{
    pub fn new() -> Result<Self, LightSensorCreateError>{
        unsafe{
            let client = IOHIDEventSystemClientCreate(ptr::null());
            if client.is_null(){
                return Err(LightSensorCreateError::FailCreateClient);
            }

            let dict = create_matching_dict(HID_PAGE_APPLE_VENDOR, HID_USAGE_APPLE_ALS);
            IOHIDEventSystemClientSetMatching(client, dict);

            let services = IOHIDEventSystemClientCopyServices(client) as CFArrayRef;

            if services.is_null() || CFArrayGetCount(services) == 0{
                return Err(LightSensorCreateError::FailToGetALS);
            }

            let service = CFArrayGetValueAtIndex(services, 0);

            Ok(Self{
                client,
                dict,
                services,
                service,
            })
        }
    }

    pub fn value(&self) -> Option<f64>{
        unsafe {
            let event = IOHIDServiceClientCopyEvent(self.service, K_IOHID_EVENT_TYPE_ALS, 0, 0);

            if !event.is_null(){
                let value = IOHIDEventGetFloatValue(event, K_IOHID_EVENT_FIELD_ALS_LEVEL);
                CFRelease(event);
                Some(value)
            }else {
                None
            }
        }
    }
}

impl Drop for LightSensor{
    fn drop(&mut self) {
        unsafe{
            CFRelease(self.services as _);
            CFRelease(self.dict as _);
            CFRelease(self.client as _);
        }
    }
}