//! Web and phone, under the web settings: opening a pairing window, accepting a device by its PIN, the list of
//! paired devices and removing one.

use gpui::prelude::*;
use gpui::*;
use serde_json::json;

use super::model::web as m;
use crate::app::App;
use crate::ui::Kind;
use lh_gui::setting_names::Category;

impl App {
    pub(super) fn pg_web(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2().pt_2();
        col = col.child(self.pg_phone(cx));
        col = col.child(self.pg_title("Paired devices")).child(self.pg_muted("A browser or phone can use this app over the address above only after you pair it here. Open pairing for a few minutes, then on the other device open the address and press Pair. It shows a PIN: type that PIN below."));
        if let Some(n) = self.pg_note_for("web") {
            col = col.child(n);
        }
        let mut open_row = div().flex().flex_wrap().gap_1();
        for minutes in [5u64, 10] {
            open_row = open_row.child(self.button(format!("pg-dev-open-{minutes}"), format!("Open pairing for {minutes} minutes"), Kind::Normal, cx, move |a, _, cx| {
                a.pg_act("web", "dev-open:", "POST", "/v1/devices/open", Some(json!({"minutes": minutes})), cx);
                a.pg_poll_pending(minutes, cx);
            }));
        }
        col = col.child(open_row);
        let pin = self.xp_field("dev.pin", "PIN shown on the other device", "123456", window, cx);
        let accept = self.button("pg-dev-accept", "Pair this device", Kind::Primary, cx, |a, _, cx| {
            let raw = a.pg_text("dev.pin", cx);
            match m::clean_pin(&raw) {
                Ok(pin) => {
                    a.pg_act("web", "dev-accept:", "POST", "/v1/devices/accept", Some(json!({"pin": pin})), cx);
                    a.pg_clear(&["dev.pin"]);
                }
                Err(e) => {
                    a.pg_note_set("web", true, e);
                    cx.notify();
                }
            }
        });
        col = col.child(self.xp_card(vec![pin, div().flex().child(accept).into_any_element()]));

        if let Some(Ok(v)) = self.pg_list("pending") {
            if let Some(f) = m::fingerprint(&v) {
                col = col.child(self.pg_muted(format!("This computer's certificate fingerprint, to compare with what the other device shows: {f}")));
            }
            for w in m::waiting(&v) {
                col = col.child(self.xp_card(vec![self.pg_title(format!("Waiting to pair: {}", w.name)), self.pg_muted(format!("Its key starts {}. Waiting {} seconds.", w.key, w.secs))]));
            }
        }
        match self.pg_list("devices") {
            None => {}
            Some(Err(e)) => col = col.child(self.pg_error_line("The device list", &e)),
            Some(Ok(v)) => {
                let list = m::devices(&v);
                if list.is_empty() {
                    col = col.child(self.pg_note_line("No device is paired."));
                }
                for d in list {
                    let (id, name) = (d.id.clone(), d.name.clone());
                    let remove = self.pg_confirm_button(&format!("dev-rm-{id}"), "Remove", "Yes, remove it", cx, {
                        let id = id.clone();
                        move |a, _, cx| a.pg_act("web", "dev-remove:", "DELETE", format!("/v1/devices/{id}"), None, cx)
                    });
                    col = col.child(
                        self.xp_card(vec![
                            div().flex().flex_wrap().items_center().gap_2().child(div().flex_1().child(name.clone())).child(remove).into_any_element(),
                            self.pg_muted(m::last_seen_words(d.last_seen)),
                        ])
                        .id(SharedString::from(format!("pg-dev-{id}")))
                        .role(Role::Group)
                        .aria_label(format!("Device {name}")),
                    );
                }
            }
        }
        col.child(div().flex().child(self.pg_refresh(Category::Web, cx))).into_any_element()
    }

    /// Phone: the sign-in link for a phone on the tailnet, as a QR code to scan and as text.
    fn pg_phone(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2().pb_2().child(self.pg_title("Phone"));
        let info = match self.pg_list("phone") {
            None => return col.into_any_element(),
            Some(Err(_)) => return col.child(self.pg_muted("Turn on \"Reach it from your phone over Tailscale\" above, then restart the app. A sign-in link for your phone appears here.")).into_any_element(),
            Some(Ok(v)) => v,
        };
        let Some(link) = info["link"].as_str().map(str::to_string) else {
            let why = if info["enabled"] == true { "The core is not listening on an address a phone can reach. Check that Tailscale is running on this computer, then restart the app." } else { "Turn on \"Reach it from your phone over Tailscale\" above, then restart the app. A sign-in link for your phone appears here." };
            return col.child(self.pg_muted(why)).into_any_element();
        };
        col = col.child(self.pg_muted("Scan this with your phone's camera. The phone must be signed in to Tailscale too. Its browser warns about the certificate once: check the fingerprint below, then continue. The phone stays signed in and shows up under Paired devices."));
        if let Some(rows) = lh_client::qr::modules(&link) {
            // always dark on white, whatever the theme, so every camera reads it
            let mut grid = div().flex().flex_col().bg(white()).p_1().rounded(px(6.)).id("pg-phone-qr").role(Role::Image).aria_label("QR code with the sign-in link for your phone");
            for row in rows {
                let mut r = div().flex();
                for dark in row {
                    r = r.child(div().size(px(5.)).when(dark, |d| d.bg(black())));
                }
                grid = grid.child(r);
            }
            col = col.child(div().flex().child(grid));
        }
        let copy_link = link.clone();
        let copy = self.button("pg-phone-copy", "Copy link", Kind::Normal, cx, move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy_link.clone())));
        let new = self.pg_confirm_button("phone-new", "Make a new link", "Yes, replace the link", cx, |a, _, cx| a.pg_act("web", "phone-new:", "POST", "/v1/devices/phone/rotate", None, cx));
        col = col.child(div().flex().flex_wrap().gap_1().child(copy).child(new));
        if let Some(f) = info["fingerprint"].as_str() {
            col = col.child(self.pg_muted(format!("Certificate fingerprint: {f}")));
        }
        col.child(self.pg_muted("Keep the link private: anyone with it on your tailnet can sign in. Make a new link if it was shared by mistake.")).into_any_element()
    }

    /// While pairing is open, look for a device that is waiting every few seconds, so it appears without a click.
    fn pg_poll_pending(&mut self, minutes: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
            for _ in 0..(minutes * 60 / 3).min(100) {
                cx.background_executor().timer(std::time::Duration::from_secs(3)).await;
                let keep = this.update(cx, |a, cx| {
                    let on_page = a.settings_open && a.settings_page == Category::Web;
                    if on_page {
                        a.pg_fetch("pending", "/v1/devices/pending");
                        cx.notify();
                    }
                    a.settings_open
                });
                if !matches!(keep, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }
}
