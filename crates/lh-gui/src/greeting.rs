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

/// A name from the system written in lower case gets capitals: "alex doe" becomes "Alex Doe". A name with
/// any capital already is left as the person wrote it.
pub fn tidy_name(name: &str) -> String {
    let name = name.trim();
    if name.chars().any(|c| c.is_uppercase()) {
        return name.to_string();
    }
    name.split(' ').map(|w| {
        let mut c = w.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    }).collect::<Vec<_>>().join(" ")
}

/// A hello for the time of day, `hour` being the local hour (0 to 23).
pub fn greeting_at(name: &str, hour: u32, rtl: bool) -> String {
    let name = tidy_name(name);
    let (en, ar) = match hour {
        5..=11 => ("Good morning", "صباح الخير"),
        12..=17 => ("Good afternoon", "مساء الخير"),
        _ => ("Good evening", "مساء الخير"),
    };
    match (rtl, name.is_empty()) {
        (false, false) => format!("{en}, {name}"),
        (false, true) => en.into(),
        (true, false) => format!("{ar} يا {name}"),
        (true, true) => ar.into(),
    }
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

    const PASSWD: &str = "root:x:0:0:root:/root:/bin/bash\nalex:x:1000:1000:Alex Doe,,,:/home/alex:/bin/zsh\nbare:x:1001:1001::/home/bare:/bin/sh\n";

    #[test]
    fn real_name_comes_from_the_first_gecos_part() {
        assert_eq!(gecos_name(PASSWD, "alex").as_deref(), Some("Alex Doe"));
        assert_eq!(gecos_name(PASSWD, "bare"), None);
        assert_eq!(gecos_name(PASSWD, "nobody"), None);
    }

    #[test]
    fn the_name_is_the_pref_then_the_real_name_then_the_login() {
        assert_eq!(pick_name(" Sam ", PASSWD, "alex"), "Sam");
        assert_eq!(pick_name("", PASSWD, "alex"), "Alex Doe");
        assert_eq!(pick_name("  ", PASSWD, "bare"), "bare");
    }

    #[test]
    fn greeting_follows_the_hour_and_tidies_the_name() {
        assert_eq!(greeting_at("alex", 8, false), "Good morning, Alex");
        assert_eq!(greeting_at("alex doe", 14, false), "Good afternoon, Alex Doe");
        assert_eq!(greeting_at("McKay", 22, false), "Good evening, McKay");
        assert_eq!(greeting_at("", 3, false), "Good evening");
        assert!(greeting_at("سامي", 9, true).starts_with("صباح الخير"));
        assert_eq!(tidy_name("سامي"), "سامي");
    }

    #[test]
    fn greeting_depends_on_past_chats() {
        assert_eq!(greeting("Alex", true, false), "Alex is back!");
        assert_eq!(greeting("Alex", false, false), "Hello, Alex");
        assert_eq!(greeting("", false, false), "Hello");
        assert!(greeting("Alex", true, true).contains("Alex"));
        assert!(greeting("Alex", false, true).starts_with("مرحبًا"));
    }
}
