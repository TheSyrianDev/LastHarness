//! The new-chat screen's words: who to greet, what to say, and the suggestion chips.

/// The real name in a passwd file for `user`: the first part of the GECOS field, if there is one.
pub fn gecos_name(passwd: &str, user: &str) -> Option<String> {
    let line = passwd.lines().find(|l| l.split(':').next() == Some(user))?;
    let gecos = line.split(':').nth(4)?;
    let name = gecos.split(',').next()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Who to greet: the name the person set, else the real name from the system, else the login name.
pub fn pick_name(pref: &str, passwd: &str, user: &str) -> String {
    let pref = pref.trim();
    if !pref.is_empty() {
        return pref.to_string();
    }
    gecos_name(passwd, user).unwrap_or_else(|| user.trim().to_string())
}

/// `pick_name` with the real passwd file and `$USER`.
pub fn system_name(pref: &str) -> String {
    let user = std::env::var("USER").unwrap_or_default();
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    pick_name(pref, &passwd, &user)
}

/// "{name} is back!" for someone with past chats, "Hello, {name}" otherwise. An empty name drops the name.
pub fn greeting(name: &str, returning: bool, rtl: bool) -> String {
    let name = name.trim();
    match (rtl, returning, name.is_empty()) {
        (false, true, false) => format!("{name} is back!"),
        (false, true, true) => "Welcome back!".into(),
        (false, false, false) => format!("Hello, {name}"),
        (false, false, true) => "Hello".into(),
        (true, true, false) => format!("أهلًا بعودتك يا {name}"),
        (true, true, true) => "أهلًا بعودتك".into(),
        (true, false, false) => format!("مرحبًا، {name}"),
        (true, false, true) => "مرحبًا".into(),
    }
}

/// Small suggestions that fill the message box: (label, text).
pub fn suggestions(rtl: bool) -> [(&'static str, &'static str); 4] {
    if rtl {
        [("خطط لمشروع", "ساعدني في التخطيط لمشروع: "), ("لخّص ملفًا", "لخّص هذا الملف: "), ("اسأل عن ملاحظاتي", "ماذا في ملاحظاتي عن "), ("دردشة فقط", "")]
    } else {
        [("Plan a project", "Help me plan a project: "), ("Summarise a file", "Summarise this file: "), ("Ask about my notes", "What do my notes say about "), ("Just chat", "")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = "root:x:0:0:root:/root:/bin/bash\nomar:x:1000:1000:Omar Saeed,,,:/home/omar:/bin/zsh\nbare:x:1001:1001::/home/bare:/bin/sh\n";

    #[test]
    fn real_name_comes_from_the_first_gecos_part() {
        assert_eq!(gecos_name(PASSWD, "omar").as_deref(), Some("Omar Saeed"));
        assert_eq!(gecos_name(PASSWD, "bare"), None);
        assert_eq!(gecos_name(PASSWD, "nobody"), None);
    }

    #[test]
    fn the_name_is_the_pref_then_the_real_name_then_the_login() {
        assert_eq!(pick_name(" Sam ", PASSWD, "omar"), "Sam");
        assert_eq!(pick_name("", PASSWD, "omar"), "Omar Saeed");
        assert_eq!(pick_name("  ", PASSWD, "bare"), "bare");
    }

    #[test]
    fn greeting_depends_on_past_chats() {
        assert_eq!(greeting("Omar", true, false), "Omar is back!");
        assert_eq!(greeting("Omar", false, false), "Hello, Omar");
        assert_eq!(greeting("", false, false), "Hello");
        assert!(greeting("Omar", true, true).contains("Omar"));
        assert!(greeting("Omar", false, true).starts_with("مرحبًا"));
    }
}
