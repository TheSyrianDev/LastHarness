//! The model slider: five stops from fastest to smartest, each holding one model. A stop the person set by hand
//! keeps its model; the others are filled from the models on hand, smallest to largest, until the assistant's
//! benchmark ranking (or the person) says otherwise.

pub const STOPS: usize = 5;

pub fn stop_name(i: usize, rtl: bool) -> &'static str {
    const EN: [&str; STOPS] = ["Fastest", "Fast", "Balanced", "Smart", "Smartest"];
    const AR: [&str; STOPS] = ["الأسرع", "سريع", "متوازن", "ذكي", "الأذكى"];
    if rtl { AR[i.min(STOPS - 1)] } else { EN[i.min(STOPS - 1)] }
}

/// Billions of parameters from a size label such as "35B", "2.6B" or "20b"; unknown sizes sort in the middle.
pub fn size_of(label: &str) -> f32 {
    label.trim().trim_end_matches(['B', 'b']).parse().unwrap_or(8.)
}

/// The model on each stop. `set` is what the person (or an accepted ranking) chose, one entry per stop, empty for
/// "fill it for me"; `models` is every model as (name, size label).
pub fn fill(set: &[String], models: &[(String, String)]) -> [String; STOPS] {
    // a model set by hand on one stop is not also put on another by the automatic fill
    // gpt-oss is never put on a stop by the fill (the user's choice)
    let mut sorted: Vec<&(String, String)> = models.iter().filter(|m| !set.contains(&m.0) && !never_filled(&m.0)).collect();
    sorted.sort_by(|a, b| size_of(&a.1).total_cmp(&size_of(&b.1)).then(a.0.cmp(&b.0)));
    let mut out: [String; STOPS] = Default::default();
    for i in 0..STOPS {
        let chosen = set.get(i).filter(|s| !s.is_empty() && models.iter().any(|m| &m.0 == *s));
        out[i] = match chosen {
            Some(s) => s.clone(),
            None if sorted.is_empty() => String::new(),
            // spread the stops evenly from the smallest model to the largest
            None => sorted[(i * (sorted.len() - 1) + (STOPS - 1) / 2) / (STOPS - 1)].0.clone(),
        };
    }
    out
}

/// Models the automatic fill never puts on a stop.
pub fn never_filled(name: &str) -> bool {
    name.to_lowercase().contains("gpt-oss")
}

/// The stop showing a model, if it sits on one.
pub fn stop_of(stops: &[String; STOPS], model: &str) -> Option<usize> {
    stops.iter().position(|s| s == model)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn fills_smallest_to_largest() {
        let models = m(&[("big", "35B"), ("tiny", "1.7B"), ("mid", "4B"), ("small", "2B"), ("large", "20b")]);
        assert_eq!(fill(&[], &models), ["tiny", "small", "mid", "large", "big"].map(String::from));
    }

    #[test]
    fn a_hand_set_stop_wins_and_unknown_names_are_ignored() {
        let models = m(&[("a", "1B"), ("b", "35B")]);
        let set = vec!["b".into(), String::new(), "gone".into()];
        let f = fill(&set, &models);
        assert_eq!(f[0], "b");
        assert_eq!(f[4], "a", "b is already on a stop, so the fill uses a");
        assert!(f[2] == "a" || f[2] == "b");
        assert_eq!(stop_of(&f, "b"), Some(0));
    }

    #[test]
    fn a_hand_set_model_is_not_repeated_by_the_fill() {
        let models = m(&[("a", "1B"), ("b", "2B"), ("c", "4B"), ("d", "35B"), ("e", "35B"), ("sage", "35B")]);
        let f = fill(&["".into(), "".into(), "".into(), "".into(), "sage".into()], &models);
        assert_eq!(f[4], "sage");
        assert_eq!(f.iter().filter(|x| *x == "sage").count(), 1);
    }

    #[test]
    fn gpt_oss_is_never_filled_in() {
        let f = fill(&[], &m(&[("gpt-oss-20b", "20b"), ("a", "1B"), ("b", "35B")]));
        assert!(f.iter().all(|s| s != "gpt-oss-20b"));
    }

    #[test]
    fn no_models_no_stops() {
        assert!(fill(&[], &[]).iter().all(String::is_empty));
        assert_eq!(size_of("2.6B"), 2.6);
        assert_eq!(size_of(""), 8.);
    }
}

/// How a held thumb looks for a pointer at `raw` pixels along the track, with `gap` pixels between stops: where it
/// is drawn and how wide it stretches. Each stop holds on: near a stop the thumb barely moves, then it gives way and
/// hurries to the next, and it stretches toward where it is being pulled.
pub fn held(raw: f32, gap: f32) -> (f32, f32) {
    held_n(raw, gap, STOPS)
}

/// `held` for a slider with `n` stops.
pub fn held_n(raw: f32, gap: f32, n: usize) -> (f32, f32) {
    let last = gap * (n.max(2) - 1) as f32;
    let raw = raw.clamp(0., last);
    let near = (raw / gap).round() * gap;
    let d = (raw - near) / (gap / 2.); // -1 to 1 between the half-way points
    let shown = near + d.signum() * d.abs().powi(3) * gap / 2.;
    // the thumb reaches from where it is drawn to the pointer: zero on a stop and at half way, so it never jumps
    let stretch = (raw - shown).abs();
    (shown, stretch)
}

/// Where the held thumb's left edge sits and how much wider than usual it is: it spans from its drawn place to
/// the pointer, so it grows toward the pull and both edges move smoothly.
pub fn knob(raw: f32, gap: f32) -> (f32, f32) {
    knob_n(raw, gap, STOPS)
}

/// `knob` for a slider with `n` stops.
pub fn knob_n(raw: f32, gap: f32, n: usize) -> (f32, f32) {
    let (shown, stretch) = held_n(raw, gap, n);
    let raw = raw.clamp(0., gap * (n.max(2) - 1) as f32);
    (shown.min(raw), stretch)
}

#[cfg(test)]
mod held_tests {
    use super::held;

    #[test]
    fn stops_hold_on_then_give_way() {
        let (at, s) = held(40., 20.);
        assert_eq!((at, s), (40., 0.), "on a stop: still");
        let (a, _) = held(44., 20.);
        assert!(a < 41., "a little pull barely moves it: {a}");
        let (b, _) = held(49.9, 20.);
        assert!(b > 49., "near half way it catches up: {b}");
        assert!(held(45., 20.).1 > 2., "part way it stretches");
        assert_eq!(held(-30., 20.).0, 0.);
        assert_eq!(held(500., 20.).0, 80.);
    }
}

#[cfg(test)]
mod knob_tests {
    use super::knob;

    #[test]
    fn edges_move_smoothly_across_stops_and_half_way() {
        let gap = 60.;
        let mut last = knob(-10., gap);
        let mut raw = -10.;
        while raw < 260. {
            raw += 0.05;
            let k = knob(raw, gap);
            assert!((k.0 - last.0).abs() < 0.5, "left edge jumped at {raw}: {last:?} -> {k:?}");
            assert!((k.1 - last.1).abs() < 0.5, "width jumped at {raw}: {last:?} -> {k:?}");
            last = k;
        }
    }

    #[test]
    fn grows_toward_the_pull() {
        let (l, w) = knob(70., 60.);
        assert!(l >= 60. && w > 0., "pulled right of a stop: left edge stays, it grows right");
        let (l, w) = knob(50., 60.);
        assert!((l - 50.).abs() < 1e-4 && w > 0., "pulled left: it grows left to the pointer");
        assert_eq!(knob(120., 60.), (120., 0.));
    }

    #[test]
    fn the_glass_follows_the_pointer_and_release_starts_where_it_was_drawn() {
        use super::knob;
        let gap = 40.;
        for raw in [0., 7., 19., 20., 21., 33., 40., 61., 80., 160.] {
            let (left, stretch) = knob(raw, gap);
            // the drawn glass spans from its left edge to the pointer: the pointer is always inside it
            assert!(left <= raw.clamp(0., 160.) + 1e-4 && raw.clamp(0., 160.) <= left + stretch + 1e-3, "raw {raw}: {left}+{stretch}");
        }
        // pulled back toward the start, the left edge is the pointer itself; release must glide from there
        let (left, _) = knob(70., gap);
        assert_eq!(left, 70.);
        assert!(super::held(70., gap).0 > left, "the old release point (the resisting centre) sat to the right: a jump");
    }
}
