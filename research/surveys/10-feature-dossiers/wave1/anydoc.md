# anydoc — Feature Dossier
Registry: #104 | Category: 03 | Verified: 17,920 stars, MIT, last push 2026-08-20, repo created 2026-08-03 (GitHub API 2026-08-23)

Rust document→Markdown extractor from Firecrawl: Word/PowerPoint/Excel/OpenDocument/RTF/EPUB/CSV/PDF into GitHub-Flavored Markdown via one shared document model. Pure local computation — no ML models, no external services — so execution never leaves LastHarness's boundary.

## Feature inventory
- 14-format conversion to GFM Markdown: `.doc/.docx/.docm`, `.ppt/.pps/.pot/.pptx/.pptm/.ppsx/.ppsm`, `.xls/.xlsx/.xlsm/.xlsb`, `.odt/.ods/.odp`, `.rtf`, `.epub`, `.csv`, `.pdf` [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- One output for every input: all formats parse into a single document model rendered by one GFM serializer, so escaping/tables/headings/footnotes behave identically and fixes propagate across formats [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Full structure preservation: heading anchors, inline code/blocks, links + internal cross-references, nested/task lists with source numbering, merged-cell tables, block quotes, foot/endnotes, speaker notes [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Equations → LaTeX `$...$`/`$$` from Word/PPT OMML, OpenDocument/EPUB MathML, RTF [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Embedded assets kept on the document model with media types; images render as alt text; external-URL images become ordinary Markdown images [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Content-based format detection from bytes (PDF header, RTF open group, OLE stream names, ZIP mimetype) — mislabeled files still convert; CSV needs explicit naming (no signature) [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Local text-layer PDF support via firecrawl/pdf-inspector, no OCR service; scanned/image-only PDFs are NOT convertible locally (hosted Firecrawl Parse adds OCR — cloud) [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Typed error taxonomy returned only when no meaningful Markdown is possible: `Unsupported`, `Malformed`, `Encrypted`, `ResourceLimit` (decompression/nesting/node-count caps), `MissingPart`, `Io`; per-language error surfaces in Node/Python/WASM [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Performance claims benchmarked openly: median 4.4 ms/doc, only entrant covering 14/14 formats, top score on every judged format vs libreoffice/markitdown/pandoc/docling/mammoth/unstructured; methodology published (blind LLM-judge, position-bias-cancelled) [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Bindings that respect hosts: Node conversions run on the libuv thread pool (event loop never blocked), Python releases the GIL; TS types + stubs shipped [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Ships as an Agent Skill (`npx skills add firecrawl/anydoc`) teaching agents the CLI; CLI reads stdin and writes stdout/files; prebuilt platform binaries via npx [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- WASM build converts files locally in-browser ("files never leave your machine") — same library compiled for sandboxed environments [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]
- Robustness engineering: committed fixture corpus snapshot tests, mutation tests per fixture, cargo-fuzz targets per format [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/anydoc/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Library adoption as the document-ingestion converter | ⚙ Feature adoption | Information dept ingestion worker | Owner drops any office doc; worker converts to Markdown for memory/RAG — pure-local MIT Rust fits every invariant (execution: local CPU, milliseconds) |
| Unified document model → single GFM serializer | 🧩 Mechanism only | Ingestion pipeline § | Normalize-before-serialize choke point: one place to fix Markdown quirks for ALL ingest paths |
| Byte-level format detection | 🧩 Mechanism only | Upload handling § | Trust content over extensions — security-relevant when accepting user-provided files |
| Typed error taxonomy incl. `ResourceLimit` caps | ⚙ Feature adoption | Worker failure recording / decision log | Deterministic failure classes (Encrypted/Malformed/Limit-hit) get logged instead of crashing workers; decompression-bomb caps protect the host |
| Local text-layer PDF extraction | ⚙ Feature adoption | Same ingestion worker | Covers most real PDFs with zero network; pairs with vision pack if a page must be read as image |
| Agent Skill distribution form | 🧩 Mechanism only | Skills system § | Confirms skills-as-installable-docs pattern LastHarness already uses for agent capability packs |
| Hosted OCR upgrade path (Firecrawl Parse) | ❌ Conflict-skip | Fast-follow note only | Cloud execution violates egress/local-first invariants; if scanned-PDF need emerges, wire LOCAL OCR instead |

## Notable absences
No OCR at all locally — scanned PDFs return `Unsupported`; anything image-based is out of scope. No HTML/web-page conversion (that's fetcher/crawler territory). No chunking, embedding, or RAG-aware splitting — output is whole-document Markdown only. No layout/reading-order guarantees documented for complex multi-column PDFs beyond text layer. Windows/macOS/Linux binaries exist but LastHarness cares about Linux only; no streaming API for huge files (bytes-in-memory model).
