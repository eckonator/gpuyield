//! The symbol in the menu bar and its menu. The menu is built anew after every change; a click is
//! handled on the main thread after the menu has closed, the switching itself in the background
//! (the password dialog may stay open for a while).

use std::cell::{Cell, RefCell};
use std::time::Duration;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::i18n::tr;
use crate::model::{self, Gpu, Mode};
use crate::{dispatch, gpu, log, privileged};

/// macOS may switch a moment after `pmset`, and nothing reports a change of the setting itself.
const POLL_EVERY: Duration = Duration::from_secs(15);
const ICON_SIZE: u32 = 36;

struct App {
    tray: TrayIcon,
    mode: Option<Mode>,
    gpu: Option<Gpu>,
    external: usize,
    passwordless: bool,
    /// A switch or the rule is being changed in the background.
    busy: bool,
    error: Option<String>,
    /// The symbol shown: `Some(true)` with the discrete GPU on.
    hot: Option<bool>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static REFRESH_PENDING: Cell<bool> = const { Cell::new(false) };
}

const MODE: &str = "mode:";
const PASSWORDLESS: &str = "passwordless";
const AUTOSTART: &str = "autostart";
const QUIT: &str = "quit";

/// Puts the symbol into the menu bar (on the main thread, once the app runs).
pub fn start() {
    let tray = icon(false).and_then(|icon| {
        TrayIconBuilder::new()
            .with_icon_templated(icon)
            .with_tooltip("GPUYield")
            .build()
            .map_err(|e| e.to_string())
    });
    let tray = match tray {
        Ok(tray) => tray,
        Err(e) => {
            log!("no menu bar symbol: {e}");
            std::process::exit(1);
        }
    };
    let app = App {
        tray,
        mode: None,
        gpu: None,
        external: 0,
        passwordless: false,
        busy: false,
        error: None,
        hot: None,
    };
    APP.set(Some(app));
    refresh();
    APP.with_borrow(|app| {
        if let Some(app) = app {
            let gpu = app.gpu.as_ref().map_or("unknown", |g| g.name.as_str());
            log!("mode {:?}, graphics card {gpu}", app.mode);
        }
    });
    MenuEvent::set_event_handler(Some(|event: MenuEvent| {
        let id = event.id.0;
        dispatch::later(move || clicked(&id));
    }));
    dispatch::after(POLL_EVERY, poll);
}

fn icon(hot: bool) -> Result<Icon, String> {
    Icon::from_rgba(crate::icon::menu_bar(ICON_SIZE, hot), ICON_SIZE, ICON_SIZE).map_err(|e| e.to_string())
}

fn poll() {
    refresh();
    dispatch::after(POLL_EVERY, poll);
}

/// After displays changed. macOS reports a change in several steps, so the menu is built now and
/// once more shortly after.
pub fn refresh_soon() {
    if REFRESH_PENDING.replace(true) {
        return;
    }
    dispatch::later(|| {
        REFRESH_PENDING.set(false);
        refresh();
    });
    dispatch::after(Duration::from_millis(1500), refresh);
}

fn refresh() {
    let (mode, gpu, external, passwordless) = (
        gpu::mode(),
        gpu::active(),
        gpu::external_monitors(),
        privileged::passwordless(),
    );
    APP.with_borrow_mut(|app| {
        let Some(app) = app else { return };
        if app.gpu.as_ref().map(|g| &g.name) != gpu.as_ref().map(|g| &g.name) && app.gpu.is_some() {
            log!(
                "graphics card now: {}",
                gpu.as_ref().map_or("unknown", |g| g.name.as_str())
            );
        }
        app.mode = mode;
        app.gpu = gpu;
        app.external = external;
        app.passwordless = passwordless;
        show(app);
    });
}

/// Puts the menu, the symbol and the tooltip in line with the state.
fn show(app: &mut App) {
    app.tray.set_menu(Some(Box::new(build(app))));
    let hot = app.gpu.as_ref().is_some_and(|g| !g.integrated);
    if app.hot != Some(hot)
        && let Ok(icon) = icon(hot)
    {
        let _ = app.tray.set_icon_templated(Some(icon));
        let tooltip = app
            .gpu
            .as_ref()
            .map_or("GPUYield".to_string(), |g| format!("GPUYield – {}", g.name));
        let _ = app.tray.set_tooltip(Some(tooltip));
        app.hot = Some(hot);
    }
}

fn with_app(f: impl FnOnce(&mut App)) {
    APP.with_borrow_mut(|app| {
        if let Some(app) = app {
            f(app);
            show(app);
        }
    });
}

fn label(mode: Mode) -> &'static str {
    match mode {
        Mode::Integrated => tr("Intel only (cool and quiet)"),
        Mode::Automatic => tr("Automatic"),
        Mode::Discrete => tr("NVIDIA only (full power)"),
    }
}

fn build(app: &App) -> Menu {
    let menu = Menu::new();
    let separator = PredefinedMenuItem::separator;
    let info = |text: String| MenuItem::new(text, false, None);
    let Some(mode) = app.mode else {
        let _ = menu.append(&info(tr("This Mac has only one graphics card").to_string()));
        let _ = menu.append(&separator());
        append_footer(&menu, app);
        return menu;
    };
    let gpu = app.gpu.as_ref().map_or(tr("unknown"), |g| g.name.as_str());
    let _ = menu.append(&info(format!("{} {gpu}", tr("Active:"))));
    let _ = menu.append(&separator());
    for m in Mode::ALL {
        // A click on the mode that is set does nothing
        let item = CheckMenuItem::with_id(format!("{MODE}{}", m.value()), label(m), !app.busy, m == mode, None);
        let _ = menu.append(&item);
    }
    if app.busy {
        let _ = menu.append(&info(tr("Switching …").to_string()));
    } else if let Some(e) = &app.error {
        let _ = menu.append(&info(format!("{} {e}", tr("Not switched:"))));
    } else if model::mismatch(mode, app.gpu.as_ref()) {
        let _ = menu.append(&info(
            tr("Not switched yet – log out and in again if it stays").to_string(),
        ));
    }
    if mode == Mode::Integrated {
        let _ = menu.append(&info(tr("External monitors need the NVIDIA").to_string()));
    } else if app.external > 0 {
        let _ = menu.append(&info(tr("External monitor connected: keep the NVIDIA").to_string()));
    }
    let _ = menu.append(&separator());
    let passwordless = CheckMenuItem::with_id(
        PASSWORDLESS,
        tr("Switch without password"),
        !app.busy,
        app.passwordless,
        None,
    );
    let _ = menu.append(&passwordless);
    append_footer(&menu, app);
    menu
}

fn append_footer(menu: &Menu, _app: &App) {
    let autostart = CheckMenuItem::with_id(AUTOSTART, tr("Start at login"), true, crate::autostart::enabled(), None);
    let _ = menu.append(&autostart);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let version = MenuItem::new(format!("GPUYield {}", crate::version()), false, None);
    let quit = MenuItem::with_id(QUIT, tr("Quit GPUYield"), true, None);
    let _ = menu.append_items(&[&version, &quit]);
}

/// Runs `job` in the background with the menu marked busy, then shows its result.
fn in_background(what: &'static str, job: impl FnOnce() -> Result<(), String> + Send + 'static) {
    with_app(|app| {
        app.busy = true;
        app.error = None;
    });
    let spawned = std::thread::Builder::new().name(what.into()).spawn(move || {
        let result = job();
        dispatch::later(move || done(what, result));
    });
    if let Err(e) = spawned {
        log!("{what} not started: {e}");
        with_app(|app| app.busy = false);
    }
}

fn done(what: &str, result: Result<(), String>) {
    with_app(|app| {
        app.busy = false;
        app.error = match result {
            Ok(()) => None,
            Err(e) if e == privileged::CANCELLED => {
                log!("{what}: cancelled");
                None
            }
            Err(e) => {
                log!("{what} failed: {e}");
                Some(e)
            }
        };
    });
    refresh();
    // The card may follow a few seconds later
    dispatch::after(Duration::from_secs(4), refresh);
}

fn clicked(id: &str) {
    let Some((mode, passwordless, busy)) =
        APP.with_borrow(|app| app.as_ref().map(|a| (a.mode, a.passwordless, a.busy)))
    else {
        return;
    };
    match id {
        PASSWORDLESS if !busy => in_background("rule", move || privileged::set_passwordless(!passwordless)),
        AUTOSTART => {
            let enable = !crate::autostart::enabled();
            if let Err(e) = crate::autostart::set(enable) {
                log!("start at login not changed: {e}");
            }
        }
        QUIT => crate::quit(),
        _ => {
            let new = id
                .strip_prefix(MODE)
                .and_then(|v| v.parse().ok())
                .and_then(Mode::from_value);
            if let Some(new) = new
                && !busy
                && Some(new) != mode
            {
                log!("switching to {new:?}");
                in_background("switch", move || privileged::set_mode(new));
                return;
            }
        }
    }
    // Also puts back the check mark macOS moved on a click that changed nothing
    refresh();
}
