// Throwaway: paint a theme's scene to a PPM so it can be looked at. Usage: scene_preview THEME.toml OUT.ppm
use std::io::Write;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let t = lh_gui::theme::Theme::parse(&std::fs::read_to_string(&a[1]).unwrap(), std::path::Path::new("")).unwrap();
    let (w, h) = (1600, 900);
    let t0 = std::time::Instant::now();
    let px = lh_gui::scene::render(&t.background.scene, w, h);
    eprintln!("rendered in {:?}", t0.elapsed());
    let mut f = std::fs::File::create(&a[2]).unwrap();
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    for p in px.chunks(4) { f.write_all(&p[..3]).unwrap(); }
}
