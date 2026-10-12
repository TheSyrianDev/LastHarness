//! Writes the rail icons as an SVG on standard output so they can be looked at without a window:
//! `cargo run -p lh-gui --example rail_preview > icons.svg`

use lh_gui::chrome::RailItem;

fn main() {
    let mut svg = String::from("<svg xmlns='http://www.w3.org/2000/svg' width='520' height='140' viewBox='0 0 520 140'><rect width='520' height='140' fill='#f4f5f3'/>");
    for (n, item) in RailItem::ALL.iter().enumerate() {
        let ox = 20. + n as f32 * 126.;
        svg += &format!("<g transform='translate({ox},10) scale(4.5)' fill='none' stroke='#1d2420' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'>");
        for line in item.icon() {
            let pts: Vec<String> = line.iter().map(|(x, y)| format!("{x:.2},{y:.2}")).collect();
            svg += &format!("<polyline points='{}'/>", pts.join(" "));
        }
        svg += "</g>";
        svg += &format!("<text x='{}' y='132' font-size='13' font-family='sans-serif'>{}</text>", ox, item.label(false));
    }
    svg += "</svg>";
    println!("{svg}");
}
