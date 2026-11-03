//! What the conversation shows. The stored blocks are a flat list of what the model did; the screen groups them:
//! your messages, the model's answers as plain text, and the thinking and tool calls between them folded into one
//! quiet line ("Worked for 14 s, 3 steps") that opens to the list. No window code here, so it is tested.

use crate::model::Block;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKind {
    Thinking,
    Tool,
}

/// One thing the model did while working: it thought, or it used a tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub kind: StepKind,
    /// The block it came from.
    pub block: usize,
    /// A short line for it: "Thinking", or the tool and the start of what it was given.
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// A block drawn as it is: your message, a notice, an error, something opened beside the chat.
    Block(usize),
    /// The model's words, as plain text.
    Answer(usize),
    /// Steps folded into one line. `first` is the block where they begin (it names the line for opening and closing).
    Work { first: usize, steps: Vec<Step>, secs: Option<u64>, running: bool },
}

/// Group the blocks for drawing. `busy` is whether the model is still working, which makes a trailing run of
/// steps "running" rather than finished.
pub fn items(blocks: &[Block], busy: bool) -> Vec<Item> {
    let mut out = vec![];
    let mut steps: Vec<Step> = vec![];
    let mut first = 0;
    let flush = |out: &mut Vec<Item>, steps: &mut Vec<Step>, first: usize, secs: Option<u64>, running: bool| {
        if !steps.is_empty() {
            out.push(Item::Work { first, steps: std::mem::take(steps), secs, running });
        }
    };
    for (i, b) in blocks.iter().enumerate() {
        match b {
            Block::Assistant { text, reasoning, worked } => {
                if !reasoning.trim().is_empty() {
                    if steps.is_empty() {
                        first = i;
                    }
                    steps.push(Step { kind: StepKind::Thinking, block: i, label: String::new() });
                }
                if text.trim().is_empty() {
                    continue;
                }
                flush(&mut out, &mut steps, first, *worked, false);
                out.push(Item::Answer(i));
            }
            Block::Tool { name, arguments, .. } => {
                if steps.is_empty() {
                    first = i;
                }
                steps.push(Step { kind: StepKind::Tool, block: i, label: format!("{name} {}", arguments.chars().take(90).collect::<String>()).trim_end().to_string() });
            }
            _ => {
                flush(&mut out, &mut steps, first, None, false);
                out.push(Item::Block(i));
            }
        }
    }
    flush(&mut out, &mut steps, first, None, busy);
    out
}

/// "14 s" or "1 min 5 s".
pub fn duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs} s")
    } else if secs.is_multiple_of(60) {
        format!("{} min", secs / 60)
    } else {
        format!("{} min {} s", secs / 60, secs % 60)
    }
}

fn steps_en(n: usize) -> String {
    if n == 1 { "1 step".into() } else { format!("{n} steps") }
}

fn steps_ar(n: usize) -> String {
    match n {
        1 => "خطوة واحدة".into(),
        2 => "خطوتان".into(),
        3..=10 => format!("{n} خطوات"),
        _ => format!("{n} خطوة"),
    }
}

/// The folded line: "Worked for 14 s, 3 steps", "Working… 2 steps" while it goes on, "3 steps" when nobody timed it.
pub fn work_label(secs: Option<u64>, steps: usize, running: bool, rtl: bool) -> String {
    if rtl {
        let n = steps_ar(steps);
        return match (running, secs) {
            (true, _) => format!("يعمل… {n}"),
            (false, Some(s)) => format!("اشتغل {}، {n}", duration_ar(s)),
            (false, None) => n,
        };
    }
    let n = steps_en(steps);
    match (running, secs) {
        (true, _) => format!("Working… {n}"),
        (false, Some(0)) => format!("Worked for under 1 s, {n}"),
        (false, Some(s)) => format!("Worked for {}, {n}", duration(s)),
        (false, None) => n,
    }
}

pub fn duration_ar(secs: u64) -> String {
    if secs < 60 {
        format!("{secs} ث")
    } else if secs.is_multiple_of(60) {
        format!("{} د", secs / 60)
    } else {
        format!("{} د {} ث", secs / 60, secs % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(id: &str, name: &str) -> Block {
        Block::Tool { id: id.into(), name: name.into(), arguments: "{\"q\":1}".into(), result: Some("r".into()) }
    }
    fn say(text: &str, reasoning: &str, worked: Option<u64>) -> Block {
        Block::Assistant { text: text.into(), reasoning: reasoning.into(), worked }
    }

    #[test]
    fn thinking_and_tools_fold_into_one_line_before_the_answer_they_led_to() {
        let blocks = vec![Block::User("find".into()), say("", "hmm", None), tool("a", "web_search"), tool("b", "web_fetch"), say("Found it.", "", Some(14))];
        let it = items(&blocks, false);
        assert_eq!(it.len(), 3);
        assert_eq!(it[0], Item::Block(0));
        match &it[1] {
            Item::Work { first, steps, secs, running } => {
                assert_eq!((*first, steps.len(), *secs, *running), (1, 3, Some(14), false), "thinking counts as a step; the answer's block says how long");
                assert_eq!(steps[0].kind, StepKind::Thinking);
                assert_eq!(steps[1].label, "web_search {\"q\":1}");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(it[2], Item::Answer(4));
    }

    #[test]
    fn thinking_inside_the_answers_own_block_still_counts_as_work_before_it() {
        let it = items(&[Block::User("x".into()), say("Yes.", "let me think", Some(2))], false);
        assert_eq!(it.len(), 3);
        assert!(matches!(&it[1], Item::Work { steps, secs: Some(2), .. } if steps.len() == 1));
        assert_eq!(it[2], Item::Answer(1));
    }

    #[test]
    fn a_plain_answer_has_no_work_line() {
        assert_eq!(items(&[Block::User("x".into()), say("Hi", "", Some(1))], false), vec![Item::Block(0), Item::Answer(1)]);
    }

    #[test]
    fn work_between_two_answers_is_its_own_line_and_the_order_is_kept() {
        let blocks = vec![Block::User("x".into()), say("First.", "", None), tool("a", "t"), say("Second.", "", Some(3)), Block::Notice("n".into())];
        let it = items(&blocks, false);
        assert_eq!(it.iter().map(|i| match i { Item::Block(_) => "block", Item::Answer(_) => "answer", Item::Work { .. } => "work" }).collect::<Vec<_>>(), ["block", "answer", "work", "answer", "block"]);
    }

    #[test]
    fn work_at_the_end_is_running_only_while_the_model_is_busy() {
        let blocks = vec![Block::User("x".into()), tool("a", "t")];
        assert!(matches!(items(&blocks, true).last(), Some(Item::Work { running: true, .. })));
        assert!(matches!(items(&blocks, false).last(), Some(Item::Work { running: false, secs: None, .. })));
    }

    #[test]
    fn work_cut_off_by_a_notice_or_error_is_closed_without_a_time() {
        let blocks = vec![Block::User("x".into()), tool("a", "t"), Block::Error("boom".into())];
        let it = items(&blocks, false);
        assert!(matches!(&it[1], Item::Work { secs: None, running: false, .. }));
        assert_eq!(it[2], Item::Block(2));
    }

    #[test]
    fn empty_answers_and_blank_thinking_make_nothing() {
        assert_eq!(items(&[say("", "  ", None)], false), vec![]);
    }

    #[test]
    fn the_folded_line_reads_plainly() {
        assert_eq!(work_label(Some(14), 3, false, false), "Worked for 14 s, 3 steps");
        assert_eq!(work_label(Some(65), 1, false, false), "Worked for 1 min 5 s, 1 step");
        assert_eq!(work_label(Some(120), 2, false, false), "Worked for 2 min, 2 steps");
        assert_eq!(work_label(Some(0), 1, false, false), "Worked for under 1 s, 1 step");
        assert_eq!(work_label(None, 2, false, false), "2 steps");
        assert_eq!(work_label(Some(9), 2, true, false), "Working… 2 steps", "no time while it goes on");
        assert_eq!(work_label(Some(14), 3, false, true), "اشتغل 14 ث، 3 خطوات");
        assert_eq!(work_label(None, 1, false, true), "خطوة واحدة");
        assert_eq!(work_label(None, 2, false, true), "خطوتان");
        assert_eq!(work_label(None, 12, false, true), "12 خطوة");
        assert_eq!(work_label(None, 4, true, true), "يعمل… 4 خطوات");
    }
}
