# Looks

A look is the whole appearance of the window: its colours, its background, its corner radius, its glass, and the fonts
and icons it asks for. The built-in looks are compiled into the app; anyone can add their own without touching the
source, and the app notices the change while it runs.

## Where looks live

```
~/.config/lastai/themes/
```

(`$XDG_CONFIG_HOME/lastai/themes` when `XDG_CONFIG_HOME` is set.)

One folder per look:

```
~/.config/lastai/themes/
  Ember/
    theme.toml
    background.png     # optional
```

- The **folder's name is the look's name** on the Appearance page, so `Ember/` is the look "Ember" whatever its
  `name` key says.
- A file called `background.png`, `background.jpg`, `background.jpeg` or `background.webp` beside the `theme.toml` is
  that look's background, used when `[background] kind = "image"` and no `image` path is given. Only the first one
  found is used.
- A single `.toml` file directly in the folder (`themes/Old.toml`) still loads, as the older shape of a look. A folder
  is the better home: it can carry its background, and it is what **Export** writes.
- The app re-reads the folder while it runs: saving a file or dropping in a new folder updates the Appearance page and,
  if that look is the one in use, the window.

## The shortest look that works

Every key has a default, so a look can be one line. Bare minimum:

```toml
[colors]
accent = "#a6c4ac"
```

That is Calm light with a different accent. Everything else is inherited from the defaults, not from another look:
there is no `extends`, so to start from a built-in, copy its file out of `crates/lh-gui/themes/` (or use **Export**
on the Appearance page) and edit it.

A more complete example:

```toml
name = "Ember"
mode = "dark"                    # light or dark; decides which way unset colours lean

[colors]
background = "#191413"
surface = "#241c1a"
text = "#f0e6e2"
muted = "#a89a95"
accent = "#e07a5f"
accent_text = "#1a0f0c"
border = "#3a2c28"

[background]
kind = "image"                   # solid, gradient or image
image = "background.png"         # relative to this file
image_opacity = 0.55

[shape]
roundness = 8

[material]
panels = "frosted"               # see below; omit for flat panels
```

## Keys

### Top level

| key | meaning |
| --- | --- |
| `name` | The look's name, used when it is not in a folder. A folder's own name always wins. |
| `mode` | `light` or `dark`. Sets the direction of unchanged colours and the app's own contrast checks. |

### `[colors]`

`#rgb`, `#rrggbb` or `#rrggbbaa` (the last two digits are opacity).

| key | meaning |
| --- | --- |
| `background` | Behind everything. |
| `surface` | Panels, cards and your own message bubbles. |
| `sunken` | Input fields and the inside of wells. |
| `text` | Body text. |
| `muted` | Secondary text and labels. |
| `accent` | The brand accent: selection, focus in the sidebar, the send button. |
| `accent_text` | Text on top of `accent`. |
| `button` / `button_hover` | Buttons, at rest and hovered. |
| `border` | Hairlines. |
| `danger` | Errors and destructive actions. |
| `warn` | Cautions that are not errors. Optional; a readable amber otherwise. |
| `good` | Success: done ticks, saved notices. Optional; a readable green otherwise. |
| `focus_ring` | The ring around the control with the keyboard. Optional; `accent` otherwise. |
| `bubble` | Your own message bubble. Optional; `button` otherwise. |

The app measures contrast for these pairs and tells you on the Appearance page when one is too low; it does not stop
you, because some looks are deliberately quiet.

### `[background]`

| key | meaning |
| --- | --- |
| `kind` | `solid`, `gradient` or `image`. |
| `from`, `to` | The two stops of a gradient, and the flat colour of a solid. |
| `angle` | Gradient direction, in degrees. |
| `image` | A path, relative to the theme file. |
| `image_opacity` | How strongly the picture shows, 0 to 1. |

A picture can also simply be dropped in as `background.png` beside the `theme.toml`, with `kind = "image"`.

### `[shape]`

| key | meaning |
| --- | --- |
| `roundness` | Corner radius of panels, in pixels. Keep it at 10 or less: the app is drawn on hairline geometry. |
| `match_window` | Take the window's own radius from the desktop (Hyprland reports one) and curve inner controls to match. |

### `[effects]`

| key | meaning |
| --- | --- |
| `blur` | Ask the desktop to blur what is behind the window. |
| `shadow_blur`, `shadow_y`, `shadow_color` | The window's shadow. All zeros for none. |

### `[animation]`

| key | meaning |
| --- | --- |
| `duration_ms` | Length of hover and press motion. `0` turns it off. |

### `[material]`

Which surfaces are liquid glass. A surface set to `none` (the default) is a flat fill with its hairline.

| key | surface |
| --- | --- |
| `panels` | The sidebar, the rail and cards. |
| `popups` | Menus, pop-up cards and dialogs. |
| `side_panel` | The panel on the right. |
| `controls` | Switch knobs, slider knobs and the segmented thumb. Defaults to `frosted`, even in a flat look. |

Each takes one of:

| value | what it is |
| --- | --- |
| `none` | Flat. |
| `regular` | Frosted, lightly bent. |
| `clear` | See-through, a strong bend at the edge and a sharp rim. |
| `regular_tinted` | Frosted, carried by the look's own colour. |
| `clear_tinted` | See-through, carried by the look's own colour. |

Also in the table: `blur` (pixels of blur behind the glass, default 4), `refraction` (pixels the edge bends, default 6)
and `rim` (strength of the bright edge, 0 to 1, default 0.5).

Inside the app, Appearance → Glass has a switch that turns all glass off at once, and a picker per surface that steps
each one through the kinds above. Those choices are yours and live in `gui.toml`; they sit on top of whatever the look
asks for, and **Back to what the look says** removes them.

### `[fonts]`

Each is one family name or a list tried in order (the first installed one is used):

```toml
[fonts]
latin = ["Inter", "DejaVu Sans"]
arabic = "Noto Kufi Arabic"
mono = "JetBrains Mono"
```

An empty or missing entry means the look has no opinion.

### `[icons]`

An SVG per icon, named after the icon. Paths are relative to the theme file. A `folder` gives a whole set at once:
every `<name>.svg` in it is picked up, and a name written out separately wins over the folder:

```toml
[icons]
folder = "icons"                 # icons/send.svg, icons/search.svg, …
send = "my-send.svg"            # this one instead of icons/send.svg
```

Icons that are mirrored for right-to-left (`sidebar-collapse`, `sidebar-expand`, `panel`, the chevrons) are flipped by
the app, so draw each one once.

The names a look can draw are listed in `crates/lh-gui/src/icons.rs` (`NAMES`): `new-chat`, `search`, `send`,
`attach`, `stop`, `sidebar-collapse`, `sidebar-expand`, `lock`, `panel`, `close`, `widen`, `popout`, `chevron-right`,
`chevron-left`, `chevron-down`, `chats`, `departments`, `memory`, `settings`, `appearance`, `themes`, `chat`, `models`,
`learning`, `permissions`, `spending`, `web`, `tools`, `channels`, `data`, `activity`, `hooks`, `schedules`, `updates`,
`other`. Anything missing or unreadable falls back to the app's own line drawing, and the problem is listed with the
look on the Appearance page.

## Moving looks around

On the Themes page, each look's row has:

- **Duplicate** copies the look into a look folder of yours, named after it, and starts editing its name. The quickest
  way to start from a built-in.
- **Export** writes the look out as a folder with its `theme.toml` (and its background, if it has one) wherever you
  choose.
- **Rename** changes a look's name. A built-in is duplicated first, so the change lands on your copy.
- **Remove** deletes a look you added, after a second click to confirm. Built-ins cannot be removed.

At the top of the page:

- **Import a look** takes a look folder, or a single `.toml`, and copies it into `~/.config/lastai/themes/`. A name
  already in use gets a number.
- **Open the folder** opens the themes folder in your file manager.

Unknown keys are an error, not something silently ignored: a misspelled key shows up as a problem next to the look
instead of a setting that does nothing. Tables from older versions (`[scene]`, `[glass]`, `[gel]`, `[focus]`,
`[motes]`) are the exception — they are read and ignored, so an old file still loads.
