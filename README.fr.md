<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lastai-lockup-on-dark.svg">
    <img alt="Last AI" src=".github/assets/lastai-lockup-on-light.svg" width="360">
  </picture>
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.ar.md">العربية</a> · <a href="README.zh-CN.md">简体中文</a> · Français · <a href="README.de.md">Deutsch</a>
</p>

<p align="center"><em>Le dernier harnais dont vous aurez besoin.</em><br>
Un harnais d'IA local d'abord, construit autour d'une équipe : des départements, et les travailleurs qui y sont.</p>

> [!WARNING]
> **Alpha.** Last AI est un logiciel en phase initiale et n'est utilisé sur une seule machine pour l'instant. Attendez-vous à des aspérités et à des changements incompatibles. Last AI (l'application de bureau et l'application terminal `la`) est testé sous Linux (Omarchy) et Windows (testé par l'auteur). La version macOS est en alpha précoce : elle démarre dans la CI, mais personne ne l'utilise encore au quotidien.

<p align="center">
  <a href="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: PolyForm Noncommercial 1.0.0" src="https://img.shields.io/badge/license-PolyForm--Noncommercial--1.0.0-blue"></a>
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange">
</p>

Last AI est un harnais d'IA local d'abord, pour l'ordinateur d'une seule personne. Un programme central (`lastaid`) gère la conversation, l'équipe (les départements et leurs travailleurs), les outils, la mémoire et l'apprentissage. Il est indépendant du modèle, sans enfermement chez un fournisseur. Il fonctionne avec n'importe quel modèle ou fournisseur : des serveurs locaux comme llama-swap, llama.cpp, Ollama et LM Studio, ou n'importe quelle API cloud compatible OpenAI ou de style Anthropic (testé jusqu'ici avec llama.cpp via llama-swap). Les interfaces sont des clients légers de l'API locale du programme central : une application terminal, une page dans le navigateur et des canaux de messagerie.
Il se souvient d'une session à l'autre (des notes que vous pouvez lire et modifier en Markdown), apprend vos habitudes sous forme de règles que vous approuvez, et peut confier les tâches importantes à des départements de travailleurs aux rôles définis, qui planifient, exécutent et vérifient le travail.
Les notes et règles sensibles ne sont utilisées qu'avec un modèle présent sur cette machine. Les permissions sont à plusieurs niveaux : les lectures sont libres, les écritures demandent une seule fois, et tout ce qui est risqué demande toujours.
Il fonctionne sous Linux et Windows ; macOS en est à l'alpha précoce.

## Compilation

```
cargo build --release
```

Cela produit quatre programmes dans `target/release/` : `lastaid` (le programme central), `la` et `lastai` (le même client terminal sous deux noms) et `lastai-gui` (l'application de bureau). L'application de bureau prend quelques minutes à compiler la première fois ; `cargo build --release -p lh-core -p lh-tui` ne compile que le programme central et le terminal.

## Premier lancement

```
target/release/la setup        # twelve plain questions; Enter accepts each default
target/release/lhd             # start the core (it reads the settings file)
target/release/la              # chat
target/release/lastai-gui # or the desktop app (needs the core running)
```

L'application de bureau trouve elle-même le programme central et son jeton (options : `--base`, `--token-file`). Elle a un aspect par défaut sobre que vous pouvez changer : placez un fichier `.toml` dans `~/.config/lastai/themes/` (les six thèmes intégrés, chacun avec trois familles en clair et en sombre, se trouvent dans `crates/lh-gui/themes/` comme exemples). Appuyez sur Ctrl+N pour une nouvelle conversation.

La version Omarchy (`LastAI-<version>-omarchy-x86_64.tar.gz`, installée avec son `install-omarchy.sh`) propose aussi l'aspect de bureau propre à Omarchy, ainsi qu'un aspect « Omarchy (transparent) », un verre liquide aux couleurs d'Omarchy. L'aspect transparent est aussi translucide que la règle de fenêtre Hyprland pour la fenêtre de l'application (classe `lastai-gui`) le permet ; sans règle, il reste opaque. Les autres versions (AppImage, Windows, macOS, ou une compilation source avec `--no-default-features`) n'incluent pas les aspects Omarchy. Pour en ajouter un, mettez cette ligne dans `~/.config/hypr/hyprland.lua` (0.85 est la part visible de la fenêtre) :

```lua
o.window("lastai-gui", { opacity = "0.85 override 0.85 override" })
```

Pour lancer le programme central à chaque connexion : `lh service preview`, puis `lh service install` (un service utilisateur systemd ; pas besoin de sudo ; fermer le terminal n'arrête jamais le programme central).

## Fournisseurs et modèles

Les fournisseurs sont les serveurs qui répondent aux conversations : un llama-swap ou llama.cpp local, Ollama, LM Studio, ou une API cloud compatible OpenAI ou de style Anthropic. Ajoutez-les et testez-les dans la page Modèles et clés de l'application de bureau, ou avec `lh settings`. Les clés d'API sont lues depuis des variables d'environnement que vous nommez ; rien n'est envoyé à un fournisseur que vous n'avez pas ajouté.

Dans la page Modèles et clés, recherchez des modèles GGUF sur Hugging Face. Les résultats sont regroupés selon leur adéquation avec la mémoire de cet ordinateur (Tient en mémoire, Juste, Trop grand, Taille inconnue). Téléchargez un fichier dans `~/AI/Models/`. Un modèle téléchargé qu'un routeur llama-swap ne sert pas encore peut être enregistré avec son bouton Run, qui sauvegarde les fichiers du routeur, valide la matrice et redémarre llama-swap. Les modèles déjà servis affichent « Use in chat ». La recherche nécessite le réseau ; sans lui, la page le signale.

## Create (images et vidéos)

Le bloc Create de la page Modèles et clés produit des images (Z-Image Turbo) et de courtes vidéos (Wan 2.2 TI2V-5B, en MP4) grâce à un ComfyUI local sur `127.0.0.1:8188`. Saisissez une consigne, choisissez Image ou Video, puis appuyez sur Generate. La liste des tâches affiche la progression et, une fois terminée, le résultat directement (l'image, ou la première image de la vidéo). Les fichiers sont enregistrés dans `~/AI/Create/`. Les tâches d'image et de vidéo prennent le verrou de la machine et libèrent la mémoire GPU ensuite ; elles attendent donc quand une autre session détient le verrou. ComfyUI et ses fichiers de modèles ne font pas partie de ce dépôt.

## Recherche web

`web_search` utilise une instance SearXNG. Si les réglages n'en nomment aucune, Last AI en cherche une sur `127.0.0.1:8888`. Si rien ne répond, l'outil le signale et suggère de lancer SearXNG ou de définir son adresse.

## Installation et compilation de version

- Source : `cargo build --release` (nécessite Rust stable ; sous Linux, l'application de bureau a besoin de GPUI et des bibliothèques système habituelles).
- AppImage Linux : `packaging/build-appimage.sh` (nécessite `linuxdeploy` ; voir l'en-tête du script).
- Omarchy : `scripts/install-omarchy.sh`.

## Commandes

Chaque commande a `help`, qui fonctionne sans que le programme central tourne, par exemple `lh team help`.

- `lastaid` est le programme central. Il écoute uniquement sur `127.0.0.1:7842`. Les réglages viennent de `config.toml` ; une option de ligne de commande remplace le fichier. Le jeton dont les clients ont besoin est écrit dans `$XDG_RUNTIME_DIR/lastai/token`.
- `la` et `lastai` ouvrent la conversation terminal. Options : `--url`, `--token-file`, `--model`, `--ask-all`, `--incognito`.
- `lh setup` : les questions du premier lancement. `lh settings` : voir et modifier chaque réglage (`set`, `reset`).
- `lh memory ...` : les notes que l'assistant conserve. `lh learn ...` : les règles, le style, les compétences et les suggestions qui attendent votre décision.
- `lh team ...` : rôles, types de modèles, départements, donner un objectif à un département (`lh team run`), travailleurs, mode absence, dossiers de plugins.
- `lh mcp ...` et `lh trust <folder>` : les serveurs MCP (rien ne tourne avant votre approbation) et les dossiers de projet autorisés à fournir leur propre configuration.
- `lh devices ...` : appairer des navigateurs (`lh devices pair`, puis `lh devices accept <PIN>`), les lister et les retirer.
- `lh channels ...` : les canaux Telegram, Matrix, Slack et Signal (tous désactivés tant que vous ne les activez pas).
- `lh export <folder>` et `lh import <folder>` : transférer la mémoire, les règles, le style, les rôles et les départements.
- `lh service ...` : lancer le programme central à la connexion.

La page du navigateur est servie en HTTPS lorsque `web.listen` est défini (par exemple `lh settings set web.listen 127.0.0.1:7843`) ; un navigateur s'appaire d'abord avec un PIN.

## Où vivent les fichiers

- Réglages : `~/.config/lastai/config.toml` (les prix de dépense facultatifs se trouvent dans `spend.toml` à côté).
- Données : `~/.local/share/lastai/` : `lh.db` (journal d'activité, règles, index), `memory/` (vos notes en Markdown), `skills/`, `team/` (rôles et départements), `mcp.json`, `web/` (le certificat de la porte navigateur).
- Jeton : `$XDG_RUNTIME_DIR/lastai/token`.

## Pas encore construit

- Une application téléphone, et tout design visuel final (pas encore de logo, de style de nom ni de couleurs de marque). La page du navigateur et l'aspect par défaut de l'application de bureau sont volontairement sobres.
- macOS ; programmes d'installation ; mises à jour signées ; les rapports de plantage sont seulement préparés, pas envoyés.
- Le relais hébergé, l'abonnement et la synchronisation de la mémoire.
- Le chargement et le déchargement de modèles locaux par les agents (ils peuvent déjà lister les modèles, en choisir un parmi vos fournisseurs et proposer un nouveau fournisseur à approuver).
- L'installation d'un plugin depuis un lien (dossiers uniquement), et les compétences au niveau du projet.
- Aucun canal de messagerie n'a encore été essayé avec un vrai compte ; ils sont testés avec des services simulés.

Tests : `cargo test --workspace` (le test du vrai modèle d'embedding est ignoré avec `-- --skip real_embeddinggemma`). Licence : PolyForm Noncommercial 1.0.0 (voir `LICENSE` et `COMMERCIAL-LICENSING.md`) ; `third_party/` conserve ses propres licences.
