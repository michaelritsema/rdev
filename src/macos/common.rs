#![allow(clippy::upper_case_acronyms)]
use crate::macos::keyboard::Keyboard;
use crate::rdev::{Button, Event, EventType};
use cocoa::base::id;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, EventField};
use lazy_static::lazy_static;
use std::convert::TryInto;
use std::os::raw::c_void;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::macos::keycodes::key_from_code;

pub type CFMachPortRef = *const c_void;
pub type CFIndex = u64;
pub type CFAllocatorRef = id;
pub type CFRunLoopSourceRef = id;
pub type CFRunLoopRef = id;
pub type CFRunLoopMode = id;
pub type CGEventTapProxy = id;
pub type CGEventRef = CGEvent;

// https://developer.apple.com/documentation/coregraphics/cgeventtapplacement?language=objc
pub type CGEventTapPlacement = u32;
#[allow(non_upper_case_globals)]
pub const kCGHeadInsertEventTap: u32 = 0;

// https://developer.apple.com/documentation/coregraphics/cgeventtapoptions?language=objc
#[allow(non_upper_case_globals)]
#[repr(u32)]
pub enum CGEventTapOption {
    #[cfg(feature = "unstable_grab")]
    Default = 0,
    ListenOnly = 1,
}

pub static mut LAST_FLAGS: CGEventFlags = CGEventFlags::CGEventFlagNull;
lazy_static! {
    pub static ref KEYBOARD_STATE: Mutex<Keyboard> = Mutex::new(Keyboard::new().unwrap());
}

// https://developer.apple.com/documentation/coregraphics/cgeventmask?language=objc
pub type CGEventMask = u64;
#[allow(non_upper_case_globals)]
pub const kCGEventMaskForAllEvents: u64 = (1 << CGEventType::LeftMouseDown as u64)
    + (1 << CGEventType::LeftMouseUp as u64)
    + (1 << CGEventType::RightMouseDown as u64)
    + (1 << CGEventType::RightMouseUp as u64)
    + (1 << CGEventType::MouseMoved as u64)
    + (1 << CGEventType::LeftMouseDragged as u64)
    + (1 << CGEventType::RightMouseDragged as u64)
    + (1 << CGEventType::OtherMouseDown as u64)
    + (1 << CGEventType::OtherMouseUp as u64)
    + (1 << CGEventType::KeyDown as u64)
    + (1 << CGEventType::KeyUp as u64)
    + (1 << CGEventType::FlagsChanged as u64)
    + (1 << CGEventType::ScrollWheel as u64);

#[cfg(target_os = "macos")]
#[link(name = "Cocoa", kind = "framework")]
extern "C" {
    #[allow(improper_ctypes)]
    pub fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOption,
        eventsOfInterest: CGEventMask,
        callback: QCallback,
        user_info: id,
    ) -> CFMachPortRef;
    pub fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        tap: CFMachPortRef,
        order: CFIndex,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFRunLoopMode);
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CFRunLoopRun();

    pub static kCFRunLoopCommonModes: CFRunLoopMode;

}
pub type QCallback = unsafe extern "C" fn(
    proxy: CGEventTapProxy,
    _type: CGEventType,
    cg_event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

pub unsafe fn convert(
    _type: CGEventType,
    cg_event: &CGEvent,
    keyboard_state: &mut Keyboard,
) -> Option<Event> {
    /*
        pub struct CGEventFlags: u64 {
        const CGEventFlagNull = 0;

        // Device-independent modifier key bits.
        const CGEventFlagAlphaShift = 0x00010000;
        const CGEventFlagShift = 0x00020000;
        const CGEventFlagControl = 0x00040000;
        const CGEventFlagAlternate = 0x00080000;
        const CGEventFlagCommand = 0x00100000;

        // Special key identifiers.
        const CGEventFlagHelp = 0x00400000;
        const CGEventFlagSecondaryFn = 0x00800000;

        // Identifies key events from numeric keypad area on extended keyboards.
        const CGEventFlagNumericPad = 0x00200000;

        // Indicates if mouse/pen movement events are not being coalesced
        const CGEventFlagNonCoalesced = 0x00000100;
    }
     */
  
    let flags = cg_event.get_flags().bits();

    /*  
    if !cg_event.get_flags().is_empty() { 
        if cg_event.get_flags().contains(CGEventFlags::CGEventFlagShift) {
            println!("Shift was down! :{:?}, {:?}", flags,cg_event.get_flags());

        }
    }
    */
    
    let option_type = match _type {
        CGEventType::LeftMouseDown => Some(EventType::ButtonPress(Button::Left)),
        CGEventType::LeftMouseUp => Some(EventType::ButtonRelease(Button::Left)),
        CGEventType::RightMouseDown => Some(EventType::ButtonPress(Button::Right)),
        CGEventType::RightMouseUp => Some(EventType::ButtonRelease(Button::Right)),
        CGEventType::OtherMouseDown => { 
            let button_id = cg_event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
            Some(EventType::ButtonPress(Button::from_integer_value(button_id)))
        },
        CGEventType::OtherMouseUp => { 
            let button_id = cg_event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
            Some(EventType::ButtonRelease(Button::from_integer_value(button_id)))
        }
        CGEventType::MouseMoved => {
            let point = cg_event.location();
            Some(EventType::MouseMove {
                x: point.x,
                y: point.y,
            })
        }
        CGEventType::KeyDown => {
            let code = cg_event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            Some(EventType::KeyPress(key_from_code(code.try_into().ok()?)))
        }
        CGEventType::KeyUp => {
            let code = cg_event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            Some(EventType::KeyRelease(key_from_code(code.try_into().ok()?)))
        }
        CGEventType::FlagsChanged => {
            let code = cg_event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            let code = code.try_into().ok()?;
            let flags = cg_event.get_flags();
            if flags < LAST_FLAGS {
                LAST_FLAGS = flags;
                Some(EventType::KeyRelease(key_from_code(code)))
            } else {
                LAST_FLAGS = flags;
                Some(EventType::KeyPress(key_from_code(code)))
            }
        }
        CGEventType::ScrollWheel => {
            let delta_y =
                cg_event.get_integer_value_field(EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1);
            let delta_x =
                cg_event.get_integer_value_field(EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2);
            Some(EventType::Wheel { delta_x, delta_y })
        }
        _ => None,
    };
    if let Some(event_type) = option_type {
        let name = match event_type {
            EventType::KeyPress(_) => {
                let code =
                    cg_event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u32;
                let flags = cg_event.get_flags();
                keyboard_state.create_string_for_key(code, flags)
            }
            _ => None,
        };
        return Some(Event {
            event_type,
            time: SystemTime::now(),
            name,
            flags
        });
    }
    None
}
