//! A short line diff of a file write, for the permission question: what goes, what comes, with a little
//! context. Big files are cut so the question stays readable.

/// Lines of unchanged context kept around each change.
const CONTEXT: usize = 2;
/// Most lines of diff shown; the rest is counted.
const MAX_LINES: usize = 80;
/// Above this many lines on either side the diff is not worked out line by line; only the sizes are given.
const MAX_INPUT: usize = 4000;

/// `before` is the file as it is (`None`: it does not exist yet), `after` what the tool would write.
pub fn diff(before: Option<&str>, after: &str) -> String {
    let Some(before) = before else {
        let lines: Vec<&str> = after.lines().collect();
        let mut out: Vec<String> = lines.iter().take(MAX_LINES).map(|l| format!("+{l}")).collect();
        if lines.len() > MAX_LINES {
            out.push(format!("… and {} more new lines", lines.len() - MAX_LINES));
        }
        return format!("New file, {} line{}:\n{}", lines.len(), plural(lines.len()), out.join("\n"));
    };
    if before == after {
        return "No change: the file already holds exactly this.".into();
    }
    let (a, b): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    if a.len() > MAX_INPUT || b.len() > MAX_INPUT {
        return format!("The file goes from {} to {} lines (too long to compare line by line).", a.len(), b.len());
    }
    let ops = lcs_ops(&a, &b);
    let (added, removed) = ops.iter().fold((0, 0), |(ad, rm), o| match o {
        Op::Add(_) => (ad + 1, rm),
        Op::Del(_) => (ad, rm + 1),
        Op::Same(_) => (ad, rm),
    });
    // keep changed lines and CONTEXT lines around them
    let keep: Vec<bool> = (0..ops.len()).map(|i| (i.saturating_sub(CONTEXT)..(i + CONTEXT + 1).min(ops.len())).any(|j| !matches!(ops[j], Op::Same(_)))).collect();
    let mut out = vec![];
    let mut skipped = false;
    for (i, o) in ops.iter().enumerate() {
        if !keep[i] {
            skipped = true;
            continue;
        }
        if skipped && !out.is_empty() {
            out.push("…".to_string());
        }
        skipped = false;
        out.push(match o {
            Op::Same(l) => format!(" {l}"),
            Op::Add(l) => format!("+{l}"),
            Op::Del(l) => format!("-{l}"),
        });
    }
    let more = out.len().saturating_sub(MAX_LINES);
    out.truncate(MAX_LINES);
    if more > 0 {
        out.push(format!("… and {more} more lines of changes"));
    }
    format!("{added} line{} added, {removed} removed:\n{}", plural(added), out.join("\n"))
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

enum Op<'a> {
    Same(&'a str),
    Add(&'a str),
    Del(&'a str),
}

/// Longest common subsequence of lines, then a walk that turns it into keep, remove and add steps.
fn lcs_ops<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<Op<'a>> {
    // trim the common start and end first; most edits touch a small middle
    let start = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let end = a[start..].iter().rev().zip(b[start..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (am, bm) = (&a[start..a.len() - end], &b[start..b.len() - end]);
    let (n, m) = (am.len(), bm.len());
    let mut t = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i][j] = if am[i] == bm[j] { t[i + 1][j + 1] + 1 } else { t[i + 1][j].max(t[i][j + 1]) };
        }
    }
    let mut ops: Vec<Op> = a[..start].iter().map(|l| Op::Same(l)).collect();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && am[i] == bm[j] {
            ops.push(Op::Same(am[i]));
            i += 1;
            j += 1;
        } else if i < n && (j == m || t[i + 1][j] >= t[i][j + 1]) {
            // removals before additions, as diff tools show them
            ops.push(Op::Del(am[i]));
            i += 1;
        } else {
            ops.push(Op::Add(bm[j]));
            j += 1;
        }
    }
    ops.extend(a[a.len() - end..].iter().map(|l| Op::Same(l)));
    ops
}

#[cfg(test)]
mod tests {
    use super::diff;

    #[test]
    fn a_new_file_shows_its_lines() {
        assert_eq!(diff(None, "a\nb"), "New file, 2 lines:\n+a\n+b");
    }

    #[test]
    fn a_change_shows_the_lines_around_it() {
        let before = "1\n2\n3\n4\n5\n6\n7\n8\n9";
        let after = "1\n2\n3\n4\nfive\n6\n7\n8\n9";
        assert_eq!(diff(Some(before), after), "1 line added, 1 removed:\n 3\n 4\n-5\n+five\n 6\n 7");
    }

    #[test]
    fn far_apart_changes_are_split_and_nothing_is_noted() {
        let before: String = (1..=20).map(|n| format!("{n}\n")).collect();
        let after: String = (1..=20).map(|n| match n { 2 => "two\n".to_string(), 19 => "nineteen\n".to_string(), n => format!("{n}\n") }).collect();
        let d = diff(Some(&before), &after);
        assert!(d.contains("-2\n+two") && d.contains("-19\n+nineteen") && d.contains("\n…\n"), "{d}");
        assert_eq!(diff(Some("same"), "same"), "No change: the file already holds exactly this.");
    }

    #[test]
    fn long_diffs_are_cut() {
        let after: String = (0..500).map(|n| format!("{n}\n")).collect();
        let d = diff(Some("x"), &after);
        assert!(d.lines().count() <= 82 && d.contains("more lines of changes"), "{d}");
    }
}
