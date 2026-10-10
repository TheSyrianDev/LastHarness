# ERPNext (frappe/erpnext) — Feature Dossier · THIN-SOURCE
Registry: #124 | Category: 08 | Verified: 38,437 stars, GPL-3.0, last push 2026-08-24

## Feature inventory
- **Monolith ERP covering named business areas**: Accounting, Warehouse Management, CRM, Sales, Purchase, HRMS, Project Management, Support, Asset Management, Quality Management, Manufacturing, Website Management, plus a customization layer — "ERPNext as a monolith includes the following areas for managing businesses" [VERIFIED-fetched:https://raw.githubusercontent.com/frappe/erpnext/version-15/README.md]
- **Built on the Frappe Framework**: "a full-stack web app framework built with Python & JavaScript"; ERPNext is an application on top of it [VERIFIED-fetched:https://raw.githubusercontent.com/frappe/erpnext/version-15/README.md]
- **Install paths**: containerized via frappe/frappe_docker (frappe/erpnext-worker image published to Docker Hub); manual install via the bench script that pulls all dependencies including MariaDB; Frappe Cloud hosted signup; Play-with-Docker sandbox [VERIFIED-fetched:https://raw.githubusercontent.com/frappe/erpnext/version-15/README.md]
- **License split**: code GNU GPL-3.0, documentation CC-BY-SA-3.0, copyright Frappe Technologies + contributors; contributions are licensed GPL-3.0 by default [VERIFIED-fetched:https://raw.githubusercontent.com/frappe/erpnext/version-15/README.md]
- **Learning/community infrastructure**: Frappe School course platform, docs.erpnext.com manual, discussion forum, community translation portal [VERIFIED-fetched:https://raw.githubusercontent.com/frappe/erpnext/version-15/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Business-area taxonomy (accounting, warehouse, CRM, sales/purchase, HRMS, projects, support, assets, quality, manufacturing, website) | 🏛 | Dept-map reference: Finance/Accounting + Operations dept candidates | Whole business functions per the verdict rubric. ERPNext's area list is a checklist for which LastHarness departments deserve their own gated memory scope and owner surfaces — a reference map, not an adoption target. |
| ERP-as-app-on-framework shape (monolith app over Frappe) | 🧩 | — | Dept functions as removable apps over one shared record framework is informative for keeping dept schemas uniform; LastHarness's kernel + SQLite already occupies the framework role. |
| frappe_docker containerized deployment (erpnext-worker image) | ⚙ | Ops; runs inside Linux boundary | Containerized self-hosting is boundary-compatible; precedent for how heavyweight human-facing dept tools would be packaged if ever landed. |
| bench installer + MariaDB dependency | ❌ | — | As adoption: drags a second database engine and a standing multi-process application server into the boundary while kernel state stays SQLite-direct. Tolerable only as an isolated dept-tool-local store, never on kernel paths. |
| Frappe Cloud hosted signup path | ❌ | — | Cloud execution of business workloads; WHERE DOES EXECUTION HAPPEN litmus fails. |
| GPL-3.0 code / CC-BY-SA-3.0 docs licensing | ⚙ | Licensing review before any integration | Strong copyleft: internal self-hosted use is unaffected, but redistribution of modified service code triggers share-alike obligations; relevant if LastHarness-derived tooling ever ships outward. |

## Notable absences
The primary source documents zero mechanism: no auth/roles/permissions model, no API surface, no data-model description, no backup/restore guidance, and no AI or agent features anywhere in the README. Everything beyond the business-area list requires leaving the primary source for docs.erpnext.com. Extraction was kept minimal accordingly rather than padded from secondary knowledge.
