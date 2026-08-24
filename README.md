# HollowChat

A private, end-to-end encrypted messenger for gamers. A Discord alternative built around anonymity: minimal server-side data, the Signal Protocol for messages, and an optional Tor mode.

## Repository layout

```
crates/server   — backend (Rust + Axum)
crates/common   — shared types between server and (eventually) client
client/         — desktop client (Tauri 2 + Svelte 5)
migrations/     — PostgreSQL SQL migrations
```

## Development

Backend:

```
cp .env.example .env
cargo run -p hollowchat-server
```

Client:

```
cd client
pnpm install
pnpm tauri dev
```

## License

AGPLv3 — see [LICENSE](LICENSE). A fork offering network access must release its source.

Emoji graphics are [Twemoji](https://github.com/jdecked/twemoji), licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).
