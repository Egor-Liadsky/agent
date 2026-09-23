# mcp

Каталог MCP-серверов проекта. Каждый сервер — отдельный репозиторий,
подключённый подмодулем git по пути `mcp/<имя>`, со своим cargo workspace:
общего workspace над серверами нет, команды выполняются из каталога
конкретного сервера. Кода самого каталога нет — только этот файл.

| Каталог   | Репозиторий | Что это |
|-----------|-------------|---------|
| `mcp/git` | [Egor-Liadsky/git-mcp-agent](https://github.com/Egor-Liadsky/git-mcp-agent) | MCP-сервер git-инструментов, бинарник `git-mcp` |

Клиенты связаны с серверами только процессом и протоколом MCP (JSON-RPC
через stdin/stdout): в `Cargo.toml` серверов нет зависимостей на крейты
`agent-cli` (`agentcore`, `agentclient`, `agentupstream`, `agentcli`), а
клиент не зависит от серверов cargo-зависимостью.

Новый сервер подключается так:

```bash
git submodule add https://github.com/Egor-Liadsky/<репозиторий>.git mcp/<имя>
```

и добавляется строкой в таблицу выше и в таблицы подмодулей `README.md` и
`CLAUDE.md` в корне.
