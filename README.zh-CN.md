<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lastai-lockup-on-dark.svg">
    <img alt="Last AI" src=".github/assets/lastai-lockup-on-light.svg" width="240">
  </picture>
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.ar.md">العربية</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a>
</p>

<p align="center"><em>你需要的最后一个框架。</em><br>
一个本地优先的 AI 框架，围绕团队构建：部门，以及部门里的工作者。</p>

> [!WARNING]
> **Alpha 版。** Last AI 是早期软件，目前只在一台机器上使用。预计会有粗糙之处和破坏性变更。Last AI（桌面应用和 `la` 终端应用）在 Linux（Omarchy）和 Windows（作者本人测试）上经过测试。macOS 版本处于早期 alpha 阶段：它能在 CI 中启动，但还没有人日常使用。

<p align="center">
  <a href="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: PolyForm Noncommercial 1.0.0" src="https://img.shields.io/badge/license-PolyForm--Noncommercial--1.0.0-blue"></a>
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange">
</p>

Last AI 是一个为个人电脑设计的本地优先 AI 框架。核心程序（`lastaid`）负责保存对话、团队（部门及其工作者）、工具、记忆和学习能力，并且与模型无关，不绑定任何模型。它可以使用任何模型或服务商：llama-swap、llama.cpp、Ollama 和 LM Studio 等本地服务器，或任何兼容 OpenAI 的云 API 或 Anthropic 风格的云 API（目前已在 llama-swap 下的 llama.cpp 上测试）。前端是核心本地 API 的轻量客户端：终端应用、浏览器页面和消息频道。
它能跨会话记住内容（你可以用 Markdown 阅读和编辑这些笔记），把你的习惯学成你批准的规则，并能把较大的任务交给由角色工作者组成的部门，由他们来规划、执行和检查工作。
敏感的笔记和规则只会交给这台机器上的模型处理。权限分级：读取无需确认，写入询问一次，有风险的操作始终询问。
它可在 Linux 和 Windows 上运行；macOS 版本处于早期 alpha 阶段。

## 构建

```
cargo build --release
```

这会在 `target/release/` 中生成四个程序：`lastaid`（核心）、`la` 和 `lastai`（同一个终端客户端的两个名字），以及 `lastai-gui`（桌面应用）。桌面应用第一次构建需要几分钟；如果只需要核心和终端，可以运行 `cargo build --release -p lh-core -p lh-tui`。

## 首次运行

```
target/release/la setup        # twelve plain questions; Enter accepts each default
target/release/lhd             # start the core (it reads the settings file)
target/release/la              # chat
target/release/lastai-gui # or the desktop app (needs the core running)
```

桌面应用会自行找到核心及其令牌（参数：`--base`、`--token-file`）。它有简单的默认外观，你可以修改：把 `.toml` 文件放进 `~/.config/lastai/themes/`（六个内置主题，每个都有浅色和深色的三个系列，示例位于 `crates/lh-gui/themes/`）。按 Ctrl+N 开始新对话。

Omarchy 版本（`LastAI-<version>-omarchy-x86_64.tar.gz`，用其中的 `install-omarchy.sh` 安装）还提供桌面自己的外观，以及 "Omarchy (transparent)" 外观，即采用 Omarchy 配色的液态玻璃效果。透明外观的透明程度取决于应用窗口（类名 `lastai-gui`）的 Hyprland 窗口规则；没有规则时它保持不透明。其他版本（AppImage、Windows、macOS，或使用 `--no-default-features` 的源码构建）不包含 Omarchy 外观。要添加，请把这一行放进 `~/.config/hypr/hyprland.lua`（0.85 表示窗口显示的比例）：

```lua
o.window("lastai-gui", { opacity = "0.85 override 0.85 override" })
```

要让核心在每次登录时启动：先运行 `lh service preview`，再运行 `lh service install`（systemd 用户服务，无需 sudo；关闭终端不会停止核心）。

## 服务商与模型

服务商是负责回答对话的服务器：本地的 llama-swap 或 llama.cpp、Ollama、LM Studio，或兼容 OpenAI 或 Anthropic 风格的云 API。你可以在桌面应用的"模型与密钥"页面添加和测试它们，也可以使用 `lh settings`。API 密钥从你指定的环境变量中读取；未添加的服务商不会收到任何数据。

在"模型与密钥"页面，可以在 Hugging Face 上搜索 GGUF 模型。结果按是否适合这台电脑的内存分组（可放入内存、紧张、过大、大小未知）。把文件下载到 `~/AI/Models/`。已下载但尚未被 llama-swap 路由服务的模型，可以用它的 Run 按钮注册：这会备份路由文件、校验配置矩阵并重启 llama-swap。已经提供服务的模型会显示"Use in chat"。搜索需要网络；没有网络时页面会提示。

## Create（图像和视频）

"模型与密钥"页面上的 Create 区域通过 `127.0.0.1:8188` 上的本地 ComfyUI 生成图像（Z-Image Turbo）和短视频（Wan 2.2 TI2V-5B，MP4 格式）。输入提示词，选择 Image 或 Video，然后按 Generate。任务列表会显示进度，完成后直接显示结果（图像，或视频的第一帧）。文件保存在 `~/AI/Create/`。图像和视频任务会占用机器锁并在完成后释放 GPU 内存，因此在其他会话持有锁时会等待。ComfyUI 及其模型文件不属于本仓库。

## 网页搜索

`web_search` 使用 SearXNG 实例。如果设置中没有指定，Last AI 会在 `127.0.0.1:8888` 查找。如果那里没有响应，工具会说明情况，并建议启动 SearXNG 或设置其地址。

## 安装与发布构建

- 源码：`cargo build --release`（需要 Rust stable；在 Linux 上，桌面应用需要常用的 GPUI 和系统库）。
- Linux AppImage：`packaging/build-appimage.sh`（需要 `linuxdeploy`；见脚本开头的说明）。
- Omarchy：`scripts/install-omarchy.sh`。

## 命令

每个命令都有 `help`，无需核心运行即可使用，例如 `lh team help`。

- `lastaid` 是核心。它只监听 `127.0.0.1:7842`。设置来自 `config.toml`；命令行参数会覆盖该文件。客户端需要的令牌写在 `$XDG_RUNTIME_DIR/lastai/token`。
- `la` 和 `lastai` 打开终端对话。选项：`--url`、`--token-file`、`--model`、`--ask-all`、`--incognito`。
- `lh setup`：首次运行的问题。`lh settings`：查看和修改每项设置（`set`、`reset`）。
- `lh memory ...`：助手保存的笔记。`lh learn ...`：等待你处理的规则、风格、技能和建议。
- `lh team ...`：角色、模型类型、部门、给部门下达目标（`lh team run`）、工作者、离开模式、插件文件夹。
- `lh mcp ...` 和 `lh trust <folder>`：MCP 服务器（在你批准之前不会运行任何内容），以及允许哪些项目文件夹提供自己的配置。
- `lh devices ...`：配对浏览器（`lh devices pair`，然后 `lh devices accept <PIN>`），列出和删除。
- `lh channels ...`：Telegram、Matrix、Slack 和 Signal 频道（默认全部关闭，需要手动开启）。
- `lh export <folder>` 和 `lh import <folder>`：迁移记忆、规则、风格、角色和部门。
- `lh service ...`：登录时启动核心。

设置 `web.listen` 后（例如 `lh settings set web.listen 127.0.0.1:7843`），浏览器页面通过 HTTPS 提供服务；浏览器需要先用 PIN 配对。

## 文件存放位置

- 设置：`~/.config/lastai/config.toml`（可选的花费价格在旁边的 `spend.toml` 中）。
- 数据：`~/.local/share/lastai/`：`lh.db`（活动日志、规则、索引）、`memory/`（你的 Markdown 笔记）、`skills/`、`team/`（角色和部门）、`mcp.json`、`web/`（浏览器入口的证书）。
- 令牌：`$XDG_RUNTIME_DIR/lastai/token`。

## 尚未构建

- 手机应用，以及任何最终视觉设计（尚无标志、名称样式或品牌色）。浏览器页面和桌面应用的默认外观有意保持朴素。
- macOS；安装包；签名更新；崩溃报告仅做了准备，尚未发送。
- 托管中继、订阅和记忆同步。
- 代理加载和卸载本地模型（它们现在已能列出模型、在你的服务商中选择一个，并提议新服务商供你批准）。
- 通过链接安装插件（仅支持文件夹），以及项目级技能。
- 尚未在真实账户上试用任何消息频道；它们只在模拟服务上测试。

测试：`cargo test --workspace`（使用 `-- --skip real_embeddinggemma` 可跳过真实嵌入模型的测试）。许可证：PolyForm Noncommercial 1.0.0（见 `LICENSE` 和 `COMMERCIAL-LICENSING.md`）；`third_party/` 保留其自身的许可证。
