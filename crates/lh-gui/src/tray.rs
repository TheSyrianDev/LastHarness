//! A tray icon (StatusNotifierItem) so the app can be reached when its window is hidden or closed.
//! Where the desktop has no tray, registering fails quietly and the app runs without one.

struct Tray;

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "lastharness-gui".into()
    }
    fn title(&self) -> String {
        "LastHarness".into()
    }
    fn icon_name(&self) -> String {
        // the brand app icon, installed under the hicolor theme as "lastharness"
        "lastharness".into()
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![StandardItem { label: "Quit".into(), activate: Box::new(|_| std::process::exit(0)), ..Default::default() }.into()]
    }
}

pub fn spawn() {
    use ksni::blocking::TrayMethods;
    if std::env::var_os("LH_NO_TRAY").is_some() {
        return;
    }
    match Tray.spawn() {
        Ok(handle) => std::mem::forget(handle),
        Err(e) => eprintln!("no tray icon: {e}"),
    }
}
