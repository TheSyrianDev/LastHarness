//! The person's own looks, as folders: copying one out, bringing one in, renaming it, removing it, and the file
//! picker each of those needs. The file work itself is in `lh_gui::theme`; this connects it to the window and keeps
//! the look in use sensible after a folder changes under it.

use gpui::*;

use crate::app::App;
use lh_gui::theme::{self, Origin};

impl App {
    /// After a look folder was copied, renamed or removed: read the folder again, keep (or re-find) the look in use,
    /// and repaint with it.
    pub fn looks_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.themes.reload();
        if !self.themes.families().iter().any(|f| *f == self.prefs.theme) {
            // the family is gone (removed or renamed): the first look takes over rather than a blank window
            self.prefs.theme = String::new();
        }
        self.active_theme = self.themes.resolve(&self.prefs.theme, self.dark());
        self.refresh_pal(cx);
        self.apply_window(window);
        self.persist();
        cx.notify();
    }

    /// Where the look of this family came from, and its picture when that picture lives outside the folder.
    fn look_origin(&self, family: &str) -> Option<(Origin, Option<std::path::PathBuf>)> {
        let t = &self.themes.themes[self.themes.resolve(family, self.dark())];
        let origin = self.themes.origin(&t.label())?.clone();
        let folder = origin.folder().map(|p| p.to_path_buf());
        let image = t.background.image.clone().filter(|img| match &folder {
            Some(f) => !img.starts_with(f),
            None => true,
        });
        Some((origin, image))
    }

    /// A name in the themes folder that is free, starting from `wanted` and adding a number.
    fn free_look_name(&self, wanted: &str) -> String {
        let base = theme::clean_look_name(wanted).unwrap_or_else(|_| "Look".into());
        let mut name = base.clone();
        let mut n = 2;
        while self.cfg.themes.join(&name).exists() {
            name = format!("{base} {n}");
            n += 1;
        }
        name
    }

    /// Copy this look into a folder of its own, so it can be edited. A compiled-in look is written from its own file.
    pub fn duplicate_look(&mut self, family: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some((origin, image)) = self.look_origin(family) else {
            self.look_note = Some("That look has nowhere to be copied from.".into());
            return cx.notify();
        };
        let name = self.free_look_name(&format!("{family} copy"));
        match theme::make_folder(&self.cfg.themes, &name, &origin, image.as_deref()) {
            Ok(_) => {
                self.prefs.theme = name.clone();
                self.look_note = Some(format!("{name} is yours now. Edit its theme.toml, or put a background.png beside it."));
                self.looks_changed(window, cx);
            }
            Err(e) => {
                self.look_note = Some(e);
                cx.notify();
            }
        }
    }

    /// Write this look out as a folder in a place the person picks. Works for every look, compiled in or not.
    pub fn export_look(&mut self, family: &str, cx: &mut Context<Self>) {
        let Some((origin, image)) = self.look_origin(family) else { return };
        let rx = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Export the look to".into()) });
        let name = family.to_string();
        cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
            if let Ok(Ok(Some(paths))) = rx.await {
                if let Some(parent) = paths.first() {
                    let _ = this.update(cx, |a, cx| {
                        a.look_note = Some(match theme::make_folder(parent, &name, &origin, image.as_deref()) {
                            Ok(p) => format!("Wrote {}.", p.display()),
                            Err(e) => e,
                        });
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Bring a look in: a folder with a theme.toml in it, or a single .toml file.
    pub fn import_look(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions { files: true, directories: true, multiple: false, prompt: Some("Import a look folder, or a theme.toml".into()) });
        cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| {
            if let Ok(Ok(Some(paths))) = rx.await {
                if let Some(path) = paths.first() {
                    let _ = this.update_in(cx, |a, window, cx| {
                        let dir = a.cfg.themes.clone();
                        a.look_note = Some(match theme::import_look(&dir, path) {
                            Ok(name) => {
                                a.prefs.theme = name.clone();
                                a.looks_changed(window, cx);
                                format!("{name} is in your looks.")
                            }
                            Err(e) => e,
                        });
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Start renaming a look folder: the name box opens with the name in it.
    pub fn begin_rename_look(&mut self, family: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.look_rename = Some(family.to_string());
        self.look_remove_armed = None;
        let name = family.to_string();
        self.look_name.update(cx, |s, cx| s.set_value(name, window, cx));
        cx.notify();
    }

    /// Take the typed name: the folder is renamed and the look in use follows it.
    pub fn commit_rename_look(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(old) = self.look_rename.clone() else { return };
        let typed = self.look_name.read(cx).value().to_string();
        match theme::rename_folder(&self.cfg.themes, &old, &typed) {
            Ok(new) => {
                if self.prefs.theme == old {
                    self.prefs.theme = new.clone();
                }
                self.look_rename = None;
                self.look_note = Some(format!("Renamed to {new}."));
                self.looks_changed(window, cx);
            }
            Err(e) => {
                self.look_note = Some(e);
                cx.notify();
            }
        }
    }

    pub fn cancel_rename_look(&mut self, cx: &mut Context<Self>) {
        self.look_rename = None;
        cx.notify();
    }

    /// Remove a look folder. The first click arms it, the second removes: a folder with someone's colours in it is
    /// not something to lose to a stray click.
    pub fn remove_look(&mut self, family: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.look_remove_armed.as_deref() != Some(family) {
            self.look_remove_armed = Some(family.to_string());
            cx.notify();
            return;
        }
        match theme::remove_folder(&self.cfg.themes, family) {
            Ok(()) => {
                self.look_remove_armed = None;
                if self.prefs.theme == family {
                    self.prefs.theme = String::new();
                }
                self.look_note = Some(format!("{family} is gone from the looks folder."));
                self.looks_changed(window, cx);
            }
            Err(e) => {
                self.look_remove_armed = None;
                self.look_note = Some(e);
                cx.notify();
            }
        }
    }

    /// Open the themes folder in the file manager, so a look can be edited with anything.
    pub fn show_looks_folder(&mut self, cx: &mut Context<Self>) {
        let dir = self.cfg.themes.clone();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::process::Command::new("xdg-open").arg(&dir).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
        cx.notify();
    }
}
