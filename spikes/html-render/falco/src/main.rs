//! hr-falco PAGE OUT.png [--width W] [--height H]
//! Prints key=value lines: memory (resident MB) before and after, time to render, time to repaint a frame.
use std::time::Instant;

use falco::paint::{paint, Canvas, FontRasterizer};
use falco::RenderOptions;

fn rss_mb() -> f64 {
    let s = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: f64 = s.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    pages * 4096.0 / 1048576.0
}

/// The same helper the Falco command line uses: the library takes CSS separately, so the page's own style tags are
/// pulled out first. (Copied from falco/src/main.rs; the library does not do it.)
fn extract_style_tags(html: &str) -> String {
    let mut out = String::new();
    let lower = html.to_lowercase();
    let mut search_from = 0;
    while let Some(open) = lower[search_from..].find("<style") {
        let open_abs = search_from + open;
        let Some(close_rel) = lower[open_abs..].find('>') else { break };
        let content_start = open_abs + close_rel + 1;
        let Some(end_rel) = lower[content_start..].find("</style>") else { break };
        let content_end = content_start + end_rel;
        out.push_str(&html[content_start..content_end]);
        out.push('\n');
        search_from = content_end + 8;
    }
    out
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: hr-falco PAGE OUT.png [--width W] [--height H] [--frames N]");
        std::process::exit(2);
    }
    let (page, out) = (&args[1], &args[2]);
    let (mut w, mut h, mut frames) = (900u32, 1000u32, 20usize);
    let mut i = 3;
    while i + 1 < args.len() {
        match args[i].as_str() {
            "--width" => w = args[i + 1].parse()?,
            "--height" => h = args[i + 1].parse()?,
            "--frames" => frames = args[i + 1].parse()?,
            _ => {}
        }
        i += 2;
    }
    // bytes, not str: a truncated or broken file may not be valid UTF-8
    let bytes = std::fs::read(page)?;
    let mut html = String::from_utf8_lossy(&bytes).to_string();
    // Falco 1.0.0 (f0b4028) draws a blank page when the page has no <script> at all (the path that skips the script
    // engine loses the document). HR_FORCE_SCRIPT=1 appends a harmless script so the real drawing path is used.
    if std::env::var_os("HR_FORCE_SCRIPT").is_some() {
        html.push_str("\n<script>0</script>\n");
    }
    let abs = std::fs::canonicalize(page)?;
    let base = format!("file://{}", abs.display());
    let css = extract_style_tags(&html);
    println!("rss_start_mb={:.1}", rss_mb());
    let opts = RenderOptions { width: w, height: h, ..Default::default() };
    let t0 = Instant::now();
    let (canvas, commands) = falco::render_with_base_url(&html, &css, opts.clone(), &base)?;
    let first = t0.elapsed().as_secs_f64() * 1000.0;
    println!("render_first_ms={first:.1}\nrss_after_render_mb={:.1}", rss_mb());
    let t1 = Instant::now();
    let _ = falco::render_with_base_url(&html, &css, opts, &base)?;
    println!("render_second_ms={:.1}", t1.elapsed().as_secs_f64() * 1000.0);
    println!("canvas={}x{}\ncommands={}", canvas.width, canvas.height, commands.len());
    std::fs::write(out, falco::png::encode(canvas.width, canvas.height, &canvas.pixels))?;
    // repaint: draw the stored paint commands again into a view-sized canvas, as a scroll would
    let raster = FontRasterizer::new()?;
    let mut total = 0.0f64;
    let mut worst = 0.0f64;
    for _ in 0..frames {
        let mut c = Canvas::new(w, h);
        let a = Instant::now();
        paint(&commands, &mut c, &raster);
        let d = a.elapsed().as_secs_f64() * 1000.0;
        total += d;
        worst = worst.max(d);
    }
    println!("repaint_avg_ms={:.2}\nrepaint_worst_ms={:.2}\nrss_end_mb={:.1}", total / frames.max(1) as f64, worst, rss_mb());
    Ok(())
}
