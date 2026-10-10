# 08 · Infrastructure & Data Platforms

Databases, search infrastructure, kanban/PM, and generic agent-workspace infrastructure. Cross-cutting candidates that would sit under LastHarness's own deployment rather than inside any one department (§16.1 stack decisions).

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #15 | Wekan | Self-hosted open-source kanban board for task management | Dev | ok | high | https://wekan.fi | https://github.com/wekan/wekan |
| #16 | Elasticsearch | Distributed search/analytics engine; candidate LastHarness search backend | Info,Product | ok | high | https://www.elastic.co | https://github.com/elastic/elasticsearch |
| #61 | Cloudflare Computer | Durable-Object virtual filesystem with sandboxed execution surfaces for agents | Dev | ok | high | - | https://github.com/cloudflare/computer |
| #124 | ERPNext | Free open-source ERP running full company operations on Frappe framework | Auto,Product | ok | high | https://erpnext.com | https://github.com/frappe/erpnext |
| #126 | Lunar | Headless e-commerce backend package bringing Shopify-style power to Laravel | Dev,Product | ok | high | https://lunarphp.com | https://github.com/lunarphp/lunar |
| #130 | PocketBase | Open-source realtime backend in one file: SQLite DB, auth, files, admin UI | Dev,Kernel | ok | high | https://pocketbase.io | https://github.com/pocketbase/pocketbase |
| #135 | Vaultwarden | Unofficial Bitwarden-compatible self-hosted credentials server written in Rust | Sec,Kernel | ok | high | https://vaultwarden.de | https://github.com/dani-garcia/vaultwarden |

**Flags & notes**
- **#135 Vaultwarden**: AGPL-3.0 (network-copyleft — fine to SELF-HOST, restrictive to embed); natural Security-Vault reference for §14.5 credential custody. "git.radar" in the harvest caption is the poster handle, not the maintainer.
- **#61**: preview-only Cloudflare agent workspace infra (SQLite VFS in Durable Objects + FUSE container backend) — conflicts with §2.1 self-hosted/private by default; reference only.
- **#15/#16**: generic infrastructure; relevant only if a department's authority (§5.2) needs a board or search backend beyond SQLite + the §6.3 in-process vector index.
