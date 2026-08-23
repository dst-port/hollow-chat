# HollowChat

Приватный E2E-зашифрованный мессенджер для геймеров. Альтернатива Discord с упором на анонимность: минимум данных на сервере, Signal Protocol для сообщений, опциональный Tor-режим.

Полный план проекта — [docs/plan.md](docs/plan.md).

## Структура репозитория

```
crates/server   — backend (Rust + Axum)
crates/common   — общие типы между сервером и (в будущем) клиентом
client/         — desktop-клиент (Tauri 2 + Svelte 5)
migrations/     — SQL-миграции PostgreSQL
```

## Разработка

Backend:

```
cp .env.example .env
cargo run -p hollowchat-server
```

Клиент:

```
cd client
pnpm install
pnpm tauri dev
```

## Лицензия

AGPLv3 — см. [LICENSE](LICENSE). Форк с сетевым доступом обязан открыть исходники.
