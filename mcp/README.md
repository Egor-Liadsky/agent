# mcp

Каталог MCP-серверов проекта. Зарегистрированные серверы — отдельные
репозитории, подключённые подмодулями git по пути `mcp/<имя>`, со своими cargo workspace:
общего workspace над серверами нет, команды выполняются из каталога
конкретного сервера. Кода самого каталога нет — только этот файл.

| Каталог   | Репозиторий | Что это |
|-----------|-------------|---------|
| `mcp/git` | [Egor-Liadsky/git-mcp-agent](https://github.com/Egor-Liadsky/git-mcp-agent) | MCP-сервер git-инструментов, бинарник `git-mcp` |
| `mcp/activity` | [Egor-Liadsky/activity-mcp-agent](https://github.com/Egor-Liadsky/activity-mcp-agent) | демон активности git-проектов со сводками по расписанию, бинарник `activity-mcp` |
| `mcp/pipeline` | [Egor-Liadsky/pipeline-mcp-agent](https://github.com/Egor-Liadsky/pipeline-mcp-agent) | пайплайн `search → summarize → save_to_file` со сводкой через MCP sampling, бинарник `pipeline-mcp` |
| `mcp/index` | [Egor-Liadsky/index-mcp-agent](https://github.com/Egor-Liadsky/index-mcp-agent) | индекс `.docx` и UTF-8 исходников/документации проекта (чанки, эмбеддинги Ollama, SQLite), `index_clear` удаляет метаданные и чанки только совпадающего проектного корпуса |
| `mcp/crm` | ожидает согласования удалённого репозитория | локальная реализация `crm-mcp`, читающая тикеты и пользователей из JSON мини-CRM |

Клиенты связаны с серверами только протоколом MCP — через процесс и
stdin/stdout (`git-mcp`, `pipeline-mcp`, `index-mcp`) или через Streamable HTTP к постоянно работающему
демону (`activity-mcp`): в `Cargo.toml` серверов нет зависимостей на крейты
`agent-cli` (`agentcore`, `agentclient`, `agentupstream`, `agentcli`), а
клиент не зависит от серверов cargo-зависимостью.

Новый сервер подключается так:

```bash
git submodule add https://github.com/Egor-Liadsky/<репозиторий>.git mcp/<имя>
```

`mcp/crm` пока является локальным каталогом реализации без `origin` и не
зарегистрирован как подмодуль. После выбора URL собственного репозитория
зарегистрировать его командой выше по пути `mcp/crm`, сохранив текущие файлы.

и добавляется строкой в таблицу выше и в таблицы подмодулей `README.md` и
`CLAUDE.md` в корне.
