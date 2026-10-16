# agent

Зонтичный репозиторий проекта: сам кода не содержит, а связывает независимые
репозитории на Rust (edition 2024) подмодулями git и хранит общую инструкцию
`CLAUDE.md`. Весь код живёт в подмодулях.

## Подмодули

| Каталог | Репозиторий | Что это |
|---------|-------------|---------|
| `agent-cli` | [Egor-Liadsky/agent-cli](https://github.com/Egor-Liadsky/agent-cli) | cargo workspace: библиотека `agentcore` (`crates/core`) — ядро без терминальных зависимостей — и бинарник `agentcli` (`crates/cli`), консольный клиент и TUI-чат |
| `agent-sever` | [Egor-Liadsky/agent-server](https://github.com/Egor-Liadsky/agent-server) | HTTP-сервис `agentd` (axum, SQLite через sqlx), подключающий `agentcore` git-зависимостью |
| `mcp/git` | [Egor-Liadsky/git-mcp-agent](https://github.com/Egor-Liadsky/git-mcp-agent) | cargo workspace с MCP-сервером git-инструментов `git-mcp` (`crates/git`), который `agentcli` запускает процессом |
| `mcp/activity` | [Egor-Liadsky/activity-mcp-agent](https://github.com/Egor-Liadsky/activity-mcp-agent) | cargo workspace с демоном `activity-mcp` (`crates/activity`): журнал изменений git-проектов и сводки по расписанию; `agentcli` подключается к нему по HTTP |
| `mcp/pipeline` | [Egor-Liadsky/pipeline-mcp-agent](https://github.com/Egor-Liadsky/pipeline-mcp-agent) | cargo workspace с MCP-сервером `pipeline-mcp` (`crates/pipeline`): инструменты `search`, `summarize`, `save_to_file`, связанные в пайплайн |
| `mcp/index` | [Egor-Liadsky/index-mcp-agent](https://github.com/Egor-Liadsky/index-mcp-agent) | cargo workspace с MCP-сервером `index-mcp` (`crates/index`): индекс `.docx` и UTF-8 файлов проекта (чанки, эмбеддинги Ollama, SQLite), инструменты `index_search`, `index_status`, `index_models`, `index_build`, `index_clear`, `index_compare`; `agentcli` запускает его процессом |
| `mcp/crm` | репозиторий ещё не зарегистрирован | локальная реализация `crm-mcp` для чтения демо-CRM; после согласования удалённого репозитория подключить подмодулем |

`mcp/crm` пока обычный локальный каталог без `origin`, а не подмодуль.

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

### `agentcli` и `git-mcp`

Git-инструменты клиенту даёт MCP-сервер `git-mcp` из подмодуля `mcp/git`. Связь
только через процесс и протокол MCP (JSON-RPC через stdin/stdout):
cargo-зависимости между `agent-cli` и `mcp/git` нет ни в одну сторону, сервер
подключается и к другим MCP-клиентам. Клиент ищет бинарник так: путь из
переменной окружения `AGENTCLI_GIT_MCP` → рядом с исполняемым `agentcli` →
`PATH`. Установка сервера:

```bash
cargo install --git https://github.com/Egor-Liadsky/git-mcp-agent git-mcp
# или из подмодуля:
cargo install --path mcp/git/crates/git
```

### `agentcli` и `activity-mcp`

Демон `activity-mcp` из подмодуля `mcp/activity` работает постоянно (launchd
или systemd), следит за каталогом с проектами, пишет журнал изменений в
SQLite и по cron собирает сводки. `agentcli` его не запускает, а подключается
по MCP Streamable HTTP (`http://127.0.0.1:7878/mcp`): показывает сводки в TUI
по `Ctrl+A`, пересказывает их моделью чата и, если включено, даёт модели
читающие инструменты `activity_*`. Cargo-зависимости нет ни в одну сторону.

```bash
cargo install --path mcp/activity/crates/activity
activity-mcp --root ~/projects
cd agent-cli && cargo run -p agentcli -- config activity set on
```

### `agentcli` и `pipeline-mcp`

Сервер `pipeline-mcp` из подмодуля `mcp/pipeline` даёт три инструмента одной
цепочки: `search` находит строки в файлах каталога, `summarize` делает по ним
сводку, `save_to_file` сохраняет её. Выход каждого шага — JSON, поля которого
и есть аргументы следующего. `agentcli` запускает сервер процессом (stdio) и
выполняет цепочку командой `agentcli pipeline run` либо отдаёт инструменты
модели в чате. Своей модели у сервера нет: сводку через MCP sampling пишет
модель клиента.

```bash
cargo install --git https://github.com/Egor-Liadsky/pipeline-mcp-agent pipeline-mcp
# или из подмодуля:
cargo install --path mcp/pipeline/crates/pipeline
cd agent-cli && cargo run -p agentcli -- pipeline run "ToolSet" --root . --output /tmp/out
```

### `agentcli` и `index-mcp`

Сервер `index-mcp` из подмодуля `mcp/index` строит индекс конспектов `.docx`
или UTF-8 исходников и документации проекта, затем ищет по нему по смыслу:
`index_search` возвращает ближайшие чанки,
`index_status` показывает, что лежит в базе, `index_models` — модели Ollama с
эмбеддингами, `index_build` строит индекс, `index_clear` очищает индекс проекта.
`agentcli` запускает
`index-mcp serve` процессом (stdio): командами `agentcli index build|clear|search|
status|models`, разделом «Индекс корпуса» в `Ctrl+P` или отдаёт модели в
обычных чатах `index_search` и `index_status` (`index_build` — после
подтверждения). В проектном чате к `project_*` добавляются четыре индексных
инструмента; обязательный документный поиск отключён, а сборка привязана к
открытому корню проекта; `index_clear` доступен с подтверждением и очищает
только индекс этого проекта. База хранит тип и корень корпуса и не позволяет
смешивать проекты и `.docx`. Поиск не смешивает модели эмбеддингов: другая
модель даёт ошибку.

```bash
cargo install --git https://github.com/Egor-Liadsky/index-mcp-agent index-mcp
# или из подмодуля:
cargo install --path mcp/index/crates/index
cd agent-cli && cargo run -p agentcli -- config index set --root ~/notes --db ~/index.db
cd agent-cli && cargo run -p agentcli -- index build && cargo run -p agentcli -- index search "вопрос"
```

Проектный пример с базой во временном каталоге и небольшим Rust-проектом без
`.docx` (нужен Ollama с `nomic-embed-text`):

```bash
TMP=$(mktemp -d)
cd agent-cli
cargo run -p agentcli -- config index set --source-type project --root ~/projects/sample --db "$TMP/project.db" --strategy fixed
cargo run -p agentcli -- index build --source-type project --min-chars 0
cargo run -p agentcli -- index search "где реализована функция normalize_name" --strategy fixed
```

В проекте индексируются распространённые UTF-8 исходники, Markdown,
конфигурации и скрипты. Пропускаются бинарные/не-UTF-8 файлы, `.env`,
приватные ключи, символьные ссылки, каталоги сборки и файлы больше 1 МиБ;
пропуски показываются как диагностика. Полная пересборка удаляет удалённые
файлы из индекса. AST-анализ и фоновое/инкрементальное обновление не входят в
первую версию.

### Локальная CRM и `agentcli support`

Мини-CRM находится в отдельном проекте `../mini-crm` и хранит тикеты и
вымышленных демо-пользователей в `data/crm.json`. `mcp/crm` запускается как
отдельный MCP-процесс и читает тот же файл. Команда поддержки получает тикет и
его пользователя через `crm_get_ticket`/`crm_get_user`, ищет FAQ через
`index-mcp` (`source_type=project`) и передаёт найденные фрагменты модели.
Использованные файлы и разделы выводятся после ответа. Если релевантных
подтверждений нет, ответ должен прямо сообщить о нехватке данных.

Соберите два MCP-бинарника, создайте тикет в CRM и настройте индекс FAQ:

```bash
cd ../mini-crm && cargo run -- data/crm.json
cd ../ai/mcp/crm && cargo build --release
cd ../ai/mcp/index && cargo build --release
cd ../ai/agent-cli
cargo run -p agentcli -- config index set --source-type project --root ../../mini-crm/faq --db ../../mini-crm/data/faq.db --strategy fixed
cargo run -p agentcli -- index build --source-type project --min-chars 0
AGENTCLI_CRM_FILE=../../mini-crm/data/crm.json \
AGENTCLI_CRM_MCP=../mcp/crm/target/release/crm-mcp \
AGENTCLI_INDEX_MCP=../mcp/index/target/release/index-mcp \
cargo run -p agentcli -- support ask --ticket T-001 "Почему не работает авторизация?"
```

Тот же сценарий доступен из TUI: из каталога `mini-crm` запустите
`cargo run --manifest-path ../ai/agent-cli/Cargo.toml -p agentcli -- chat` с теми же
переменными окружения и настройте FAQ через `Ctrl+P` → «Индекс корпуса»
(тип `project`, каталог `faq`, база `data/faq.db`). Запуск вне Git-проекта
сохраняет корень FAQ при сборке. В чате введите `/support T-001 Почему не
работает авторизация?`. Команда `/support` без аргументов выводит все тикеты
CRM. Там же можно повторно собрать или очистить индекс.

Перед командой проверьте созданный в TUI ID тикета. JSON имеет `version: 1`,
массивы `users` и `tickets`; подробности и демонстрационный FAQ приведены в
README проекта `mini-crm`. Продукт «Люмен Тест» и клиентские записи полностью
вымышлены. Для другого каталога JSON задайте `AGENTCLI_CRM_FILE`; для бинарника
сервера — `AGENTCLI_CRM_MCP`.

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
cd mcp/git     && cargo test && cargo build --release
cd mcp/activity && cargo test && cargo run --release -- --root ~/projects
cd mcp/pipeline && cargo test && cargo build --release
cd mcp/index    && cargo test && cargo build --release
cd agent-cli   && cargo test && AGENTCLI_GIT_MCP=$PWD/../mcp/git/target/release/git-mcp cargo run -p agentcli -- chat
cd agent-sever && cargo test && AGENTD_UPSTREAM_API_KEY=sk-... PORT=8080 cargo run --release
```

Подробности — в README каждого подмодуля.
