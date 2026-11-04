# Borrowing ideas from iiSU (plan, not decided)

Status: proposal for the user. Nothing here is a decision until the user says so.

## What iiSU is

iiSU (github.com/iisu-network/iiSU, iisu.network) is an Android launcher for emulation on handhelds, in alpha (0.0.7.4, 2026-07-31). It calls itself "visuals-first" and draws on the PSP XMB, the 3DS and the Wii U.

What I checked (2026-11-04): the GitHub README, the site, the 0.0.7.4 release notes, and RetroDodo's 0.0.7 write-up. I did not watch the walkthrough video (youtube.com/watch?v=sM43EAdfAe8). The text sources name features but rarely say how they look, so the visual notes below are partly my reading. Someone should watch the video before we copy any feel.

## Licence check

- The GitHub repo has no LICENSE file. It holds a README, an `updates/catalog.json` and screenshots. The app source is not public.
- The site says "© 2025-2026 iiSU" and links a "DMCA & Licensing" page.
- With no licence, all rights are reserved. **We borrow ideas only.** We copy no code, assets, icons, fonts, music or screenshots, and we use no iiSU names ("Title Swivel", "domino", "iiSU") in our UI. We recreate everything ourselves in our own style. Cal Sans is iiSU's font; we keep our own fonts.

## What to borrow

| Idea (iiSU) | What it is there | Our version | Where in our code | Effort | Risk |
|---|---|---|---|---|---|
| Glass context-menu buttons | Frosted buttons inside menus, matched across panels and popups | Menu rows and pill buttons drawn with the `controls` glass, with energy on hover/press | `style.rs` `surface_glass`, `glass_layer`, `GlassParams.energy`; theme `[material] controls` | S | Low. The shader and `controls` knob exist. Watch the GPU cost of many small glass layers. |
| Sharper focus visuals | A focus ring that glides between items instead of jumping | One focus highlight per list that eases from item to item | `motion.rs` `Glide`/`Eases`; lists in `ui.rs`, `sidebar.rs` | M | Medium. Keyboard focus in GPUI must stay correct. Screen readers must not lose focus. |
| Title swivel | Title art turns or slides when the selection changes | A small tilt-and-settle of the chat or run title when you switch | `motion.rs` new ease; title in `chrome.rs` | S | Low. Easy to overdo, so keep it small and off when motion is reduced. |
| Domino transitions | Items enter one after another with a short stagger | Staggered entry for sidebar rows, panel cards and menu rows | `motion.rs` `Reveal` plus a per-index delay | S–M | Low. Stagger must stay short (under ~150 ms total) so the app never feels slow. |
| Effects toggles / low power | Low power mode, per-feature switches | One "Effects" setting: Full / Calm / Off. It controls glass, stagger, swivel and blur | `prefs.rs`, `settings.rs`; `Pal::from(.., reduce_motion, ..)` already takes a flag | S | Low. Also helps the 4 GB GPU. |
| Layouts: XMB vertical, carousel | Same library shown as grid, vertical list or horizontal carousel | A layout switch for one list we have: chats or departments as list or carousel | `layout.rs`, `sidebar.rs` | L | High. Our app is a chat desk, not a game shelf. Only worth it if a screen has big items to browse. |
| Global search | L+R opens one search over ROMs, apps, collections | One shortcut opens search over chats, memory, skills, settings and departments | `palette.rs` (a command palette exists) plus core search API | M | Medium. It needs the core to search memory well. |
| Home widgets | Pages of widgets (achievements, playtime, "been a while") | A home page with cards: recent runs, memory notes, model status, "pick up where you left off" | `panels.rs`, `greeting.rs` | M–L | Medium. Risk of clutter; it goes against "Quiet desk". |
| Onboarding picks theme colour and browsing style | First run asks colour, layout and look with live previews | First run asks theme colour (our wheel) and effects level, with a live preview | `wheel.rs`, `appearance.rs`, onboarding in core milestone 5 | M | Low. Must stay short and skippable. |
| Themes and marketplace | Importable theme and platform packs (ZIP) | Theme packs as a folder of TOML plus images, import from a file. A shared gallery later | `theme.rs` `ThemeSet::load_with` | M (import), L (market) | High for the market: hosting, trust and safety of shared files. |
| Light and dark previews for layouts | Each layout shows small light and dark previews | Theme picker shows both variants side by side | `appearance.rs`, `ThemeSet::variant` | S | Low. |

## Phases (ordered)

1. **Effects setting and glass controls.** Add the Full / Calm / Off setting. Draw menu buttons and pills with the `controls` glass and its hover energy. Small, uses what we have, and gives us the safety switch before we add motion.
2. **Motion set.** Staggered entry, the gliding focus highlight, and the title swivel. All of them obey the Effects setting and reduced motion.
3. **Search everywhere.** Grow the command palette into one search over chats, memory, skills and settings.
4. **First run look.** Theme colour and effects level picked with a live preview, plus light/dark previews in the theme picker.
5. **Home cards.** A calm home page with a few cards, only if the user wants one.
6. **Theme packs.** Import from a folder or ZIP. A marketplace stays parked.
7. **Alternate layouts (carousel / vertical).** Last, and only if a screen needs it.

**Recommended first slice:** phase 1. It is small (a few days), low risk, uses the shader and `[material] controls` we already have, and it makes every later effect safe to try on the 4 GB GPU.

## Questions for the user (one at a time)

1. Should phase 1 (an Effects setting with Full / Calm / Off, plus glass menu buttons) be the first slice? Recommended: yes.
2. Should Calm be the default effects level, so glass stays but stagger and swivel are off until chosen? Recommended: yes.
3. Should someone watch iiSU's walkthrough video and describe the motion before we design the stagger and swivel? Recommended: yes.
4. Should home cards (phase 5) wait until you design a home page with the planning session? Recommended: yes.
5. Should the theme marketplace stay parked, with only local theme pack import for now? Recommended: yes.

Sources: github.com/iisu-network/iiSU, iisu.network, iisu.network/news/iisu-alpha-0-0-7-4-mizu-and-media, retrododo.com/iisu-007-is-here-with-some-welcomed-upgrades/.
