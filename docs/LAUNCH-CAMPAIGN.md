# acp-bridge v0.5.0 Launch Campaign

```
  目標: 最大化曝光 openab + acp-bridge 生態
  核心訊息: "你的 Ollama 不只能聊天，現在能讀你的 code"
  時機: 2026-04-15 首次公開
```

---

## 發佈順序 + 時間

```
  第一波 (Day 1) — 自己的社群先
  ├── 1. openab Discord
  └── 2. acp-bridge GitHub Release Notes

  第二波 (Day 1-2) — 技術社群
  ├── 3. Ollama Discord (#showcase)
  ├── 4. Ollama GitHub Discussions
  ├── 5. r/LocalLLaMA (Reddit)
  └── 6. r/rust (Reddit)

  第三波 (Day 2-3) — 社群媒體
  ├── 7. X (Twitter)
  ├── 8. LinkedIn
  ├── 9. Facebook (台灣 AI 社團)
  └── 10. Threads

  第四波 (Day 3-5) — 長文 + EDM
  ├── 11. Dev.to 教學文
  ├── 12. Medium (EN)
  └── 13. EDM (如果有 mailing list)
```

---

## 1. openab Discord (zh-TW)

```
頻道: #general 或 #showcase
```

```
acp-bridge v0.5.0 — 讓你的 Ollama 變成 AI coding agent

從 openab 社群中發現地端 AI 的缺口
做了 acp-bridge 填補 Ollama → agent protocol 的最後一哩路

在 Discord 問「分析這個專案」
它真的會去讀你的 code 回答

v0.1.0 | 能動了
v0.2.0 | 能上 production 了
v0.2.1 | 安全了
v0.3.0 | 穩了
v0.4.0 | Ollama 原生了
v0.5.0 | 能讀你的 code 了 ← 最新

5MB Rust binary / 43 tests / 零 API 費用 / MIT
全部地端跑，資料不出內網

感謝 openab 社群前輩們的架構和指導

GitHub: https://github.com/BlakeHung/acp-bridge
搭配 openab: https://github.com/openabdev/openab
```

---

## 2. GitHub Release Notes

```
  已自動產生 (v0.5.0 tag workflow)
  確認: https://github.com/BlakeHung/acp-bridge/releases/tag/v0.5.0
```

---

## 3. Ollama Discord (EN)

```
頻道: #showcase 或 #community-projects
```

```
acp-bridge v0.5.0 — Turn your Ollama into an AI coding agent

Built a 5MB Rust binary that connects Ollama to AI coding agents.
Now with built-in tools: your LLM can actually read your code.

- Native Ollama API (/api/chat, /api/show, /api/ps)
- Built-in tools: read_file, list_dir, search_code (sandboxed)
- Pairs with openab for team access via Discord
- Zero API cost, data never leaves your machine

Ask "analyze this project" → it reads your source code and answers.

GitHub: https://github.com/BlakeHung/acp-bridge
Discord bridge: https://github.com/openabdev/openab
```

---

## 4. Ollama GitHub Discussions (EN)

```
去哪: https://github.com/ollama/ollama/discussions
Category: Show and tell
```

```
Title: acp-bridge: Native Ollama integration for AI coding agents — now with file access

Hi Ollama community,

I built acp-bridge — a single Rust binary (~5MB) that turns your
Ollama into a backend for AI coding agents.

## What's new in v0.5.0

The LLM can now interact with your local filesystem through
sandboxed tools:
- read_file — read source code (max 1MB)
- list_dir — browse project structure
- search_code — grep for patterns

Ask "analyze this project" and it actually reads your code.

## Ollama-native

acp-bridge uses Ollama's native API directly:
- /api/chat with NDJSON streaming
- /api/show to query model context length
- /api/ps to check VRAM status
- Auto-detects Ollama vs OpenAI-compatible backends

## How it works

Discord user → openab → acp-bridge → Ollama (your GPU)
                                    ↕
                              read_file / list_dir / search_code

Your team talks to your local Ollama through Discord.
Zero API keys. Zero cloud. Data never leaves your machine.

## Quick start

cargo install acp-bridge
LLM_BASE_URL=http://localhost:11434 acp-bridge

## Links
- GitHub: https://github.com/BlakeHung/acp-bridge
- Discord bridge: https://github.com/openabdev/openab
- License: MIT

Would love feedback!
```

---

## 5. r/LocalLLaMA (EN)

```
去哪: https://www.reddit.com/r/LocalLLaMA/submit
Flair: Resource
```

```
Title: acp-bridge v0.5.0: 5MB Rust binary that turns Ollama into
an AI coding agent — now with file access

Body:

My local Ollama can now read my source code and give real answers.

Built acp-bridge, a lightweight bridge between Ollama and the Agent
Client Protocol. Combined with openab (Discord-to-ACP bridge), my
whole team can use our local GPU through Discord.

What it does:
- Ollama native API (not just OpenAI compat)
- Built-in tools: read_file, list_dir, search_code
- All sandboxed to working directory
- 5MB binary, no runtime deps, zero API cost

Tested with gemma4:26b. Ask "analyze this project" → it calls
list_dir, reads files, and gives a real analysis.

14 days from first commit to v0.5.0. 43 tests. MIT license.

GitHub: https://github.com/BlakeHung/acp-bridge
Discord bridge: https://github.com/openabdev/openab
```

---

## 6. r/rust (EN)

```
去哪: https://www.reddit.com/r/rust/submit
Flair: Project
```

```
Title: acp-bridge: Rust CLI that bridges Ollama to AI agent protocols
with built-in sandboxed tools

Body:

Sharing a project I've been building in Rust — acp-bridge connects
local LLMs (Ollama, vLLM, etc.) to the Agent Client Protocol (ACP)
used by coding assistants.

Tech highlights:
- Tokio async runtime, reqwest with rustls (no OpenSSL)
- Dual stream parser: Ollama NDJSON + OpenAI SSE
- Path sandboxing via canonicalize() for built-in tools
- ~2000 lines, 43 tests, builds to ~5MB
- Zero unsafe code

The interesting part: LLM tool calling loop — non-streaming chat()
detects tool_calls, executes locally (read_file, list_dir,
search_code), feeds results back, up to 5 rounds.

GitHub: https://github.com/BlakeHung/acp-bridge
```

---

## 7. X / Twitter (EN + zh-TW)

```
EN version:

acp-bridge v0.5.0 — your Ollama can now read your code.

5MB Rust binary. Zero API cost. Data stays local.

Ask "analyze this project" → it reads your source code and answers.

Built-in tools: read_file, list_dir, search_code (sandboxed)
Native Ollama API support.

Pairs with @openaborgs for team access via Discord.

https://github.com/BlakeHung/acp-bridge

#Ollama #LocalAI #Rust #OpenSource #ACP
```

```
zh-TW version:

acp-bridge v0.5.0 — 你的 Ollama 現在能讀你的 code 了

在 Discord 問「分析這個專案」
→ LLM 真的會去讀你的原始碼回答

5MB Rust binary / 零 API 費用 / 資料不出內網
搭配 openab 讓團隊在 Discord 用地端 AI

https://github.com/BlakeHung/acp-bridge

#Ollama #LocalAI #Rust #開源 #主權AI #台灣
```

---

## 8. LinkedIn (EN)

```
I'm excited to share acp-bridge v0.5.0 — an open-source tool that
turns your local Ollama into an AI coding agent backend.

The problem: Cloud AI APIs are expensive, and sensitive code can't
leave your network. But local LLMs couldn't interact with your
codebase — they could only chat.

The solution: acp-bridge is a 5MB Rust binary that bridges Ollama
to the Agent Client Protocol. Now with built-in tools, the LLM can:
- Read your source files
- Browse directory structures
- Search code patterns
All sandboxed to the working directory.

Combined with openab (open-source Discord bridge), your entire team
can use your local GPU through Discord. Zero API cost. Data never
leaves your machine.

This matters for enterprises in finance, healthcare, and government
where data sovereignty is non-negotiable.

Built with Rust. 43 tests. MIT license. 14 days from first commit.

GitHub: https://github.com/BlakeHung/acp-bridge
Discord bridge: https://github.com/openabdev/openab

#OpenSource #AI #LocalAI #Ollama #Rust #DataSovereignty
#EnterpriseTech #CodingAgent
```

---

## 9. Facebook 台灣 AI 社團 (zh-TW)

```
社團: 台灣人工智慧社團 / AI Taiwan / Rust Taiwan
```

```
[開源] acp-bridge v0.5.0 — 讓 Ollama 變成 AI coding agent

做了一個 Rust 工具，讓你的地端 Ollama 不只能聊天
現在還能讀你的專案原始碼

在 Discord 問「分析這個專案」
→ LLM 呼叫 list_dir 看目錄
→ 呼叫 read_file 讀檔案
→ 根據真實 code 回答

特色：
- Ollama 原生 API（不是套 OpenAI compat）
- 內建工具：read_file / list_dir / search_code
- 安全沙箱：只能讀 working_dir 底下
- 搭配 openab 讓團隊在 Discord 用地端 AI
- 5MB Rust binary / 零 API 費用 / 資料不出內網

適合在意資料主權的場景：金融、醫療、政府
台灣正在推主權 AI，這個工具剛好對上

GitHub: https://github.com/BlakeHung/acp-bridge
openab: https://github.com/openabdev/openab
```

---

## 10. Threads (zh-TW)

```
acp-bridge v0.5.0

你的 Ollama 現在能讀你的 code 了
問「分析這個專案」→ 它真的讀原始碼回答

5MB Rust binary
零 API 費用
資料不出內網

https://github.com/BlakeHung/acp-bridge
```

---

## 11. Dev.to 教學文 (EN)

```
去哪: https://dev.to/new
```

```
Title: How I turned my local Ollama into an AI coding agent
that reads my source code

Tags: ollama, rust, ai, opensource

Body:

## The problem

I wanted my team to use a local LLM through Discord. Cloud APIs
were too expensive and our code couldn't leave the network.

But local LLMs had a fatal flaw: they could only chat. Ask
"analyze my project" and they'd make stuff up because they
couldn't see your files.

## The solution

I built acp-bridge — a 5MB Rust binary that:
1. Bridges Ollama to the Agent Client Protocol (ACP)
2. Provides built-in tools so the LLM can read your code
3. Sandboxes everything to the working directory

## How it works

[architecture diagram from the main post]

## Built-in tools

| Tool | What it does | Limits |
|------|-------------|--------|
| read_file | Read file contents | Max 1MB |
| list_dir | List directory tree | Max depth 3 |
| search_code | Grep for patterns | Max 50 matches |

## Quick start

[quick start code]

## The journey

14 days. 6 versions. From "it connects" to "it reads your code."

[version history from the main post]

GitHub: https://github.com/BlakeHung/acp-bridge
Discord bridge: https://github.com/openabdev/openab
```

---

## 12. Medium (EN)

```
同 Dev.to 內容，調整格式
```

---

## 13. EDM

```
如果你有 mailing list:

Subject: acp-bridge v0.5.0 — Your Ollama can now read your code

Body: 簡短版，連結到 GitHub Release
```

---

## 核心素材 checklist

```
  [ ] 30 秒 demo GIF (你需要自己錄)
      場景: Discord @bot "分析專案結構"
      → emoji 狀態變化
      → tool call 執行
      → 回應出現
      工具: 螢幕錄影 → gifski 轉 GIF

  [x] 更新紀錄 (簡短版) — 已產出
  [x] 各平台宣傳文 — 已產出
  [ ] GitHub repo description 更新
      建議改成:
      "Turn your Ollama into an AI coding agent —
       native API, built-in tools, zero API cost"
```

---

## 追蹤指標

```
  發佈後 7 天追蹤:
  [ ] GitHub stars 數量
  [ ] crates.io 下載數
  [ ] Reddit upvotes / comments
  [ ] Ollama Discussion 回應
  [ ] Discord 新成員
  [ ] Twitter impressions
```
