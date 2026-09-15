## 1. Ядро (`agent-cli/crates/core`)

- [x] 1.1 Убрать `ContextStrategy::MemoryLayers` из `ALL`, `label`,
      `as_str`, `parse` (`crates/core/src/config.rs`); обновить/удалить
      тесты, ссылавшиеся на это значение
      (`memory_layers_strategy_round_trips_snake_case_json`,
      `memory_layers_strategy_parses_from_str`); `cargo test -p agentcore`
      проходит.
- [x] 1.2 Убрать `ChatSettings.memory_short_term_tail`; добавить
      `ChatSettings.memory_layers_enabled: Option<bool>` с сериализацией
      `snake_case`; тест на то, что старый JSON без этого поля разбирается
      как `None`, и тест на то, что JSON со старым полем
      `memory_short_term_tail` разбирается без ошибки (лишнее поле
      игнорируется serde).
- [x] 1.3 Исправить прямые конструкторы `ChatSettings { .. }` в
      `crates/client/src/chats/tests.rs` и `crates/cli/src/tui.rs` (замена
      поля `memory_short_term_tail` на `memory_layers_enabled` везде, где
      оно перечислено явно); `cargo build --workspace` без ошибок.
- [x] 1.4 Запушить `agent-cli` в `main`; в `agent-sever` временно убрать
      `[patch]`, выполнить `cargo generate-lockfile`, проверить
      `grep -A2 'name = "agentcore"' Cargo.lock` (коммит, не путь), прогнать
      `cargo test --locked`, вернуть `[patch]` для дальнейшей локальной
      разработки.

## 2. Развязка стратегии и памяти (`agent-sever`)

- [x] 2.1 В `src/memory.rs` заменить `MemoryLayersImpl`/`assemble` (реализацию
      `ContextStrategyImpl`) на свободные функции: `layers(state, chat,
      settings, owner) -> MemoryLayers` (читает долговременную и рабочую
      память с лимитами, строит `sections: Vec<String>` в порядке
      `[long_term, working]`, пропуская пустые) и
      `merge_into_context(&mut ContextDto, &MemoryLayers, history: &[Message])`
      (заполняет поля памяти по факту переданной истории, не трогая
      остальные поля DTO); убрать `effective_short_term_tail` и собственную
      сборку краткосрочного хвоста (`tail_boundary`); тест на то, что
      `merge_into_context` считает `memory_short_term_messages`/`_chars` по
      длине/символам переданной истории, а не по отдельной настройке.
- [x] 2.2 В `src/context.rs` удалить `MemoryLayersImpl` и ветку
      `ContextStrategy::MemoryLayers` в `impl_for`; в `assemble` после
      вызова `impl_for(strategy).assemble(...)` добавить: если
      `crate::memory::effective_layers_enabled(state, settings)` — получить
      `crate::memory::layers(...)`, вставить её разделы ПЕРЕД разделами
      стратегии в `assembled.sections`, вызвать `merge_into_context` и
      подмешать счётчики маршрутизатора из `AppState.memory_route_outcomes`
      (как раньше в `MemoryLayersImpl`); тест: при стратегии `summary` с
      непустым пересказом и включённой памятью системное сообщение
      содержит раздел долговременной памяти, затем рабочей, затем раздел
      пересказа — в этом порядке.
- [x] 2.3 Убрать `ContextDto::for_memory_layers` (`src/dto.rs`); поля памяти
      в `ContextDto` заполняются только через `merge_into_context`; тест:
      `ContextDto`, собранный для стратегии `facts` с включённой памятью,
      одновременно содержит `facts_applied`/`facts_updated` и поля памяти,
      а `strategy` равно `Facts`, не отсутствующему более `MemoryLayers`.
- [x] 2.4 Тест: при выключенной памяти (`memory_layers_enabled: false` или
      не задано при выключенном `AGENTD_MEMORY_LAYERS_ENABLED`) системное
      сообщение не содержит разделов памяти и блок `context` не содержит
      полей памяти ни при одной из четырёх стратегий — параметризовать по
      стратегии, а не проверять одну.

## 3. Маршрутизатор и конфигурация (`agent-sever`)

- [x] 3.1 В `src/app.rs` заменить условие запуска
      `route_after_exchange` (`strategy == ContextStrategy::MemoryLayers`)
      на `crate::memory::effective_layers_enabled(&state, &settings)`; тест:
      маршрутизатор запускается при стратегии `sliding_window` с включённой
      памятью и не запускается при той же стратегии с выключенной.
- [x] 3.2 В `src/config.rs` добавить `AGENTD_MEMORY_LAYERS_ENABLED` (bool,
      умолчание `false`) через `parse_bool_named`; убрать
      `AGENTD_MEMORY_SHORT_TERM_TAIL_MESSAGES` и поле
      `memory_short_term_tail_messages`; тесты валидных/невалидных значений
      новой переменной и тест, что старая переменная больше не читается
      (задать её в окружении — конфигурация не падает и не использует
      значение).
- [x] 3.3 Добавить `crate::memory::effective_layers_enabled(state, settings)
      -> bool` (настройка чата поверх операторского умолчания, по образцу
      `effective_router_enabled`); тест переопределения умолчания
      настройкой чата.
- [x] 3.4 Убедиться, что доступность слоистой памяти не зависит от
      `AGENTD_ALLOWED_CONTEXT_STRATEGIES`: тест — список разрешённых
      стратегий без `memory_layers` (значения, которого больше нет) не
      мешает включить память поверх `sliding_window`, входящего в список.

## 4. Тесты на комбинации стратегий и памяти (`agent-sever`)

- [x] 4.1 Параметризованный (или явно по одному на стратегию) интеграционный
      тест в `src/tests.rs`: для каждой из `summary`, `sliding_window`,
      `facts`, `branching` с включённой памятью — ответ `POST /v1/chat`
      содержит и поля своей стратегии, и поля памяти одновременно.
- [x] 4.2 Тест сценария `facts` + память: факт, сохранённый ранее, и запись
      долговременной памяти, сохранённая маршрутизатором, одновременно
      присутствуют в системном сообщении следующего запроса, каждая в своём
      разделе.
- [x] 4.3 Обновить существующие тесты `add-memory-layers`, писавшие
      `context_strategy: "memory_layers"` (`src/tests.rs`,
      `src/context.rs`, `src/memory.rs`) — переписать на нужную стратегию
      плюс `memory_layers_enabled: true`; `cargo test` зелёный.

## 5. Клиент (`agent-cli/crates/cli`)

- [x] 5.1 В `SettingsEditor`/`FormatField` (`tui.rs`) убрать «Слои памяти»
      из `cycle_context_strategy` (5 состояний вместо 6, как до
      `add-memory-layers`); добавить поле-переключатель `MemoryLayersEnabled`
      («Слоистая память: вкл/выкл/умолчание сервиса», три состояния как у
      `SummaryEnabled`) в разделе «Контекст», видимое всегда, а не при
      определённой стратегии.
- [x] 5.2 Поля `MemoryRouterEnabled`/`MemoryWorkingMaxEntries`/`MemoryLongTermMaxEntries`
      показывать при включённом переключателе памяти (`memory_layers_enabled`),
      а не при `context_strategy == memory_layers`; убрать поле
      `MemoryShortTermTail` (настройки больше нет); тест: поля памяти видны
      при `context_strategy: sliding_window` + включённом переключателе,
      не видны при выключенном независимо от стратегии.
- [ ] 5.3 `cargo test -p agentcli` проходит; вручную запустить `cargo run -p
      agentcli -- chat`, пройти сценарий: выбрать стратегию `sliding_window`,
      включить «Слоистая память», добавить запись рабочей памяти в экране
      `Ctrl+K`, увидеть её в разбивке блока `context` следующего ответа
      вместе со счётчиками `sliding_window`.

## 6. README

- [x] 6.1 Обновить `agent-sever/README.md`: раздел «Стратегии контекста»
      без `memory_layers` как значения `context_strategy`; новый подраздел
      «Слоистая память» описывает независимый переключатель, переменную
      `AGENTD_MEMORY_LAYERS_ENABLED`, отсутствие `AGENTD_MEMORY_SHORT_TERM_TAIL_MESSAGES`.
- [x] 6.2 Обновить `agent-cli/README.md`: раздел «Контекст» параметров чата
      и раздел «Память чата» — переключатель отдельно от перебора
      стратегий, без поля «Хвост краткосрочной памяти».

## 7. Финальная проверка

- [x] 7.1 Итоговый прогон `cargo test` в `agent-cli` и в `agent-sever`, а
      также `cargo test --locked` в `agent-sever` без `[patch]` — все
      проходят.
- [x] 7.2 Убедиться, что `docs/context-strategies-comparison.md` и
      `openspec/changes/add-memory-layers/comparison.md` (уже
      заархивированного изменения) не содержат утверждений, ставших
      неверными после этого изменения (например, «стратегия memory_layers»)
      — при необходимости добавить сноску, что `memory_layers` как
      стратегия удалена этим изменением, без переписывания архивных данных
      прогона.
