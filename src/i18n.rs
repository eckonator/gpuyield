//! Menu texts in English (the keys) and German, chosen by the first preferred system language.

use std::sync::OnceLock;

use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation_sys::base::CFRelease;
use core_foundation_sys::locale::CFLocaleCopyPreferredLanguages;
use core_foundation_sys::string::{CFStringGetCString, CFStringRef, kCFStringEncodingUTF8};

fn german() -> bool {
    static GERMAN: OnceLock<bool> = OnceLock::new();
    *GERMAN.get_or_init(|| {
        if let Ok(lang) = std::env::var("GY_LANG") {
            return lang.starts_with("de");
        }
        first_language().is_some_and(|l| l.starts_with("de"))
    })
}

fn first_language() -> Option<String> {
    unsafe {
        let languages = CFLocaleCopyPreferredLanguages();
        if languages.is_null() {
            return None;
        }
        let mut text = None;
        if CFArrayGetCount(languages) > 0 {
            let first = CFArrayGetValueAtIndex(languages, 0) as CFStringRef;
            let mut buf = [0i8; 32];
            if CFStringGetCString(first, buf.as_mut_ptr(), buf.len() as _, kCFStringEncodingUTF8) != 0 {
                text = Some(std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned());
            }
        }
        CFRelease(languages.cast());
        text
    }
}

pub fn tr(key: &'static str) -> &'static str {
    if !german() {
        return key;
    }
    GERMAN.iter().find(|(en, _)| *en == key).map_or(key, |(_, de)| de)
}

const GERMAN: &[(&str, &str)] = &[
    ("Active:", "Aktiv:"),
    ("unknown", "unbekannt"),
    ("Intel only (cool and quiet)", "Nur Intel (kühl und leise)"),
    ("Automatic", "Automatisch"),
    ("NVIDIA only (full power)", "Nur NVIDIA (volle Leistung)"),
    ("Switching …", "Wird umgeschaltet …"),
    ("Not switched:", "Nicht umgeschaltet:"),
    (
        "Not switched yet – log out and in again if it stays",
        "Noch nicht umgeschaltet – falls es so bleibt: ab- und wieder anmelden",
    ),
    (
        "External monitors need the NVIDIA",
        "Externe Monitore brauchen die NVIDIA",
    ),
    (
        "External monitor connected: keep the NVIDIA",
        "Externer Monitor angeschlossen: NVIDIA nicht abschalten",
    ),
    ("Switch without password", "Ohne Passwort umschalten"),
    ("Start at login", "Beim Anmelden starten"),
    ("Quit GPUYield", "GPUYield beenden"),
    (
        "This Mac has only one graphics card",
        "Dieser Mac hat nur eine Grafikkarte",
    ),
    (
        "GPUYield wants to switch the graphics card.",
        "GPUYield möchte die Grafikkarte umschalten.",
    ),
    (
        "GPUYield wants to allow switching the graphics card without a password.",
        "GPUYield möchte das Umschalten der Grafikkarte ohne Passwort erlauben.",
    ),
    (
        "GPUYield wants to remove its sudoers rule.",
        "GPUYield möchte seine sudoers-Regel entfernen.",
    ),
];

#[cfg(test)]
mod tests {
    use super::GERMAN;

    /// Every text passed to `tr` in the sources has a German text (also calls over several lines).
    #[test]
    fn every_text_is_translated() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let source = std::fs::read_to_string(&path).unwrap();
            let compact: String = source.split_whitespace().collect::<Vec<_>>().join(" ");
            for part in compact.split("tr(\"").skip(1) {
                let key = &part[..part.find('"').unwrap()];
                if !GERMAN.iter().any(|(en, _)| *en == key) {
                    missing.push(key.to_string());
                }
            }
        }
        assert!(missing.is_empty(), "missing German texts: {missing:?}");
    }
}
