//! The model slider: 2 to 7 stops (5 by default), each with a title the person can edit and holding one model. A stop
//! the person set by hand keeps its model; the others are filled from the models on hand, one model per stop, smallest
//! to largest, until the assistant's benchmark ranking (or the person) says otherwise. With fewer models than stops,
//! the extra stops stay empty.

pub const MIN_STOPS: usize = 2;
pub const MAX_STOPS: usize = 7;
/// The stop count when the person has not chosen one (also what an older settings file gets).
pub const DEFAULT_STOPS: usize = 5;

/// The stop count to use: 0 (never set) means the default; anything else is kept within 2 to 7.
pub fn clamp_count(n: usize) -> usize {
    if n == 0 { DEFAULT_STOPS } else { n.clamp(MIN_STOPS, MAX_STOPS) }
}

const EN_TITLES: [&str; DEFAULT_STOPS] = ["Low", "Medium", "High", "Extra", "Max"];
const AR_TITLES: [&str; DEFAULT_STOPS] = ["منخفض", "متوسط", "مرتفع", "إضافي", "أقصى"];

/// The title stop `i` shows: the one the person typed, else the default. Past the fifth stop the default is "Stop N".
pub fn title_of(titles: &[String], i: usize, rtl: bool) -> String {
    match titles.get(i).map(|t| t.trim()).filter(|t| !t.is_empty()) {
        Some(t) => t.to_string(),
        None if i < DEFAULT_STOPS => (if rtl { AR_TITLES[i] } else { EN_TITLES[i] }).to_string(),
        None if rtl => format!("مستوى {}", i + 1),
        None => format!("Stop {}", i + 1),
    }
}

/// Billions of parameters from a size label such as "35B", "2.6B" or "20b"; unknown sizes sort in the middle.
pub fn size_of(label: &str) -> f32 {
    label.trim().trim_end_matches(['B', 'b']).parse().unwrap_or(8.)
}

/// The model on each of `n` stops (the count is kept within 2 to 7). `set` is what the person (or an accepted
/// ranking) chose, one entry per stop, empty for "fill it for me"; `models` is every model as (name, size label).
/// No model sits on two stops: a repeat in `set` is treated as empty, and the fill uses models not already placed.
pub fn fill(set: &[String], models: &[(String, String)], n: usize) -> Vec<String> {
    let n = clamp_count(n);
    let on_hand = |s: &str| models.iter().any(|m| m.0 == s);
    let mut out = vec![String::new(); n];
    let mut taken: Vec<String> = vec![];
    // a stop the person set keeps its model, once
    for i in 0..n {
        if let Some(s) = set.get(i).filter(|s| !s.is_empty() && on_hand(s) && !taken.contains(s)) {
            out[i] = s.clone();
            taken.push(s.clone());
        }
    }
    // the built-in default for a stop the person left empty
    for i in 0..n {
        if let Some(d) = DEFAULTS.get(i).filter(|d| !d.is_empty() && on_hand(d) && out[i].is_empty() && !taken.iter().any(|t| t == *d)) {
            out[i] = d.to_string();
            taken.push(d.to_string());
        }
    }
    // gpt-oss is never put on a stop by the fill (the user's choice)
    let mut free: Vec<&(String, String)> = models.iter().filter(|m| !taken.contains(&m.0) && !never_filled(&m.0)).collect();
    free.sort_by(|a, b| size_of(&a.1).total_cmp(&size_of(&b.1)).then(a.0.cmp(&b.0)));
    let empty: Vec<usize> = (0..n).filter(|&i| out[i].is_empty()).collect();
    let (k, m) = (empty.len(), free.len());
    if m >= k {
        // spread the empty stops evenly from the smallest model to the largest, each with a different model
        for (j, &i) in empty.iter().enumerate() {
            let at = if k == 1 { (m - 1) / 2 } else { (j * (m - 1) * 2 + (k - 1)) / (2 * (k - 1)) };
            out[i] = free[at].0.clone();
        }
    } else {
        // fewer models than stops: the lowest empty stops get the models, smallest first; the rest stay empty
        for (j, &i) in empty.iter().take(m).enumerate() {
            out[i] = free[j].0.clone();
        }
    }
    out
}

/// The model a stop holds when the person has not set one and it is on hand: the first stop, where gpt-oss used to
/// sit, is Qwen3.6 MiniPlus (Sage-A1.5) with sharp thinking (the user's choice); its default entry is balanced and sharp.
pub const DEFAULTS: [&str; DEFAULT_STOPS] = ["Sage-A1.5-35B-A3B", "", "", "", ""];

/// Models the automatic fill never puts on a stop.
pub fn never_filled(name: &str) -> bool {
    name.to_lowercase().contains("gpt-oss")
}

/// The stop showing a model, if it sits on one.
pub fn stop_of(stops: &[String], model: &str) -> Option<usize> {
    stops.iter().position(|s| s == model)
}

/// Whether stop `i` is the one a model is shown on. A model can sit on two stops (the fill does this when there are
/// fewer models than stops); only the first is shown, as the slider's thumb shows it, so only that one is lit.
pub fn is_shown_stop(stops: &[String], model: &str, i: usize) -> bool {
    stop_of(stops, model) == Some(i)
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
        assert_eq!(fill(&[], &models, 5), ["tiny", "small", "mid", "large", "big"]);
    }

    #[test]
    fn a_hand_set_stop_wins_and_unknown_names_are_ignored() {
        let models = m(&[("a", "1B"), ("b", "35B")]);
        let set = vec!["b".into(), String::new(), "gone".into()];
        let f = fill(&set, &models, 5);
        assert_eq!(f[0], "b");
        assert_eq!(f[1], "a", "b is already on a stop, so the fill uses a");
        assert!(f[2..].iter().all(|s| s.is_empty()), "two models, so the rest stay empty: {f:?}");
        assert_eq!(stop_of(&f, "b"), Some(0));
    }

    #[test]
    fn a_hand_set_model_is_not_repeated_by_the_fill() {
        let models = m(&[("a", "1B"), ("b", "2B"), ("c", "4B"), ("d", "35B"), ("e", "35B"), ("sage", "35B")]);
        let f = fill(&["".into(), "".into(), "".into(), "".into(), "sage".into()], &models, 5);
        assert_eq!(f[4], "sage");
        assert_eq!(f.iter().filter(|x| *x == "sage").count(), 1);
    }

    #[test]
    fn a_model_is_never_on_two_stops() {
        let models = m(&[("a", "1B"), ("b", "2B"), ("c", "35B")]);
        for n in MIN_STOPS..=MAX_STOPS {
            let f = fill(&[], &models, n);
            assert_eq!(f.len(), n);
            for model in models.iter().map(|m| &m.0) {
                assert!(f.iter().filter(|x| *x == model).count() <= 1, "{model} twice at n={n}: {f:?}");
            }
            let set: Vec<String> = f.clone();
            assert_eq!(fill(&set, &models, n), f, "a filled slider stays the same at n={n}");
        }
    }

    #[test]
    fn fewer_models_than_stops_leaves_the_extra_stops_empty() {
        let models = m(&[("a", "1B"), ("b", "2B"), ("c", "35B")]);
        let f = fill(&[], &models, 7);
        assert_eq!(f, ["a", "b", "c", "", "", "", ""]);
    }

    #[test]
    fn more_models_spread_across_the_stops() {
        let models = m(&[("a", "1B"), ("b", "2B"), ("c", "4B"), ("d", "8B"), ("e", "14B"), ("f", "35B"), ("g", "70B")]);
        assert_eq!(fill(&[], &models, 2), ["a", "g"]);
        assert_eq!(fill(&[], &models, 7), ["a", "b", "c", "d", "e", "f", "g"]);
        let three = fill(&[], &models, 3);
        assert_eq!(three[0], "a");
        assert_eq!(three[2], "g");
    }

    #[test]
    fn the_count_is_kept_within_two_to_seven_and_zero_means_five() {
        assert_eq!(clamp_count(0), 5);
        assert_eq!(clamp_count(1), 2);
        assert_eq!(clamp_count(3), 3);
        assert_eq!(clamp_count(9), 7);
        assert_eq!(fill(&[], &m(&[("a", "1B")]), 0).len(), 5);
    }

    #[test]
    fn titles_default_then_follow_the_person() {
        let none: Vec<String> = vec![];
        assert_eq!(title_of(&none, 0, false), "Low");
        assert_eq!(title_of(&none, 4, false), "Max");
        assert_eq!(title_of(&none, 5, false), "Stop 6");
        assert_eq!(title_of(&none, 0, true), "منخفض");
        let set = vec!["  Tiny  ".to_string(), String::new()];
        assert_eq!(title_of(&set, 0, false), "Tiny", "a typed title wins, trimmed");
        assert_eq!(title_of(&set, 1, false), "Medium", "an empty title falls back to the default");
    }

    #[test]
    fn the_first_stop_defaults_to_sage() {
        let f = fill(&[], &m(&[("a", "1B"), ("Sage-A1.5-35B-A3B", "35B")]), 5);
        assert_eq!(f[0], "Sage-A1.5-35B-A3B");
        let f = fill(&["a".into()], &m(&[("a", "1B"), ("Sage-A1.5-35B-A3B", "35B")]), 5);
        assert_eq!(f[0], "a", "a stop set by hand wins");
    }

    #[test]
    fn gpt_oss_is_never_filled_in() {
        let f = fill(&[], &m(&[("gpt-oss-20b", "20b"), ("a", "1B"), ("b", "35B")]), 5);
        assert!(f.iter().all(|s| s != "gpt-oss-20b"));
    }

    #[test]
    fn a_lit_model_is_shown_on_its_first_stop_only() {
        let f = vec!["a".to_string(), "b".into(), "b".into()];
        assert!(is_shown_stop(&f, "b", 1));
        assert!(!is_shown_stop(&f, "b", 2));
    }

    #[test]
    fn no_models_no_stops() {
        assert!(fill(&[], &[], 5).iter().all(String::is_empty));
        assert_eq!(size_of("2.6B"), 2.6);
        assert_eq!(size_of(""), 8.);
    }
}

/// How a held thumb looks for a pointer at `raw` pixels along the track, with `gap` pixels between stops: where it
/// is drawn and how wide it stretches. Each stop holds on: near a stop the thumb barely moves, then it gives way and
/// hurries to the next, and it stretches toward where it is being pulled.
pub fn held(raw: f32, gap: f32) -> (f32, f32) {
    held_n(raw, gap, DEFAULT_STOPS)
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
    knob_n(raw, gap, DEFAULT_STOPS)
}

/// `knob` for a slider with `n` stops.
pub fn knob_n(raw: f32, gap: f32, n: usize) -> (f32, f32) {
    let (shown, stretch) = held_n(raw, gap, n);
    let raw = raw.clamp(0., gap * (n.max(2) - 1) as f32);
    (shown.min(raw), stretch)
}

/// `knob_n` for a thumb `height` pixels tall, with the stretch reined in so a wide thumb (the mode switch) and a
/// small one (a slider ball) give way alike: the stretch eases toward at most 40% of the height instead of reaching
/// the pointer. The trailing edge stays where the stop holds it, so only the leading edge moves with the pull.
pub fn thumb_n(raw: f32, gap: f32, n: usize, height: f32) -> (f32, f32) {
    let (shown, s) = held_n(raw, gap, n);
    let cap = (height * 0.4).max(1.);
    let soft = cap * (1. - (-s / cap).exp());
    let raw = raw.clamp(0., gap * (n.max(2) - 1) as f32);
    (if raw < shown { shown - soft } else { shown }, soft)
}

/// How much a thumb stretched by `stretch` pixels flattens, so it keeps its volume like a drop: a quarter of the
/// stretch, taken evenly off the top and bottom.
pub fn squash(stretch: f32) -> f32 {
    stretch.max(0.) * 0.25
}

#[cfg(test)]
mod thumb_tests {
    use super::{held_n, thumb_n};

    #[test]
    fn the_trailing_edge_never_moves_and_the_stretch_is_capped() {
        let mut prev = thumb_n(0., 60., 5, 22.);
        for i in 1..=2400 {
            let raw = i as f32 * 0.1;
            let (left, s) = thumb_n(raw, 60., 5, 22.);
            assert!(s <= 22. * 0.4 + 1e-3, "capped at {raw}: {s}");
            let (shown, _) = held_n(raw, 60., 5);
            let right = left + s;
            assert!((left - shown).abs() < 1e-3 || (right - shown).abs() < 1e-3, "one edge on the held place at {raw}");
            assert!((left - prev.0).abs() < 1.5 && (s - prev.1).abs() < 1.5, "no jump at {raw}");
            prev = (left, s);
        }
    }
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
