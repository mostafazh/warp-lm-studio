<a href="https://www.warp.dev">
    <img width="1024" alt="Warp Agentic Development Environment product preview" src="https://github.com/user-attachments/assets/9976b2da-2edd-4604-a36c-8fd53719c6d4" />
</a>

<p align="center">
  <a href="https://www.warp.dev">Website</a>
  ·
  <a href="https://www.warp.dev/code">Code</a>
  ·
  <a href="https://www.warp.dev/agents">Agents</a>
  ·
  <a href="https://www.warp.dev/terminal">Terminal</a>
  ·
  <a href="https://www.warp.dev/drive">Drive</a>
  ·
  <a href="https://docs.warp.dev">Docs</a>
  ·
  <a href="https://www.warp.dev/blog/how-warp-works">How Warp Works</a>
</p>

> [!NOTE]
> OpenAI is the founding sponsor of the new, open-source Warp repository, and the new agentic management workflows are powered by GPT models.

<h1></h1>

## About

[Warp](https://www.warp.dev) is an agentic development environment, born out of the terminal. Use Warp's built-in coding agent, or bring your own CLI agent (Claude Code, Codex, Gemini CLI, and others).

## Installation

You can [download Warp](https://www.warp.dev/download) and [read our docs](https://docs.warp.dev/) for platform-specific instructions.

## Warp Contributions Overview Dashboard

Explore [build.warp.dev](https://build.warp.dev) to:
- Watch thousands of Oz agents triage issues, write specs, implement changes, and review PRs
- View top contributors and in-flight features
- Track your own issues with GitHub sign-in
- Click into active agent sessions in a web-compiled Warp terminal

## Using a Local Model with LM Studio

Warp includes a built-in client library for [LM Studio](https://lmstudio.ai), which lets you
serve local models (e.g. **Gemma 2**) via an OpenAI-compatible HTTP API and use them as a
backend for AI features.

### Prerequisites

1. Download and install [LM Studio](https://lmstudio.ai).
2. Inside LM Studio, download the model you want to use.
   Recommended Gemma models:
   - `gemma-2-2b-it` (lightweight, fast)
   - `gemma-2-9b-it` (higher quality)
3. Start the local server:
   - Open the **Server** tab in LM Studio.
   - Click **Start Server**.
   - Make sure **"OpenAI Compatible Server"** is enabled (default port: **1234**).

### Configuration

Configure Warp to use LM Studio via environment variables. Set these before launching Warp,
or export them in your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):

```sh
# Base URL of the LM Studio server (default shown; change port if needed)
export WARP_LM_STUDIO_BASE_URL="http://localhost:1234/v1"

# The model identifier to request (must match what is loaded in LM Studio)
export WARP_LM_STUDIO_MODEL="gemma-2-2b-it"

# API key – LM Studio does not require a real key.
# Leave unset (no Authorization header will be sent) or set a placeholder:
# export WARP_LM_STUDIO_API_KEY="lm-studio"
```

| Variable | Default | Description |
|---|---|---|
| `WARP_LM_STUDIO_BASE_URL` | `http://localhost:1234/v1` | Base URL of the LM Studio OpenAI-compatible endpoint |
| `WARP_LM_STUDIO_MODEL` | *(use loaded model)* | Model identifier (e.g. `gemma-2-2b-it`) |
| `WARP_LM_STUDIO_API_KEY` | *(none)* | API key sent in `Authorization: Bearer` header; not required by LM Studio |

### Verifying the connection

You can test your LM Studio server independently with `curl`:

```sh
curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemma-2-2b-it",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": false
  }'
```

### Architecture note

Warp's default inference path routes requests through the Warp backend server
(`app.warp.dev`). LM Studio runs **locally** on your machine, so its requests
go directly from the Warp client to `localhost:1234`, bypassing the backend.
The `crates/ai/src/lm_studio/` module provides the client library, types, and
configuration helpers for this direct path. Wiring it into specific agent
conversation flows is an ongoing effort—see the linked issue for status.

### Troubleshooting

| Symptom | Fix |
|---|---|
| `Cannot reach the LM Studio server` | Make sure LM Studio is open and you clicked **Start Server** in the Server tab |
| Wrong model response | Check that `WARP_LM_STUDIO_MODEL` matches the model name shown in LM Studio |
| Port conflict | Change the port in LM Studio's server settings and update `WARP_LM_STUDIO_BASE_URL` accordingly |

## Licensing

Warp's UI framework (the `warpui_core` and `warpui` crates) are licensed under the [MIT license](LICENSE-MIT).

The rest of the code in this repository is licensed under the [AGPL v3](LICENSE-AGPL).

## Open Source & Contributing

Warp's client codebase is open source and lives in this repository. We welcome community contributions and have designed a lightweight workflow to help new contributors get started. For the full contribution flow, read our [CONTRIBUTING.md](CONTRIBUTING.md) guide.

### Issue to PR

Before filing, [search existing issues](https://github.com/warpdotdev/warp/issues?q=is%3Aissue+is%3Aopen+sort%3Areactions-%2B1-desc) for your bug or feature request. If nothing exists, [file an issue](https://github.com/warpdotdev/warp/issues/new/choose) using our templates. Security vulnerabilities should be reported privately as described in [CONTRIBUTING.md](CONTRIBUTING.md#reporting-security-issues).

Once filed, a Warp maintainer reviews the issue and may apply a readiness label: [`ready-to-spec`](https://github.com/warpdotdev/warp/issues?q=is%3Aissue+is%3Aopen+label%3Aready-to-spec) signals the design is open for contributors to spec out, and [`ready-to-implement`](https://github.com/warpdotdev/warp/issues?q=is%3Aissue+is%3Aopen+label%3Aready-to-implement) signals the design is settled and code PRs are welcome. Anyone can pick up a labeled issue — mention **@oss-maintainers** on an issue if you'd like it considered for a readiness label.

### Building the Repo Locally

To build and run Warp from source:

```bash
./script/bootstrap   # platform-specific setup
./script/run         # build and run Warp
./script/presubmit   # fmt, clippy, and tests
```

See [WARP.md](WARP.md) for the full engineering guide, including coding style, testing, and platform-specific notes.

## Joining the Team

Interested in joining the team? See our [open roles](https://www.warp.dev/careers).

## Support and Questions

1. See our [docs](https://docs.warp.dev/) for a comprehensive guide to Warp's features.
2. Join our [Slack Community](https://go.warp.dev/join-preview) to connect with other users and get help from the Warp team.
3. Try our [Preview build](https://www.warp.dev/download-preview) to test the latest experimental features.
4. Mention **@oss-maintainers** on any issue to escalate to the team — for example, if you encounter problems with the automated agents.

## Code of Conduct

We ask everyone to be respectful and empathetic. Warp follows the [Code of Conduct](CODE_OF_CONDUCT.md). To report violations, email warp-coc at warp.dev.

## Open Source Dependencies

We'd like to call out a few of the [open source dependencies](https://docs.warp.dev/help/licenses) that have helped Warp to get off the ground:

* [Tokio](https://github.com/tokio-rs/tokio)
* [NuShell](https://github.com/nushell/nushell)
* [Fig Completion Specs](https://github.com/withfig/autocomplete)
* [Warp Server Framework](https://github.com/seanmonstar/warp)
* [Alacritty](https://github.com/alacritty/alacritty)
* [Hyper HTTP library](https://github.com/hyperium/hyper)
* [FontKit](https://github.com/servo/font-kit)
* [Core-foundation](https://github.com/servo/core-foundation-rs)
* [Smol](https://github.com/smol-rs/smol)
