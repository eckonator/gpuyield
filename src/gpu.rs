//! Which graphics card draws the picture now, which mode `pmset` is set to, and being told when
//! the displays change (a switch of the graphics card reconfigures them, too).

use std::ffi::c_void;

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

use crate::model::{self, Gpu, Mode};

type ReconfigurationCallback = extern "C" fn(display: u32, flags: u32, user_info: *mut c_void);

/// `kCGDisplayBeginConfigurationFlag`: the change is about to happen.
const BEGIN_CONFIGURATION: u32 = 1;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGGetActiveDisplayList(max: u32, ids: *mut u32, count: *mut u32) -> i32;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayIsBuiltin(display: u32) -> bool;
    fn CGDisplayRegisterReconfigurationCallback(callback: ReconfigurationCallback, user_info: *mut c_void) -> i32;
    /// Returns a retained `id<MTLDevice>`, or nil.
    fn CGDirectDisplayCopyCurrentMetalDevice(display: u32) -> *mut AnyObject;
}

// The Metal device classes come from here
#[link(name = "Metal", kind = "framework")]
unsafe extern "C" {}

fn displays() -> Vec<u32> {
    let mut ids = [0u32; 32];
    let mut count = 0u32;
    if unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) } != 0 {
        return Vec::new();
    }
    ids[..count as usize].to_vec()
}

/// External monitors with a picture.
pub fn external_monitors() -> usize {
    displays()
        .into_iter()
        .filter(|&id| !unsafe { CGDisplayIsBuiltin(id) })
        .count()
}

/// The graphics card of the built-in screen (with the lid closed: of the main display).
pub fn active() -> Option<Gpu> {
    let display = displays()
        .into_iter()
        .find(|&id| unsafe { CGDisplayIsBuiltin(id) })
        .unwrap_or_else(|| unsafe { CGMainDisplayID() });
    unsafe {
        let device = Retained::from_raw(CGDirectDisplayCopyCurrentMetalDevice(display))?;
        let name: Retained<NSString> = msg_send![&*device, name];
        let integrated: bool = msg_send![&*device, isLowPower];
        Some(Gpu {
            name: name.to_string(),
            integrated,
        })
    }
}

/// The mode `pmset` is set to; `None` on Macs with only one graphics card.
pub fn mode() -> Option<Mode> {
    let output = std::process::Command::new(model::PMSET).arg("-g").output().ok()?;
    model::parse_pmset(&String::from_utf8_lossy(&output.stdout))
}

/// Calls `on_change` on the main thread after displays came, went or changed their graphics card.
pub fn watch(on_change: fn()) {
    extern "C" fn callback(_display: u32, flags: u32, user_info: *mut c_void) {
        if flags & BEGIN_CONFIGURATION != 0 {
            return;
        }
        let on_change: fn() = unsafe { std::mem::transmute(user_info) };
        on_change();
    }
    unsafe { CGDisplayRegisterReconfigurationCallback(callback, on_change as *mut c_void) };
}
