<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lastai-lockup-on-dark.svg">
    <img alt="Last AI" src=".github/assets/lastai-lockup-on-light.svg" width="360">
  </picture>
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.ar.md">العربية</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.fr.md">Français</a> · Deutsch
</p>

<p align="center"><em>Das letzte Harness, das du brauchst.</em><br>
Ein lokal-first KI-Harness, das um ein Team herum gebaut ist: Abteilungen und die Mitarbeiter darin.</p>

> [!WARNING]
> **Alpha.** Last AI ist frühe Software und wird bisher nur auf einem Rechner genutzt. Erwarte Ecken und Kanten sowie Änderungen, die bestehende Funktionen brechen. Last AI (die Desktop-App und die Terminal-App `la`) wird unter Linux (Omarchy) und Windows getestet (vom Autor getestet). Der macOS-Build ist eine frühe Alpha: Er startet in der CI, aber bisher nutzt ihn niemand im Alltag.

<p align="center">
  <a href="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: PolyForm Noncommercial 1.0.0" src="https://img.shields.io/badge/license-PolyForm--Noncommercial--1.0.0-blue"></a>
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange">
</p>

Last AI ist ein lokal-first KI-Harness für den Computer einer einzelnen Person. Ein Kernprogramm (`lastaid`) verwaltet das Gespräch, das Team (Abteilungen und ihre Mitarbeiter), Werkzeuge, Gedächtnis und Lernen. Es ist modellunabhängig und nicht an einen Anbieter gebunden. Es arbeitet mit jedem Modell oder Anbieter: lokalen Servern wie llama-swap, llama.cpp, Ollama und LM Studio oder jeder cloudbasierten API, die mit OpenAI kompatibel ist oder dem Anthropic-Stil folgt (bisher getestet mit llama.cpp über llama-swap). Die Oberflächen sind schlanke Clients der lokalen API des Kernprogramms: eine Terminal-App, eine Browserseite und Messaging-Kanäle.
Es erinnert sich über Sitzungen hinweg (Notizen, die du als Markdown lesen und bearbeiten kannst), lernt deine Gewohnheiten als Regeln, die du genehmigst, und kann größere Aufgaben an Abteilungen mit Rollen-Mitarbeitern übergeben, die die Arbeit planen, ausführen und prüfen.
Sensible Notizen und Regeln werden nur mit einem Modell auf diesem Rechner verwendet. Die Berechtigungen sind gestuft: Lesen ist frei, Schreiben fragt einmal nach, und alles Riskante fragt immer.
Es läuft unter Linux und Windows; macOS ist eine frühe Alpha.

## Bauen

```
cargo build --release
```

Dies erzeugt vier Programme in `target/release/`: `lastaid` (das Kernprogramm), `la` und `lastai` (derselbe Terminal-Client unter zwei Namen) und `lastai-gui` (die Desktop-App). Die Desktop-App braucht beim ersten Mal ein paar Minuten zum Bauen; `cargo build --release -p lh-core -p lh-tui` baut nur den Kern und das Terminal.

## Erster Start

```
target/release/la setup        # twelve plain questions; Enter accepts each default
target/release/lhd             # start the core (it reads the settings file)
target/release/la              # chat
target/release/lastai-gui # or the desktop app (needs the core running)
```

Die Desktop-App findet den Kern und sein Token selbst (Optionen: `--base`, `--token-file`). Sie hat ein schlichtes Standarddesign, das du ändern kannst: Lege eine `.toml`-Datei in `~/.config/lastai/themes/` ab (die sechs eingebauten Themes, jeweils drei Familien in Hell und Dunkel, liegen als Beispiele in `crates/lh-gui/themes/`). Drücke Strg+N für einen neuen Chat.

Die Omarchy-Version (`LastAI-<version>-omarchy-x86_64.tar.gz`, installiert mit ihrer `install-omarchy.sh`) bietet außerdem das eigene Desktop-Design und ein Design „Omarchy (transparent)“, also Flüssigglas in den Omarchy-Farben. Das transparente Design ist so durchsichtig, wie es die Hyprland-Fensterregel für das Fenster der App (Klasse `lastai-gui`) vorgibt; ohne Regel bleibt es undurchsichtig. Andere Versionen (AppImage, Windows, macOS oder ein Quellbuild mit `--no-default-features`) enthalten die Omarchy-Designs nicht. Um eines hinzuzufügen, setze diese Zeile in `~/.config/hypr/hyprland.lua` (0.85 ist der sichtbare Anteil des Fensters):

```lua
o.window("lastai-gui", { opacity = "0.85 override 0.85 override" })
```

Um den Kern bei jeder Anmeldung zu starten: `lh service preview`, dann `lh service install` (ein systemd-Benutzerdienst; ohne sudo; das Schließen des Terminals beendet den Kern nie).

## Anbieter und Modelle

Anbieter sind die Server, die Chats beantworten: ein lokales llama-swap oder llama.cpp, Ollama, LM Studio oder eine cloudbasierte API, die mit OpenAI kompatibel ist oder dem Anthropic-Stil folgt. Füge sie auf der Seite „Modelle und Schlüssel“ der Desktop-App hinzu und teste sie dort oder mit `lh settings`. API-Schlüssel werden aus Umgebungsvariablen gelesen, die du benennst; an einen Anbieter, den du nicht hinzugefügt hast, wird nichts gesendet.

Auf der Seite „Modelle und Schlüssel“ kannst du auf Hugging Face nach GGUF-Modellen suchen. Die Ergebnisse sind danach gruppiert, ob sie in den Speicher dieses Rechners passen (Passt in den Speicher, Knapp, Zu groß, Größe unbekannt). Lade eine Datei nach `~/AI/Models/` herunter. Ein heruntergeladenes Modell, das ein llama-swap-Router noch nicht bereitstellt, kann mit seinem Run-Knopf registriert werden; dieser sichert die Router-Dateien, prüft die Matrix und startet llama-swap neu. Bereits bereitgestellte Modelle zeigen „Use in chat“. Die Suche braucht eine Netzwerkverbindung; ohne sie meldet die Seite das.

## Create (Bilder und Video)

Der Create-Block auf der Seite „Modelle und Schlüssel“ erstellt Bilder (Z-Image Turbo) und kurze Videos (Wan 2.2 TI2V-5B, MP4) über ein lokales ComfyUI auf `127.0.0.1:8188`. Gib einen Prompt ein, wähle Image oder Video und drücke Generate. Die Auftragsliste zeigt den Fortschritt und, wenn er fertig ist, das Ergebnis direkt (das Bild oder das erste Bild des Videos). Dateien werden in `~/AI/Create/` gespeichert. Bild- und Videoaufträge nehmen die Maschinensperre und geben danach den GPU-Speicher frei; sie warten daher, solange eine andere Sitzung die Sperre hält. ComfyUI und seine Modelldateien sind nicht Teil dieses Repositorys.

## Websuche

`web_search` nutzt eine SearXNG-Instanz. Nennen die Einstellungen keine, sucht Last AI eine auf `127.0.0.1:8888`. Antwortet dort niemand, sagt das Werkzeug es und schlägt vor, SearXNG zu starten oder seine Adresse festzulegen.

## Installation und Release-Build

- Quellcode: `cargo build --release` (braucht Rust stable; unter Linux braucht die Desktop-App GPUI und die üblichen Systembibliotheken).
- Linux-AppImage: `packaging/build-appimage.sh` (braucht `linuxdeploy`; siehe den Kopf des Skripts).
- Omarchy: `scripts/install-omarchy.sh`.

## Befehle

Jeder Befehl hat `help`, das auch ohne laufenden Kern funktioniert, zum Beispiel `lh team help`.

- `lastaid` ist der Kern. Er lauscht nur auf `127.0.0.1:7842`. Die Einstellungen kommen aus `config.toml`; ein Kommandozeilen-Flag überschreibt die Datei. Das Token, das die Clients brauchen, wird nach `$XDG_RUNTIME_DIR/lastai/token` geschrieben.
- `la` und `lastai` öffnen den Terminal-Chat. Optionen: `--url`, `--token-file`, `--model`, `--ask-all`, `--incognito`.
- `lh setup`: Fragen beim ersten Start. `lh settings`: jede Einstellung ansehen und ändern (`set`, `reset`).
- `lh memory ...`: die Notizen, die der Assistent führt. `lh learn ...`: Regeln, Stil, Fähigkeiten und Vorschläge, die auf dich warten.
- `lh team ...`: Rollen, Modelltypen, Abteilungen, einer Abteilung ein Ziel geben (`lh team run`), Mitarbeiter, Abwesenheitsmodus, Plugin-Ordner.
- `lh mcp ...` und `lh trust <folder>`: MCP-Server (nichts läuft, bevor du zustimmst) und welche Projektordner ihre eigene Konfiguration mitbringen dürfen.
- `lh devices ...`: Browser koppeln (`lh devices pair`, dann `lh devices accept <PIN>`), auflisten und entfernen.
- `lh channels ...`: Telegram-, Matrix-, Slack- und Signal-Kanäle (alle ausgeschaltet, bis du sie aktivierst).
- `lh export <folder>` und `lh import <folder>`: Gedächtnis, Regeln, Stil, Rollen und Abteilungen verschieben.
- `lh service ...`: den Kern bei der Anmeldung starten.

Die Browserseite wird über HTTPS bereitgestellt, wenn `web.listen` gesetzt ist (zum Beispiel `lh settings set web.listen 127.0.0.1:7843`); ein Browser koppelt sich zuerst mit einer PIN.

## Wo Dateien liegen

- Einstellungen: `~/.config/lastai/config.toml` (optionale Ausgabenpreise in `spend.toml` daneben).
- Daten: `~/.local/share/lastai/`: `lh.db` (Aktivitätsprotokoll, Regeln, Indizes), `memory/` (deine Notizen als Markdown), `skills/`, `team/` (Rollen und Abteilungen), `mcp.json`, `web/` (das Zertifikat der Browser-Schnittstelle).
- Token: `$XDG_RUNTIME_DIR/lastai/token`.

## Noch nicht gebaut

- Eine Telefon-App und ein endgültiges visuelles Design (noch kein Logo, keine Namensgestaltung und keine Markenfarben). Die Browserseite und das Standarddesign der Desktop-App sind mit Absicht schlicht.
- macOS; Installationsprogramme; signierte Updates; Absturzberichte sind nur vorbereitet, werden aber nicht gesendet.
- Der gehostete Relay, das Abonnement und die Speicher-Synchronisation.
- Das Laden und Entladen lokaler Modelle durch Agenten (sie können bereits Modelle auflisten, eines aus deinen Anbietern auswählen und einen neuen Anbieter zur Genehmigung vorschlagen).
- Die Installation eines Plugins über einen Link (nur Ordner) und Projekt-Fähigkeiten.
- Kein Messaging-Kanal wurde bisher mit einem echten Konto getestet; sie werden mit simulierten Diensten getestet.

Tests: `cargo test --workspace` (der Test mit dem echten Embedding-Modell wird mit `-- --skip real_embeddinggemma` übersprungen). Lizenz: PolyForm Noncommercial 1.0.0 (siehe `LICENSE` und `COMMERCIAL-LICENSING.md`); `third_party/` behält seine eigenen Lizenzen.
