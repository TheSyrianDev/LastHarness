//! "Always for this chat": a yes that is remembered for one kind of question, in one chat, until the app closes.
//! The core asks again for every new action; this only lets the window answer yes for the person when the same
//! tool asks for the same reason again. It is never saved, and it is never offered for anything that deletes,
//! installs or runs as root.

use std::collections::HashSet;

/// Can the person be offered "Always" for a question with this reason?
pub fn may_offer(reason: &str) -> bool {
    let r = reason.to_lowercase();
    !r.trim().is_empty() && !["delete", "install", "root", "sudo"].iter().any(|w| r.contains(w))
}

#[derive(Debug, Default)]
pub struct Always {
    grants: HashSet<(u64, String, String)>,
}

impl Always {
    /// Remember a yes for this chat. Returns false (and remembers nothing) when "Always" is not allowed for the reason.
    pub fn grant(&mut self, chat: u64, tool: &str, reason: &str) -> bool {
        may_offer(reason) && self.grants.insert((chat, tool.to_string(), reason.to_string()))
    }

    pub fn covers(&self, chat: u64, tool: &str, reason: &str) -> bool {
        may_offer(reason) && self.grants.contains(&(chat, tool.to_string(), reason.to_string()))
    }

    /// A chat that is gone takes its grants with it.
    pub fn forget_chat(&mut self, chat: u64) {
        self.grants.retain(|(c, _, _)| *c != chat);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_is_never_offered_for_deleting_installing_or_root() {
        for bad in ["deletes a file", "deletes files", "installs software", "runs a command as root", "", "  "] {
            assert!(!may_offer(bad), "{bad:?}");
        }
        for ok in ["changes files in the project", "uses the network", "reads outside the project folder", "this tool changes things"] {
            assert!(may_offer(ok), "{ok:?}");
        }
    }

    #[test]
    fn a_grant_covers_the_same_tool_and_reason_in_the_same_chat_only() {
        let mut a = Always::default();
        assert!(a.grant(1, "run_command", "uses the network"));
        assert!(a.covers(1, "run_command", "uses the network"));
        assert!(!a.covers(2, "run_command", "uses the network"), "another chat");
        assert!(!a.covers(1, "web_fetch", "uses the network"), "another tool");
        assert!(!a.covers(1, "run_command", "changes files in the project"), "another reason");
        assert!(!a.grant(1, "run_command", "deletes files"), "refused");
        assert!(!a.covers(1, "run_command", "deletes files"));
        a.forget_chat(1);
        assert!(!a.covers(1, "run_command", "uses the network"));
    }
}
