# CLAUDE.md

Рабочая директория `/Users/egor_lyadskiy/ai` — зонтичный репозиторий
`https://github.com/Egor-Liadsky/agent`. Кода в нём нет: он хранит этот файл и
общий экземпляр OpenSpec, а весь код живёт в независимых репозиториях на Rust
(edition 2024), подключённых подмодулями git: двух, связанных общим ядром, и
MCP-серверах в каталоге `mcp/`, с которыми клиент связан только процессом:

| Каталог        | Что это                                                              |
|----------------|----------------------------------------------------------------------|
| `agent-cli`    | cargo workspace: библиотека `agentcore` (`crates/core`) и бинарник `agentcli` (`crates/cli`) — консольный клиент и TUI-чат |
| `agent-sever`  | HTTP-сервис `agentd` (axum), подключающий `agentcore` git-зависимостью |
| `mcp/git`      | cargo workspace `git-mcp-agent`: MCP-сервер git-инструментов `git-mcp` (`crates/git`, rmcp) |
| `mcp/activity` | cargo workspace `activity-mcp-agent`: демон `activity-mcp` (`crates/activity`, rmcp + axum + sqlx) — журнал изменений git-проектов и сводки по cron |

Имя каталога `agent-sever` содержит опечатку («sever» вместо «server»), но
именно так называется путь подмодуля — не «исправлять» пути; репозиторий на
GitHub при этом называется `agent-server`. Аналогично подмодуль `mcp/git` —
репозиторий `git-mcp-agent`, `mcp/activity` — `activity-mcp-agent`.

`mcp/` — обычный каталог зонтика, а не подмодуль: в нём лежат MCP-серверы,
каждый своим подмодулем `mcp/<имя>` со своим cargo workspace. Новый сервер
подключается `git submodule add <url> mcp/<имя>` и добавляется строкой в
таблицы `mcp/README.md`, `README.md` и этого файла.

Коммитить работу в каждом подмодуле из его каталога и пушить в его собственный
`origin`. Зонтичный репозиторий хранит лишь указатель на коммит подмодуля:
после пуша дочернего репозитория новый указатель фиксируется отдельным
коммитом в корне.

Любую команду выполнять из каталога конкретного репозитория: общего
workspace над ними нет, `cargo` из `/Users/egor_lyadskiy/ai` не работает.

## Ключевая связь двух репозиториев

`agent-sever/Cargo.toml` тянет `agentcore` из ветки `main` репозитория
`https://github.com/Egor-Liadsky/agent-cli`. Локально рядом лежит
`agent-sever/.cargo/config.toml` (он в `.gitignore`) с секцией `[patch]`,
подменяющей git-зависимость на `../agent-cli/crates/core`. Из-за этого
локальная сборка сервиса идёт против рабочей копии ядра, а образ и CI —
против запушенного `main`.

Порядок изменения ядра, если правка нужна и сервису:

1. Изменить `agent-cli/crates/core`, прогнать там `cargo test`.
2. Запушить `agent-cli` в `main`.
3. В `agent-sever` временно убрать `[patch]`, выполнить `cargo generate-lockfile`,
   чтобы `Cargo.lock` зафиксировал новый коммит, и прогнать `cargo test --locked`.

Проверка, что в лок-файле коммит, а не путь:
`grep -A2 'name = "agentcore"' agent-sever/Cargo.lock`.

## Связь `agentcli` ↔ `git-mcp`

Git-инструменты клиенту даёт MCP-сервер `git-mcp` из подмодуля `mcp/git`. Связь
только через процесс: `agentcli` запускает `git-mcp --repository <путь>` и
говорит с ним JSON-RPC через stdin/stdout (`crates/cli/src/mcp.rs`).
Cargo-зависимости между ними нет ни в одну сторону: в `mcp/**/Cargo.toml`
не бывает `agentcore`, `agentclient`, `agentupstream`, `agentcli`, а
`agent-cli` не зависит от `git-mcp`. Имена инструментов и схемы аргументов
— общий контракт: `READ_ONLY_TOOLS` в `mcp.rs` классифицирует инструменты
по имени, поэтому переименование инструмента на сервере требует правки
клиента.

Клиент ищет бинарник так: путь из `AGENTCLI_GIT_MCP` (берётся как есть) →
рядом с исполняемым `agentcli` → `PATH`. Workspace `agent-cli` сервер не
собирает, поэтому для локального запуска из исходников:

```bash
(cd mcp/git && cargo build --release)
cd agent-cli && AGENTCLI_GIT_MCP=$PWD/../mcp/git/target/release/git-mcp cargo run -p agentcli -- chat
```

Путь в `AGENTCLI_GIT_MCP` для `cargo test` — абсолютный: тесты идут из
каталога крейта.

## Связь `agentcli` ↔ `activity-mcp`

`activity-mcp` из `mcp/activity` — не процесс клиента, а постоянный демон
(launchd/systemd): он следит за каталогом с проектами (`notify` + опрос
`git`), пишет события в SQLite и по cron собирает сводки даже при закрытом
клиенте. Поэтому транспорт — MCP Streamable HTTP на loopback
(`http://127.0.0.1:7878/mcp`), а `agentcli` подключается к работающему
демону на одну операцию или один ход (`crates/cli/src/activity.rs`); `--stdio`
у демона — лишь чтение его базы для других клиентов. Сводку пересказывает
модель чата клиента (реплика в чат), у демона нет LLM и ключей.

Настройки клиента — поля `activity_*` в `Config` ядра (`agentcli config
activity …`), не `ChatSettings`: демон один на машину. Модели в чате
отдаются только читающие `activity_digest`, `activity_projects`,
`activity_changes` (`CHAT_TOOLS` в `activity.rs`); вместе с git-инструментами
их объединяет `ToolSet` из `tool_loop.rs`. Имена инструментов и формат JSON
ответов — общий контракт, как и у `git-mcp`.

Живой тест клиента против запущенного демона (из `agent-cli`):
`AGENTCLI_ACTIVITY_URL=http://127.0.0.1:7878/mcp cargo test -p agentcli live_daemon -- --ignored`.

## Команды

`agent-cli`:

```bash
cargo build --release
cargo test
cargo run -p agentcli -- ask "вопрос"    # разовый вопрос
cargo run -p agentcli -- chat            # интерактивный TUI
cargo run -p agentcli -- config show     # конфиг, ключ маскируется
```

`agent-sever`:

```bash
cargo test
AGENTD_UPSTREAM_API_KEY=sk-... PORT=8080 cargo run --release
docker build -t agentd .
```

`mcp/git` (и любой сервер в `mcp/<имя>`):

```bash
cargo build --release
cargo test                                   # тесты протокола запускают собранный git-mcp
cargo clippy --all-targets -- -D warnings
cargo run --release -- --repository <путь>   # сервер на stdio
```

Живой тест клиента против настоящего сервера (из `agent-cli`):
`AGENTCLI_GIT_MCP=$PWD/../mcp/git/target/release/git-mcp cargo test -p agentcli live_server -- --ignored`.

## Устройство `agent-cli`

- `crates/core` (`agentcore`) не зависит ни от терминала, ни от
  пользовательского конфига CLI — этим ядром пользуется и сервис. Все
  терминальные крейты (`clap`, `ratatui`, `crossterm`, `termimad`,
  `ansi-to-tui`, `indicatif`, `console`, `arboard`) живут только в
  `crates/cli`; тянуть их в ядро нельзя.
- `agent/mod.rs` — трейт `Agent` с единственным методом
  `ask(&self, history: &[Message], settings: &ChatSettings) -> Result<AgentReply>`.
  Новый провайдер добавляется реализацией трейта, а не правкой вызывающего кода.
- `agent/http.rs` — облачный провайдер (OpenAI-совместимый
  `POST {base_url}/chat/completions`), `agent/ollama.rs` — локальный Ollama
  через нативный API (`/api/chat`, `/api/tags`), выбранный ради `thinking`,
  счётчиков токенов и `top_k`.
- `agent/error.rs` — типизированная `AgentError`, которая кладётся внутрь
  `anyhow::Error`. Различать причины ошибки через `downcast_ref`, а не по
  тексту сообщения.
- `pipeline.rs` — конвейер с фиксированным порядком стадий: нормализация →
  входные политики → вызов модели → выходные политики → судья. Стадии
  передаются списками, поэтому новое правило регистрируется реализацией
  стадии, а не правкой самого конвейера или транспортного уровня.
- Конфиг (`~/.config/agentcli/config.toml`, на macOS —
  `~/Library/Application Support/agentcli/config.toml`) задаёт лишь
  умолчания для новых чатов. Рабочие параметры (`ChatSettings`: провайдер,
  модель, формат ответа, сэмплирование, стратегия рассуждения) хранятся в
  файле самого чата в `~/.config/agentcli/chats/*.json`.
- `tui.rs` — самый большой файл проекта (~2100 строк). Правки в нём делать
  точечно.

## Устройство `agent-sever`

- Конфигурация только из переменных окружения с префиксом `AGENTD_`;
  конфиг консольного клиента сервис не читает. Ключ провайдера принадлежит
  сервису: тело запроса с полем `api_key` отклоняется явным `400`.
- Отсутствующий `AGENTD_UPSTREAM_API_KEY` — фатальная ошибка при старте, а
  не отказ на первом запросе. Пустой `AGENTD_CLIENT_TOKENS` выключает
  аутентификацию и сопровождается записью `WARN`.
- Эндпоинты: `POST /v1/chat`, `GET /v1/models`, `POST /v1/chats` и
  `GET /v1/chats` (создание и список), `GET /PATCH /DELETE /v1/chats/{id}`,
  `GET /healthz` (живость, провайдера не трогает), `GET /readyz` (валидность
  конфигурации).
- Хранилище — SQLite через `sqlx` (`src/store.rs`), файл БД задаётся
  `AGENTD_DB_PATH` (по умолчанию `agentd.db`), пул — `AGENTD_DB_MAX_CONNECTIONS`
  и `AGENTD_DB_BUSY_TIMEOUT_MS`. Миграции — `migrations/*.sql`, применяются
  автоматически при старте (`sqlx::migrate!()`). Владелец чата в колонке
  `owner` — SHA-256-отпечаток клиентского токена (`owner_fingerprint`), сам
  токен в БД не попадает.
- Ошибки отдаются единым конвертом
  `{ "error": { "code", "message", "request_id" } }`; идентификатор запроса
  дублируется в заголовке `x-request-id`.
- Секреты в журнале только маскированные (`secr***alue`). Тексты промптов и
  ответов пишутся, только если `AGENTD_LOG_CONTENT=true`.
- Тесты лежат в `src/tests.rs` (модуль под `#[cfg(test)]`), провайдер
  подменяется `wiremock`.

## Соглашения

- Общение в чате — исключительно на русском языке, включая пояснения и
  рассуждения; англоязычное окружение (логи, README, вывод инструментов)
  язык ответа не меняет. Дословно на английском остаются код, команды,
  тексты ошибок и сообщения коммитов.
- Документация, doc-комментарии и комментарии в коде — на русском языке;
  сообщения коммитов — на английском, в повелительном наклонении
  («Add Ollama provider and per-chat model selection»). Держаться этого
  разделения.
- Комментарии в этом коде объясняют выбор («почему нативный API Ollama, а не
  OpenAI-совместимый»), а не пересказывают строку. Писать так же.
- README обоих репозиториев подробные и поддерживаются вместе с кодом:
  изменение поведения, команд, переменных окружения или горячих клавиш
  требует правки README в том же изменении.
- Логи обмена с провайдером — JSON Lines (`requests.jsonl`,
  `responses.jsonl`) в директории данных ОС; `AGENTCLI_LOG_DIR` её
  переопределяет. Каталог `logs/` в `.gitignore`.

## OpenSpec

Процесс OpenSpec (`openspec/`, схема `spec-driven`) включён в обоих
репозиториях — и в `agent-cli`, и в `agent-sever`, — с командами и навыками
в `.claude/commands/opsx/` и `.claude/skills/`. Каждый репозиторий ведёт свои
артефакты: изменение заводится там, где лежит код, который оно меняет, а
изменение, затрагивающее оба репозитория, — там, где основная часть работы.
Артефакты изменения лежат в `openspec/changes/<имя>/` (`proposal.md`,
`design.md`, `tasks.md`, `specs/<capability>/spec.md`). Навыки propose и
apply разделены намеренно: propose создаёт только планирующие артефакты и
на этом останавливается, реализация начинается отдельным запросом.
