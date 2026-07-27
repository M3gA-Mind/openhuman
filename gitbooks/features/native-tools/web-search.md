---
description: >-
  A native search tool the agent can call directly - managed search is powered by
  Exa and needs no API key.
icon: magnifying-glass
---

# Web Search

The agent can search the live web on its own. By default this runs on **OpenHuman Managed** search: the query goes through the OpenHuman backend, currently powered by [Exa](https://exa.ai), so you never carry a search API key. If you would rather search directly from your own machine, you can bring your own key for Exa, Parallel, Brave, or Querit. If you run your own [SearXNG](https://docs.searxng.org/) instance, you can enable `searxng_search` as a private, self-hosted search tool.

## What it's good for

* Research - "what's the latest on X".
* Citation hunting - "find me three sources for Y".
* Fact-checking before answering - the agent runs a quick search if it isn't confident.

## Search engines

Pick the engine under **Settings → Search engine**. Exactly one engine is active at a time, and that engine owns the canonical `web_search_tool` the agent calls.

| Engine | Your own API key | Where your queries go |
| --- | --- | --- |
| **OpenHuman Managed** (default) | Not needed | The OpenHuman backend, currently powered by [Exa](https://exa.ai). |
| **Exa** | Required | Straight to `https://api.exa.ai` with your key. |
| **Parallel** | Required | Straight to the Parallel API with your key. |
| **Brave** | Required | Straight to the Brave Search API with your key. |
| **Querit** | Required | Straight to the Querit API with your key. |
| **Disabled** | Not needed | Nowhere. Search tools are removed from the agent's tool list entirely. |

Selecting a bring-your-own-key engine without saving a key falls back to managed search, so the agent always has working search. Once a search finishes, the chat timeline names the provider that answered it ("Searched with Exa"), so the managed path is never an unattributed black box.

### OpenHuman Managed (default)

Managed search is the out-of-the-box path and needs no setup: it is proxied through the OpenHuman backend on your existing subscription, and Exa is the provider behind it today. Your machine holds no search credentials, and the agent gets the single `web_search_tool` slot.

### Exa (bring your own key)

Prefer to run search on your own Exa account? Grab a key from [exa.ai](https://exa.ai) and paste it under **Settings → Search engine → Exa**. Calls then go straight from your machine to `https://api.exa.ai` with your key and never touch the managed backend. The key is stored encrypted in your OS keyring alongside your other secrets.

Choosing Exa registers Exa's neural-search family for the agent, on top of the usual `web_search_tool`:

* `exa_search` - ranked pages with URLs, titles, publish dates, and optional page text. Supports search modes from instant to deep reasoning, domain include/exclude filters, a published-date range, and result categories.
* `exa_find_similar` - pages semantically similar to a URL you already have, for expanding from one good source to comparable ones (competitors, related papers, similar articles).
* `exa_get_contents` - the full crawled contents of one or more URLs, with an optional summary or query-relevant highlights per URL.

You can also select it from `config.toml`:

```toml
[search]
engine = "exa"

[search.exa]
api_key = "your-exa-api-key"
```

Or via environment:

```bash
OPENHUMAN_SEARCH_ENGINE=exa
EXA_API_KEY=your-exa-api-key
# OPENHUMAN_EXA_API_KEY is accepted as well
```

## Self-hosted SearXNG

SearXNG search is opt-in. When enabled, OpenHuman registers `searxng_search` for agents and MCP clients. The tool calls your configured SearXNG `/search?format=json` endpoint and returns normalized `{ title, url, snippet, source }` results.

Enable it in `config.toml`:

```toml
[searxng]
enabled = true
base_url = "http://localhost:8080"
max_results = 10
default_language = "en"
timeout_seconds = 10
```

Or via environment:

```bash
OPENHUMAN_SEARXNG_ENABLED=true
OPENHUMAN_SEARXNG_BASE_URL=http://localhost:8080
OPENHUMAN_SEARXNG_MAX_RESULTS=10
OPENHUMAN_SEARXNG_DEFAULT_LANGUAGE=en
OPENHUMAN_SEARXNG_TIMEOUT_SECONDS=10
```

Per call, the tool accepts `query`, optional `categories` (`web`, `news`, `images`), optional `language`, and optional `max_results` up to 50. Empty queries, unsupported categories, non-2xx SearXNG responses, and timeout failures return structured tool errors instead of silently falling back to a cloud search provider.

## How it differs from generic HTTP

A pure `http_request` tool can fetch a URL but can't *find* one. Web Search is the discovery layer: it picks the right URLs for the agent, which then hands them off to the [Web Scraper](web-scraper.md) for the actual reading.

## See also

* [MCP Server](../../developing/mcp-server.md) - how `searxng_search` appears to MCP clients.
* [Web Scraper](web-scraper.md) - fetch and clean a specific URL.
* [Smart Token Compression](../token-compression.md) - search snippets are compressed before they hit the model.
