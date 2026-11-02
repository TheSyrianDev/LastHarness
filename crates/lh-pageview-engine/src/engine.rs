//! The engine itself: one document, loaded, resized, scrolled, painted and pointed at. It runs only inside the helper
//! process (`bin/lh-pageview-helper.rs`); the app never loads Blitz into its own process.

use std::sync::Arc;

use anyrender::render_to_buffer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::DocumentConfig;
use blitz_html::HtmlDocument;
use blitz_traits::shell::{ColorScheme, Viewport};

use lh_pageview::{resolve_link, scroll, Frame, Info, Link, BASE};

use crate::net::{Deny, NetStats, NoNavigation};

pub struct Engine {
    doc: HtmlDocument,
    pub stats: Arc<NetStats>,
    w: u32,
    h: u32,
    scale: f32,
}

fn viewport(w: u32, h: u32, scale: f32) -> Viewport {
    // a page is drawn in the light scheme and in its own colours: the app does not recolour other people's pages
    Viewport::new(w.max(1), h.max(1), scale.clamp(0.5, 4.0), ColorScheme::Light)
}

impl Engine {
    pub fn load(html: &str, w: u32, h: u32, scale: f32) -> Engine {
        let stats = Arc::new(NetStats::default());
        let cfg = DocumentConfig {
            viewport: Some(viewport(w, h, scale)),
            base_url: Some(BASE.to_string()),
            net_provider: Some(Arc::new(Deny::new(stats.clone()))),
            navigation_provider: Some(Arc::new(NoNavigation)),
            ..Default::default()
        };
        let mut doc = HtmlDocument::from_html(html, cfg);
        doc.resolve(0.0);
        Engine { doc, stats, w: w.max(1), h: h.max(1), scale: scale.clamp(0.5, 4.0) }
    }

    pub fn info(&self) -> Info {
        let title = self.doc.find_title_node().map(|n| n.text_content()).map(|t| t.split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default();
        Info { title, content_height: self.doc.root_element().final_layout().size.height, scroll_y: self.doc.viewport_scroll().y as f32, follow_up: String::new() }
    }

    fn view_css_height(&self) -> f32 {
        self.h as f32 / self.scale
    }

    pub fn resize(&mut self, w: u32, h: u32, scale: f32) -> Info {
        (self.w, self.h, self.scale) = (w.max(1), h.max(1), scale.clamp(0.5, 4.0));
        self.doc.set_viewport(viewport(self.w, self.h, self.scale));
        self.doc.resolve(0.0);
        let y = scroll::clamp(self.doc.viewport_scroll().y as f32, self.doc.root_element().final_layout().size.height, self.view_css_height());
        self.doc.set_viewport_scroll(blitz_dom::Point { x: 0.0, y: y as f64 });
        self.info()
    }

    pub fn scroll_to(&mut self, y: f32) -> Info {
        let y = scroll::clamp(y, self.doc.root_element().final_layout().size.height, self.view_css_height());
        self.doc.set_viewport_scroll(blitz_dom::Point { x: 0.0, y: y as f64 });
        self.info()
    }

    pub fn render(&mut self) -> (Frame, Info) {
        self.doc.resolve(0.0);
        let (w, h, s) = (self.w, self.h, self.scale as f64);
        let doc = &mut self.doc;
        let rgba = render_to_buffer::<VelloCpuImageRenderer, _>(|scene| blitz_paint::paint_scene(scene, doc, s, w, h, 0, 0), w, h);
        (Frame { width: w, height: h, rgba }, self.info())
    }

    /// The link at a point of the picture (physical pixels), found by walking up from what is under it.
    pub fn link_at(&self, x: f32, y: f32) -> Option<Link> {
        let (x, y) = (x / self.scale, y / self.scale + self.doc.viewport_scroll().y as f32);
        let hit = self.doc.hit(x, y)?;
        let mut id = Some(hit.node_id);
        while let Some(n) = id.and_then(|i| self.doc.get_node(i)) {
            if let Some(href) = n.attr(blitz_dom::local_name!("href")) {
                return resolve_link(href);
            }
            id = n.parent;
        }
        None
    }
}
