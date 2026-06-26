use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::CFString;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
}

pub fn is_macos_locked() -> bool {
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