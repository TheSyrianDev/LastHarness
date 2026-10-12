# For Omar: decisions waiting for you

Everything below was built while you were away (Milestones 1 to 7 and a hardening pass). These are the choices only you can make, ordered by how much work each one blocks. Each has my recommended answer; saying "yes" to a question accepts it. Details are in `.grill/parked.md` and `docs/milestone-N-report.md`.

## Blocks the most work

1. **GUI and branding design.** The native desktop app (Milestone 8), the phone app (9), themes, the worker flow diagram and the look of the browser page all wait for your design with the planning session. The browser page is unstyled on purpose. Nothing else can start on these until you decide the look and the toolkit tests. Recommended: start with the toolkit test week from the roadmap (Slint, egui, iced, GPUI, Arabic text and a screen reader) while you design. Do that?
2. **How should first run be presented?** `lh setup` asks about 12 plain questions, one at a time. The other options are three one-choice profiles (Careful, Balanced, Hands-off) with an advanced view, or questions asked only when needed. Recommended: profiles for the first run, plus questions at the moment they matter, with today's step-by-step kept as the advanced view. Use that?
3. **How bold should away mode be?** Today, while you are away, a plan is approved without asking and every permission question is answered no (the step is set aside and listed when you return). The spec lets the assistant decide for you using what it knows about you. Recommended: keep it blunt until you have used it for a while. Keep it?
4. **May a department lead accept a permission request on its own?** Today every request goes straight to you; the middle levels never say yes. Recommended: no, never; only you (or away mode's "no") decide. Keep it that way?

## Needs your word before it can be used

5. **Channel tokens.** A messaging token must be the name of an environment variable (`env:NAME`); the system secret store is not wired for channels. Recommended: add the secret store, unlocked when you log in, and keep `env:NAME` as the fallback. Do that?
6. **Cloud model run.** The Milestone 1 exit test has only a local half; the cloud half sends requests off the machine. Recommended: run it once with a model and key you name, spending cap set to a few dollars. Go ahead when you give the key and cap?
7. **Should `assistant` mode be allowed on a channel?** In that mode, anyone who controls an allow-listed chat account can approve tool use with yes or no. Recommended: allow it for Telegram, Matrix and Signal; refuse it for Slack. Use that?
8. **What may a paired phone browser do?** It can chat, edit memory, review rules, run departments and change most settings. It cannot reach MCP servers, folder trust, export, import, plugin install, or the provider, permission, web, image and spending settings. Recommended: keep exactly that. Keep it?

## Small defaults I chose that you may want to change

9. **Sensitive by default.** An unsure judge counts a note as sensitive, so "Lives in Berlin" never reaches a cloud model. Recommended: keep it safe and flip notes yourself in review. Keep it?
10. **"Forget" deletes for good** (the note, its history text and versions). Recommended: keep it; archive is for changed facts. Keep it?
11. **Incognito** still records tool names and permission answers (no text) in the activity log as a security record. Recommended: keep it. Keep it?
12. **Web fetch blocks local and private addresses**, and ignores proxy settings unless `web.use_proxy` is on. Recommended: keep it blocked with no allow list for now. Keep it?
13. **Rules and the style profile do not reach department workers**, to keep their prompts small. Recommended: send the style profile to the writer role only. Do that?
14. **Channel chats are remembered like any session.** Recommended: make channel chats incognito. Do that?
15. **Move `--allow` rules and `--embed-*` options into the settings list.** Recommended: yes for allow rules, no for the embedding paths. Do that?

## Housekeeping

16. **The Seagate drive.** It was not cleanly unmounted and damaged the backup repository twice. Recommended: unmount it, run `sudo fsck.exfat /dev/sda1`, delete the two `LastHarness.git.corrupt-*` copies, and always unmount before unplugging. Do that (it needs your password, so I cannot)?
17. **"Clippy".** Closed 2026-10-12. The user read it as CLIP, the image-and-text model, and said the app has nothing to do with it. Clippy is only the Rust linter the agents run. No action.
18. **Tray icon.** It needs the GUI toolkit decision from item 1, so it is parked until then. Fine?
