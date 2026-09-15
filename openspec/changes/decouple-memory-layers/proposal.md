## Why

`memory_layers` сегодня — одно из шести взаимоисключающих значений
`ContextStrategy`: включить слоистую память можно только ценой отказа от
`summary`/`sliding_window`/`facts`/`branching` для краткосрочной истории.
Это смешивает два разных решения в одно: то, как собирается краткосрочная
история диалога (Context management), и то, есть ли у агента рабочая и
долговременная память поверх этой истории (Memory management). Пользователь
не может, например, держать пересказ длинной истории (`summary`) и при этом
получать долговременную память между чатами — приходится выбирать одно из
двух. Слоистая память должна быть независимым переключателем, применимым
поверх любой стратегии контекста.

## What Changes

- **BREAKING**: значение `ContextStrategy::MemoryLayers` удаляется из
  перечисления (`agentcore::config::ContextStrategy`, `agent-cli`).
  `context_strategy: "memory_layers"` в запросах сервиса становится
  неизвестным значением стратегии и отклоняется так же, как любое другое
  незнакомое имя.
- Слоистая память включается новым независимым полем: `memory_layers_enabled:
  Option<bool>` в `ChatSettings` (`agent-cli`) и операторская переменная
  `AGENTD_MEMORY_LAYERS_ENABLED` (умолчание — выключено) в `agent-sever`.
  Существующие поля `memory_router_enabled`, `memory_working_max_entries`,
  `memory_long_term_max_entries` сохраняются и относятся к включённой
  слоистой памяти при любой стратегии контекста. Поле `memory_short_term_tail`
  (и переменная `AGENTD_MEMORY_SHORT_TERM_TAIL_MESSAGES`) удаляются:
  краткосрочная история больше не собирается слоем памяти отдельным
  размером окна — она уже собрана действующей стратегией контекста, и
  отчёт по этому слою (`memory_short_term_messages`/`memory_short_term_chars`)
  теперь считается по факту от того, что стратегия отправила, а не по
  отдельной настройке.
- `agent-sever/src/context.rs`: сборка истории остаётся работой действующей
  стратегии (`summary`/`sliding_window`/`facts`/`branching`) без изменений
  её поведения; если слоистая память включена, разделы долговременной и
  рабочей памяти добавляются в системное сообщение поверх разделов
  стратегии, а фоновый маршрутизатор запускается после записи обмена
  независимо от того, какая стратегия действует. `MemoryLayersImpl`
  (отдельная реализация `ContextStrategyImpl`, дублировавшая логику хвоста
  `sliding_window`) удаляется — краткосрочная история теперь всегда
  собирается стратегией, а не отдельной копией той же логики.
- Блок `context` ответа `POST /v1/chat` несёт поля стратегии и поля памяти
  одновременно (а не одно вместо другого): например, при `summary` +
  включённой памяти в ответе одновременно `replaced_messages`/`summary_built`
  и `memory_long_term_entries`/`memory_working_entries`/…
- `agent-cli/crates/cli/src/tui.rs`: раздел «Контекст» параметров чата
  (`Ctrl+P`) получает отдельный переключатель «Слоистая память» вне перебора
  стратегий; параметры памяти (`memory_router_enabled` и остальные) видны
  всегда, когда слоистая память включена, а не только при выборе
  определённой стратегии. Экран памяти (`Ctrl+K`) не меняется.
- Ручное управление памятью через HTTP-эндпоинты и хранилище (`chat_working_memory`,
  `owner_long_term_memory`, `chats.active_task_id`) не меняются: они уже не
  зависели от стратегии.

## Capabilities

### Modified Capabilities
- `memory-layers`: слоистая память перестаёт быть значением
  `ContextStrategy` и становится независимым переключателем, применимым
  поверх любой стратегии контекста; счётчики блока `context` для памяти
  присутствуют при включённой памяти независимо от действующей стратегии, а
  не только при стратегии `memory_layers`.

## Impact

### agent-cli

- `crates/core/src/config.rs`: убрать `ContextStrategy::MemoryLayers`
  (`ALL`, `label`, `as_str`, `parse`); добавить `ChatSettings.memory_layers_enabled:
  Option<bool>`; убрать `ChatSettings.memory_short_term_tail`.
- `crates/cli/src/tui.rs`: убрать «Слои памяти» из перебора
  `cycle_context_strategy`; добавить поле-переключатель «Слоистая память» в
  разделе «Контекст»; показывать поля `memory_router_enabled`/`memory_short_term_tail`/
  `memory_working_max_entries`/`memory_long_term_max_entries` при включённой
  памяти, а не при выбранной стратегии.
- `README.md`: описание стратегий контекста без `memory_layers` как
  стратегии; отдельный подраздел про переключатель слоистой памяти.
- Пуш в `main` требуется: `agent-sever` использует изменённое перечисление
  `ContextStrategy` и новое поле `ChatSettings`.

### agent-sever

- `src/context.rs`: удалить `MemoryLayersImpl` и его ветку в `impl_for`;
  `assemble` добавляет разделы памяти поверх разделов действующей стратегии,
  когда память включена.
- `src/memory.rs`: убрать сборку собственного хвоста краткосрочной истории
  (`assemble`, дублировавший `summary::tail_boundary`) и связанный
  `effective_short_term_tail`; оставить чтение слоёв, сборку разделов,
  маршрутизатор и остальные `effective_*` без изменений в их внутренней
  логике, но без завязки на `ContextStrategy::MemoryLayers`. Счётчики
  краткосрочного слоя блока `context` считаются по истории, которую
  фактически собрала действующая стратегия.
- `src/app.rs`: запуск фонового маршрутизатора (`route_after_exchange`)
  проверяет `memory_layers_enabled`, а не `strategy == ContextStrategy::MemoryLayers`.
- `src/dto.rs`: `ContextDto` перестаёт иметь отдельный вариант
  `for_memory_layers`, вместо этого поля памяти добавляются к DTO любой
  стратегии, когда память включена.
- `src/config.rs`: новая переменная `AGENTD_MEMORY_LAYERS_ENABLED` (умолчание
  `false`); убрать `AGENTD_MEMORY_SHORT_TERM_TAIL_MESSAGES`.
- `src/tests.rs`, `src/context.rs`, `src/memory.rs`: тесты на комбинацию
  памяти с каждой из четырёх стратегий контекста.
- `README.md`: раздел про слоистую память переписывается как независимая от
  стратегий контекста возможность.
- Схема БД не меняется: `chat_working_memory`, `owner_long_term_memory`,
  `chats.active_task_id` уже не зависели от стратегии.
- После пуша `agent-cli` в `main`: временно убрать `[patch]` в
  `.cargo/config.toml`, выполнить `cargo generate-lockfile`, проверить
  `grep -A2 'name = "agentcore"' Cargo.lock` (коммит, не путь), прогнать
  `cargo test --locked`, вернуть `[patch]` для локальной разработки.
