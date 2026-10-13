// Throwaway: draw a pane of a theme's glass and a few buttons as SVG over plain backdrops, so the look can be
// checked without a window. It follows the same numbers as the app (fill, rim, edge, reflection, hairlines) but it
// is a picture made by hand, not the toolkit's own drawing. Usage: glass_preview THEME.toml OUT.svg [backdrop #rrggbb]...
use lh_gui::sheen;
use lh_gui::theme::{Color, Theme};
use std::fmt::Write;

fn rgba(c: Color) -> String {
    format!("rgba({},{},{},{:.3})", c.r, c.g, c.b, c.a as f32 / 255.)
}
fn white(a: f32) -> String {
    format!("rgba(255,255,255,{a:.3})")
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let t = Theme::parse(&src, std::path::Path::new("")).unwrap();
    let backdrops: Vec<String> = if a.len() > 3 { a[3..].to_vec() } else { vec!["#2e6fa8".into(), "#9a9a9a".into(), "#e8e0c8".into(), "#101418".into()] };
    let g = t.glass.clone().expect("a theme with [glass]");
    let (pw, ph, gap, r) = (330.0f32, 230.0f32, 24.0f32, t.roundness);
    let total_w = (pw + gap) * backdrops.len() as f32 + gap;
    let mut s = String::new();
    writeln!(s, "<svg xmlns='http://www.w3.org/2000/svg' width='{total_w}' height='{}'>", ph + 2. * gap + 90.).unwrap();
    writeln!(s, "<defs><linearGradient id='fill' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{0}'/><stop offset='1' stop-color='{1}'/></linearGradient>", rgba(g.top), rgba(g.bottom)).unwrap();
    writeln!(s, "<linearGradient id='sheen' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{}'/><stop offset='1' stop-color='{}'/></linearGradient>", white(g.sheen), white(0.)).unwrap();
    let gel = t.gel.clone().expect("a theme with [gel]");
    writeln!(s, "<linearGradient id='gloss' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{}'/><stop offset='1' stop-color='{}'/></linearGradient>", white(gel.gloss), white(0.)).unwrap();
    // under the pointer a button takes on the blue of its glow
    let blue = Color { a: 255, ..gel.glow };
    writeln!(s, "<linearGradient id='gh' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{}'/><stop offset='1' stop-color='{}'/></linearGradient>", rgba(gel.top.mix(Color { r: 255, g: 255, b: 255, a: 255 }, 0.1).mix(blue, 0.16)), rgba(gel.bottom.mix(Color { r: 255, g: 255, b: 255, a: 255 }, 0.06).mix(blue, 0.30))).unwrap();
    for (id, top, bot) in [("gb", gel.top, gel.bottom), ("gp", gel.primary_top, gel.primary_bottom)] {
        writeln!(s, "<linearGradient id='{id}' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{}'/><stop offset='1' stop-color='{}'/></linearGradient>", rgba(top), rgba(bot)).unwrap();
    }
    writeln!(s, "<filter id='sh' x='-20%' y='-20%' width='140%' height='150%'><feDropShadow dx='0' dy='{}' stdDeviation='{}' flood-color='{}'/></filter>", t.shadow_y, t.shadow_blur / 2., rgba(t.shadow_color)).unwrap();
    writeln!(s, "<filter id='glow' x='-30%' y='-60%' width='160%' height='220%'><feDropShadow dx='0' dy='0' stdDeviation='7' flood-color='{}'/></filter></defs>", rgba(gel.glow)).unwrap();
    for (i, bd) in backdrops.iter().enumerate() {
        let x0 = gap + i as f32 * (pw + gap);
        writeln!(s, "<rect x='{}' y='0' width='{}' height='{}' fill='{bd}'/>", x0 - gap / 2., pw + gap, ph + 2. * gap + 90.).unwrap();
        writeln!(s, "<g transform='translate({x0},{gap})'>").unwrap();
        writeln!(s, "<rect width='{pw}' height='{ph}' rx='{r}' fill='url(#fill)' stroke='{}' filter='url(#sh)'/>", rgba(g.edge)).unwrap();
        writeln!(s, "<clipPath id='c{i}'><rect x='1' y='1' width='{}' height='{}' rx='{}'/></clipPath><g clip-path='url(#c{i})'>", pw - 2., ph - 2., (r - 1.).max(0.)).unwrap();
        writeln!(s, "<rect width='{pw}' height='{}' fill='url(#sheen)'/>", (ph * 0.42).min(150.)).unwrap();
        if g.reflection > 0. {
            for (poly, share) in sheen::reflection(pw, ph, r) {
                let pts: Vec<String> = poly.iter().map(|p| format!("{:.1},{:.1}", p.0, p.1)).collect();
                writeln!(s, "<polygon points='{}' fill='{}'/>", pts.join(" "), white(g.reflection * share)).unwrap();
            }
        }
        if g.lines > 0. {
            let (ax, ay, aw, ah) = sheen::hairline_area(pw, ph, r);
            for (p, q) in sheen::hairlines(aw, ah, g.lines_gap) {
                writeln!(s, "<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='{}' stroke-width='1'/>", ax + p.0, ay + p.1, ax + q.0, ay + q.1, white(g.lines)).unwrap();
            }
        }
        writeln!(s, "</g><rect x='1' y='1' width='{}' height='{}' rx='{}' fill='none' stroke='{}'/>", pw - 2., ph - 2., (r - 1.).max(0.), rgba(g.rim)).unwrap();
        writeln!(s, "<text x='18' y='34' font-family='sans-serif' font-size='16' fill='{}'>A pane of glass</text>", rgba(t.colors.text)).unwrap();
        writeln!(s, "<text x='18' y='58' font-family='sans-serif' font-size='13' fill='{}'>Muted text sits here, readable on the tint.</text>", rgba(t.colors.muted)).unwrap();
        writeln!(s, "<rect x='18' y='78' width='{}' height='30' rx='{}' fill='{}' stroke='{}'/>", pw - 36., (r - 4.).max(3.), rgba(t.colors.sunken), rgba(t.colors.border)).unwrap();
        writeln!(s, "<text x='28' y='98' font-family='sans-serif' font-size='13' fill='{}'>An input field</text>", rgba(t.colors.text)).unwrap();
        for (j, (id, label, hover)) in [("gb", "Button", false), ("gh", "Hovered", true), ("gp", "Primary", false)].into_iter().enumerate() {
            let bx = 18. + j as f32 * 100.;
            let rr = (r - 4.).max(3.);
            let f = if hover { " filter='url(#glow)'" } else { "" };
            writeln!(s, "<g{f}><rect x='{bx}' y='130' width='88' height='32' rx='{rr}' fill='url(#{id})' stroke='{}'/></g>", rgba(gel.edge)).unwrap();
            writeln!(s, "<rect x='{}' y='131' width='86' height='20' rx='{}' fill='url(#gloss)'/>", bx + 1., (rr - 1.).max(2.)).unwrap();
            let fg = if id == "gp" { rgba(t.colors.accent_text) } else { rgba(t.colors.text) };
            writeln!(s, "<text x='{}' y='151' font-family='sans-serif' font-size='13' fill='{fg}' text-anchor='middle'>{label}</text>", bx + 44.).unwrap();
        }
        writeln!(s, "<text x='18' y='196' font-family='sans-serif' font-size='11' fill='{}'>backdrop {bd}</text>", rgba(t.colors.muted)).unwrap();
        writeln!(s, "</g>").unwrap();
    }
    writeln!(s, "</svg>").unwrap();
    std::fs::write(&a[2], s).unwrap();
}
