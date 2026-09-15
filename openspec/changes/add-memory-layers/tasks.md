## 1. Ядро (`agent-cli/crates/core`)

- [x] 1.1 Добавить `ContextStrategy::MemoryLayers` (`ALL`, `label`, `as_str`
      `"memory_layers"`, `parse`) и юнит-тесты round-trip по образцу
      существующих вариантов; `cargo test -p agentcore` проходит.
- [x] 1.2 Добавить в `ChatSettings` поля `memory_router_enabled: Option<bool>`,
      `memory_short_term_tail: Option<u32>`, `memory_working_max_entries:
      Option<u32>`, `memory_long_term_max_entries: Option<u32>` с
      сериализацией `snake_case`; тест на то, что старый JSON без этих полей
      разбирается как `None` (по образцу
      `old_chat_settings_without_context_strategy_fields_parse_as_none`).
- [x] 1.3 Запушить `agent-cli` в `main`; в `agent-sever` временно убрать
      `[patch]`, выполнить `cargo generate-lockfile`, проверить
      `grep -A2 'name = "agentcore"' Cargo.lock` (коммит, не путь), прогнать
      `cargo test --locked`, вернуть `[patch]` для дальнейшей локальной
      разработки.

## 2. Миграция и хранилище (`agent-sever`)

- [x] 2.1 Написать `migrations/0004_memory_layers.sql`: `ALTER TABLE chats
      ADD COLUMN active_task_id TEXT` с бэкофиллом `active_task_id = id`,
      `CREATE TABLE chat_working_memory`, `CREATE TABLE
      owner_long_term_memory` с индексами (design.md, решение 1); проверить
      на существующей БД с данными — `cargo test` поднимает миграции без
      ошибок, старые чаты читаются.
- [x] 2.2 Добавить в `src/store.rs` структуры и функции рабочей памяти:
      `WorkingMemoryEntry`, `load_working_memory(chat_id, task_id)`,
      `set_working_memory(chat_id, task_id, key, value, source, updated_at)`,
      `delete_working_memory(chat_id, task_id, key)`,
      `finish_task(chat_id, carry_forward_keys)` (удаляет записи задачи,
      возвращает перенесённые); тесты хранилища на CRUD и на очистку при
      смене задачи.
- [x] 2.3 Добавить структуры и функции долговременной памяти:
      `LongTermMemoryEntry`, `load_long_term_memory(owner, limit)`,
      `set_long_term_memory(owner, entry_type, key, value, source,
      source_chat_id, updated_at)`, `delete_long_term_memory(owner, id)`;
      тест на изоляцию по `owner` (запись одного владельца не видна другому).
- [x] 2.4 Реализовать правило приоритета «побеждает более позднее
      `updated_at`» в функциях `set_*` (`UPSERT ... WHERE excluded.updated_at
      >= updated_at`, design.md решение 4); тест: ранняя автоматическая
      операция, затем поздняя ручная — итоговое значение от ручной, и
      наоборот при обратном порядке времени.

## 3. Маршрутизатор (`agent-sever`)

- [x] 3.1 Создать `src/memory.rs`, определить типы операций
      (`MemoryOp { Set, Delete, FinishTask }` с полями `layer`, `key`,
      `value`, `entry_type`, `carry_forward`, `reason`) и `parse_operations`
      по образцу `facts::parse_operations`; тесты разбора: валидный массив,
      мусор вокруг JSON, невалидный JSON → `None`, пустой массив.
- [x] 3.2 Реализовать валидацию операции (неизвестный `layer`, пустой `key`
      кроме `finish_task`, превышение `*_KEY_MAX_CHARS`/`*_VALUE_MAX_CHARS`,
      неизвестный `entry_type` для `long_term`) с журналированием причины
      отказа; тесты на каждую причину отказа по отдельности (specs/memory-layers,
      «Автоматический маршрутизатор распределяет записи по слоям»).
- [x] 3.3 Реализовать `route_after_exchange(state, chat, settings,
      user_message, through_seq)`: промпт с текущим состоянием памяти
      (усечённым по лимитам), вызов модели `AGENTD_MEMORY_ROUTER_MODEL`
      (умолчание — `state.config.model`) с отключёнными reasoning/thinking,
      разбор и применение операций по одной с валидацией; отказ вызова или
      разбора логируется и возвращает пустой результат без изменения памяти
      (specs/memory-layers, «Отказ маршрутизатора не блокирует ответ
      пользователю»). Вызывается после `append_exchange`, как
      `facts::update_after_exchange`.
- [x] 3.4 Реализовать `finish_task` через операцию `finish_task`: перенос
      записей с `carry_forward = true` в долговременную память ДО удаления
      записей рабочей памяти прежней задачи, новый `active_task_id`; тест на
      порядок (перенесённая запись существует уже к моменту удаления
      исходной).
- [x] 3.5 Тест устойчивости: таймаут/ошибка провайдера при вызове
      маршрутизатора (через `wiremock`) не влияет на успешно возвращённый
      основной ответ и не меняет состояние памяти.

## 4. Стратегия контекста `memory_layers` (`agent-sever`)

- [x] 4.1 Реализовать `MemoryLayersImpl: ContextStrategyImpl` в
      `src/memory.rs`: чтение долговременной и рабочей памяти с лимитами и
      вытеснением по `updated_at` (design.md, решение 5), сборка разделов
      `[long_term_section, working_section]` (пропуская пустые), хвост
      краткосрочной памяти через `window::assemble` с
      `effective_memory_tail`.
- [x] 4.2 Добавить ветку `ContextStrategy::MemoryLayers =>
      Box::new(MemoryLayersImpl)` в `impl_for` (`src/context.rs`) — без
      условий по слоям в `handle_chat_in_existing`; тест на то, что
      системное сообщение содержит раздел долговременной памяти раньше
      раздела рабочей памяти при непустых обоих слоях (specs/memory-layers,
      «Стратегия контекста memory_layers»).
- [x] 4.3 Тест: пустой слой не создаёт раздела (аналог
      `facts_section_is_absent_when_empty`).
- [x] 4.4 Тест: хвост краткосрочной памяти ограничен настроенным размером —
      использовать существующие тестовые сценарии `window.rs` как образец
      границ (усечение, сдвиг границы к сообщению пользователя).
- [x] 4.5 Добавить `ContextDto::for_memory_layers(...)` в `src/dto.rs`:
      число записей и объём символов по каждому слою, счётчики
      `applied_set`/`applied_update`/`applied_delete`/`rejected`
      маршрутизатора ПРЕДЫДУЩЕГО сообщения (design.md, решение 5); тест на
      корректность счётчиков при известном наборе применённых и отброшенных
      операций.

## 5. Конфигурация сервиса (`agent-sever`)

- [x] 5.1 Добавить в `src/config.rs` разбор `AGENTD_MEMORY_ROUTER_ENABLED`,
      `AGENTD_MEMORY_ROUTER_MODEL`, `AGENTD_MEMORY_SHORT_TERM_TAIL_MESSAGES`,
      `AGENTD_MEMORY_WORKING_MAX_ENTRIES`,
      `AGENTD_MEMORY_WORKING_VALUE_MAX_CHARS`,
      `AGENTD_MEMORY_WORKING_KEY_MAX_CHARS`,
      `AGENTD_MEMORY_LONG_TERM_MAX_ENTRIES`,
      `AGENTD_MEMORY_LONG_TERM_VALUE_MAX_CHARS`,
      `AGENTD_MEMORY_LONG_TERM_KEY_MAX_CHARS` через существующие
      `parse_bool_named`/`parse_nonzero`; тесты валидных и невалидных
      значений по образцу тестов `AGENTD_SUMMARY_*`.
- [x] 5.2 Добавить `effective_*` функции в `src/memory.rs` для полей,
      дублируемых в `ChatSettings` (настройка чата поверх операторского
      умолчания, по образцу `context::effective_window`); тест
      переопределения умолчания настройкой чата.
- [x] 5.3 Убедиться, что `memory_layers` подчиняется
      `AGENTD_ALLOWED_CONTEXT_STRATEGIES` без отдельного кода — тест:
      список разрешённых стратегий без `memory_layers` отклоняет выбор этой
      стратегии для чата так же, как для прочих стратегий.

## 6. HTTP-эндпоинты (`agent-sever`)

- [x] 6.1 Добавить в `src/dto.rs` DTO чтения/записи/удаления записей памяти
      (`WorkingMemoryEntryDto`, `SetWorkingMemoryRequest`,
      `LongTermMemoryEntryDto`, `SetLongTermMemoryRequest`).
- [x] 6.2 Добавить маршруты в `src/app.rs`: `GET/POST/DELETE
      /v1/chats/{id}/memory/working`, `POST
      /v1/chats/{id}/memory/working/finish-task`, `GET/POST/DELETE
      /v1/memory/long-term` (в области видимости владельца запроса); ошибки —
      единый конверт `{ "error": {...} }`. Тест на каждый метод: успешный
      путь плюс отказ для чужого владельца (403/404 по существующему
      соглашению сервиса).
- [x] 6.3 Тест: ручная запись через HTTP переопределяет более раннюю
      автоматическую операцию того же ключа в рамках одного такта
      (specs/memory-layers, «Ручная запись имеет приоритет над
      автоматической при конфликте»).

## 7. Журналирование (`agent-sever`)

- [x] 7.1 Логировать применённые и отброшенные операции памяти со
      структурными полями (`layer`, `op`, `key`, `accepted`/`reject_reason`);
      добавлять значение записи в лог только при `state.config.log_content`;
      тест (перехват логов или явная проверка функции форматирования) на то,
      что значение отсутствует в записи лога при выключенном флаге.

## 8. Клиент (`agent-cli/crates/cli`)

- [x] 8.1 Добавить в TUI выбор стратегии `memory_layers` и её параметров
      (`memory_short_term_tail`, лимиты числа записей) без пересоздания
      чата — через существующий путь `PATCH /v1/chats/{id}`.
- [x] 8.2 Добавить экран памяти: три раздела (краткосрочная — только
      просмотр хвоста сообщений, рабочая и долговременная — просмотр,
      добавление, правка, удаление записей через новые эндпоинты).
- [x] 8.3 Отобразить разбивку по слоям из блока `context` ответа после
      каждого сообщения на стратегии `memory_layers` (аналогично тому, как
      уже отображаются счётчики других стратегий, если такой индикатор
      существует — иначе завести в том же месте `tui.rs`).
- [x] 8.4 `cargo test -p agentcli` проходит; вручную запустить `cargo run -p
      agentcli -- chat`, пройти сценарий: сменить стратегию на
      `memory_layers`, добавить и увидеть запись рабочей памяти, завершить
      задачу и увидеть перенос в долговременную память.

## 9. README

- [x] 9.1 Обновить `agent-sever/README.md`: стратегия `memory_layers`,
      эндпоинты памяти, переменные `AGENTD_MEMORY_*`, поля `ChatSettings`.
- [x] 9.2 Обновить `agent-cli/README.md`: горячие клавиши и экран памяти
      TUI, выбор стратегии `memory_layers`.

## 10. Практический прогон и финальная проверка

- [x] 10.1 Выполнить сценарий из 12–15 сообщений (сбор требований к
      небольшой фиче со сменой темы в середине) дважды — на `memory_layers`
      и на `sliding_window` с тем же размером окна; записать в
      `openspec/changes/add-memory-layers/comparison.md` таблицу «какие
      данные попали в каждый слой» и сравнение ответов на контрольные
      вопросы, требующие информации из начала диалога и из предыдущего чата
      того же владельца.
- [x] 10.2 Итоговый прогон `cargo test` в `agent-cli` и в `agent-sever`, а
      также `cargo test --locked` в `agent-sever` без `[patch]` — все
      проходят, зафиксировать вывод в описании PR/итогового отчёта.
