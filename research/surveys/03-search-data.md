# 03 Â· Search & Data Extraction

Web search APIs, metasearch, scraping, crawling, document extraction. Feeds LastHarness Research department tooling (Â§14.3) and the Â§16.4 self-hosted-search decision (SearXNG chosen for V1; SaaS options off by default).

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #59 | T-search | Round-based agentic retrieval harness for the T-Search ranking model | Res,Info | ok | high | - | https://github.com/turbo-llm/t-search-harness |
| #74 | Firecrawl | Web scraping/crawling API turning sites into LLM-ready markdown | Res,Product | ok | high | https://firecrawl.dev | https://github.com/firecrawl/firecrawl |
| #90 | SearXNG | Self-hosted privacy metasearch engine aggregating multiple search engines | Res,Sec | ok | high | https://searxng.org | https://github.com/searxng/searxng |
| #94 | Exa | Neural web-search API for AI agents; repo is its MCP server | Dev,Res | ok | high | https://exa.ai | https://github.com/exa-labs/exa-mcp-server |
| #95 | Tavily | LLM-optimized web search API for agents/RAG pipelines | Dev,Res | ok | high | https://tavily.com | https://github.com/tavily-ai/tavily-python |
| #100 | Scrapling | Adaptive anti-bot Python web-scraping library | Dev,Auto | ok | high | - | https://github.com/D4Vinci/Scrapling |
| #104 | AnyDoc | Firecrawl document extractor converting PDFs/docs to clean markdown | Dev,Res | ok | med | https://firecrawl.dev | https://github.com/firecrawl/anydoc |
| #122 | SpiderFoot | OSINT automation platform aggregating 200+ data sources for threat recon | Sec,Res | ok | high | https://spiderfoot.xyz | https://github.com/smicallef/spiderfoot |
| #110 | ScrapeGraphAI | LLM-driven web-scraping pipelines for sites/docs/markdown | Dev,Res | ok | high | https://scrapegraphai.com | https://github.com/ScrapeGraphAI/Scrapegraph-ai |

**Flags & notes**
- **#90 SearXNG** is the adopted V1 choice per docs/archive/lastharness-v1-spec.md Â§16.4 (self-hosted only; Exa/Tavily/cloud Firecrawl excluded â€” SaaS stays an unclosed-but-unadopted option).
- **#104 AnyDoc**: official link resolves to parent product firecrawl.dev (same domain as #74); it is Firecrawl's document-extraction product, distinct entry.
- **#94/#95**: rival but distinct SaaS search products; both excluded from V1 per Â§16.4.
- **Website Downloader** (harvest wave-3): `Ahmadibrahim/Website-downloader` returns 404; closest live wget-based cloner (`Qfas1726/Website-downloader`) is 0★ with no license — not registrable; revisit if a canonical repo surfaces.
