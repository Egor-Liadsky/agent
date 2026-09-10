# agent

Зонтичный репозиторий проекта: сам кода не содержит, а связывает два
независимых репозитория на Rust (edition 2024) подмодулями git и хранит общие
для них артефакты — инструкцию `CLAUDE.md` и общий экземпляр OpenSpec
(`openspec/`, `.claude/`) для изменений, затрагивающих оба репозитория сразу.

## Подмодули

| Каталог | Репозиторий | Что это |
|---------|-------------|---------|
| `agent-cli` | [Egor-Liadsky/agent-cli](https://github.com/Egor-Liadsky/agent-cli) | cargo workspace: библиотека `agentcore` (`crates/core`) — ядро без терминальных зависимостей — и бинарник `agentcli` (`crates/cli`), консольный клиент и TUI-чат |
| `agent-sever` | [Egor-Liadsky/agent-server](https://github.com/Egor-Liadsky/agent-server) | HTTP-сервис `agentd` (axum, SQLite через sqlx), подключающий `agentcore` git-зависимостью |

Имя каталога `agent-sever` содержит опечатку («sever» вместо «server»), но
именно так называется путь подмодуля — не «исправлять».

## Связь репозиториев

`agent-sever/Cargo.toml` тянет `agentcore` из ветки `main` репозитория
`agent-cli`, а не из подмодуля: для сервиса ядро — обычная внешняя зависимость.
Локальный `agent-sever/.cargo/config.toml` (он в `.gitignore` дочернего
репозитория) секцией `[patch]` подменяет git-зависимость на
`../agent-cli/crates/core`, поэтому локальная сборка идёт против рабочей копии
ядра, а образ и CI — против запушенного `main`.

Отсюда порядок изменения ядра, если правка нужна и сервису:

1. Изменить `agent-cli/crates/core`, прогнать там `cargo test`.
2. Запушить `agent-cli` в `main`.
3. В `agent-sever` временно убрать `[patch]`, выполнить
   `cargo generate-lockfile` и прогнать `cargo test --locked`.

## Клонирование

```bash
git clone --recurse-submodules git@github.com:Egor-Liadsky/agent.git
```

Для уже склонированной копии:

```bash
git submodule update --init --recursive
```

Подтянуть в подмодулях свежий `main` и зафиксировать новые коммиты в зонтичном
репозитории:

```bash
git submodule update --remote --merge
git commit -am "Update submodules"
```

## Команды

Общего cargo-workspace над подмодулями нет: любую команду выполнять из каталога
конкретного репозитория, `cargo` из корня не работает.

```bash
cd agent-cli   && cargo test && cargo run -p agentcli -- chat
cd agent-sever && cargo test && AGENTD_UPSTREAM_API_KEY=sk-... PORT=8080 cargo run --release
```

Подробности — в README каждого подмодуля.
