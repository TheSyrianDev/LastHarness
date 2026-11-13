# Alpha GUI smoke test: findings (2026-11-13)

Screens captured with grim in docs/screenshots/alpha/: chat.png, providers.png (Models and keys), settings.png, memory.png.

## Not covered by this run
- Create tab (image and video) and any GPU generation: not run. Navigation needs a pointer or keyboard driver, and no pointer tool (ydotool/xdotool) is installed here. The app's start-up views only open Providers, Settings and Memory, so Create could not be reached without clicking. No machine lock was taken.
- Clicking Chat, Providers, Models, Settings in sequence: not done for the same reason. Each page was opened by its debug start-up view (LASTHARNESS_VIEW) instead.

## Observed
- Models and keys page shows Main provider (67 models), the search box, Downloads and the installed list with "Use in chat" buttons. Renders without error.
- Chat, Settings and Memory pages open without crash.

## Open
- Add a click driver or keyboard path for Create and page-to-page navigation, then repeat this run.
