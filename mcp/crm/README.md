# crm-mcp

MCP-сервер только для чтения локальной мини-CRM поддержки. У него нет Cargo-
зависимостей на `agent-cli` и `agentcore`; JSON-RPC работает через stdio.

```bash
cargo build --release
cargo run --release -- --data ../../../mini-crm/data/crm.json
```

Передайте один и тот же JSON-файл, который использует TUI. Сервер перечитывает
его при каждом вызове, поэтому сохранённые изменения видны без перезапуска.
Схема версии 1 содержит массивы `users` и `tickets`; формат и вымышленные
демо-записи описаны в `../../../mini-crm/README.md`.

Инструменты возвращают JSON в `structuredContent`:

- `crm_list_tickets` возвращает `{ "tickets": [...] }` со всеми тикетами CRM.
- `crm_get_ticket` возвращает найденный тикет с полями `id`, `user_id`,
  `subject`, `description`, `status` и `created_at`.
- `crm_get_user` возвращает найденного пользователя с полями `id`, `name` и
  `support_notes`.

Для неизвестного ID, недоступного файла или некорректного JSON сервер
возвращает явную ошибку; пишущих инструментов нет. Тест протокола запускает
настоящий процесс `crm-mcp` и проверяет, что следующие вызовы видят обновлённый
файл.

Для полного сценария общий JSON лежит по пути
`../../../mini-crm/data/crm.json` относительно этого каталога. Соберите индекс
FAQ сервером `index-mcp` с `source_type=project`, корнем
`../../../mini-crm/faq` и базой, например `../../../mini-crm/data/faq.db`.
После создания тикета в TUI мини-CRM запустите из `agent-cli`:

```bash
AGENTCLI_CRM_FILE=../../mini-crm/data/crm.json \
AGENTCLI_CRM_MCP=../mcp/crm/target/release/crm-mcp \
AGENTCLI_INDEX_MCP=../mcp/index/target/release/index-mcp \
cargo run -p agentcli -- support ask --ticket T-001 "Почему не работает авторизация?"
```

Ответ содержит обычный текст, затем список файлов, разделов и ID чанков.
FAQ, продукт «Люмен Тест» и примеры пользователей вымышлены.
