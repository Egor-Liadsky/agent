## Context

См. `proposal.md — Why` про мотивацию. Здесь — что уже есть после
`add-memory-layers` и как это перестраивается.

`agent-sever/src/context.rs` — единственная точка ветвления по стратегии:
приватный трейт `ContextStrategyImpl` с методом `assemble`, реализации —
приватные структуры (`SummaryImpl`, `WindowImpl`, `FactsImpl`,
`BranchingImpl`, `MemoryLayersImpl`), подбираемые в `impl_for` по значению
`ContextStrategy`. Каждая реализация возвращает `Assembled { history,
context: ContextDto, sections: Vec<String> }` — `sections` вставляются в
системное сообщение функцией `system_message::build` в фиксированном
порядке, `context` уходит клиенту как блок наблюдаемости. Сегодня
`MemoryLayersImpl::assemble` (`src/memory.rs`) делает три вещи разом: читает
долговременную и рабочую память, строит их разделы, и — отдельно —
пересобирает дословный хвост краткосрочной истории через `summary::tail_boundary`
напрямую, той же функцией, которой уже пользуется `WindowImpl` (`window.rs`)
для стратегии `sliding_window`. Это и есть дублирование, которое разводит
изменение: у краткосрочного слоя нет причины иметь собственную копию логики
усечения окна, когда для этого уже есть стратегия.

`ContextDto` (`src/dto.rs`) — один плоский `struct` с опциональными полями
под все стратегии сразу; у каждой стратегии свой конструктор
(`for_summary`, `for_window`, `for_facts`, `for_branching`,
`for_memory_layers`), выставляющий поля своей стратегии и `None` для
остальных. `for_memory_layers` также единолично владеет полем `strategy:
ContextStrategy::MemoryLayers` — после этого изменения ни один конструктор
не может одновременно отражать «эта стратегия» и «эта память», раз память
больше не стратегия.

`ChatSettings` (`agentcore::config`, `agent-cli`) хранит
`context_strategy: Option<ContextStrategy>` и параллельно
`memory_router_enabled`/`memory_short_term_tail`/`memory_working_max_entries`/`memory_long_term_max_entries`
— поля памяти уже отделены от `context_strategy` физически (это разные
поля), просто раньше применялись только при одном её значении.

## Goals / Non-Goals

**Goals:**
- Слоистая память включается независимым флагом и работает поверх любой из
  четырёх оставшихся стратегий контекста без изменения их поведения.
- Краткосрочная история собирается один раз — стратегией; слой памяти не
  хранит и не пересчитывает свой размер окна.
- Блок `context` ответа несёт поля стратегии и поля памяти одновременно.

**Non-Goals:**
- Изменение поведения самих стратегий `summary`/`sliding_window`/`facts`/`branching`
  — они не знают о существовании памяти, и после этого изменения не должны
  узнать.
- Миграция существующих данных `chat_working_memory`/`owner_long_term_memory`
  — хранилище не меняется, меняется только то, откуда берётся флаг
  включения и куда встраиваются разделы.
- Ретроактивная миграция чатов с `context_strategy: "memory_layers"` в
  автоматическом режиме — это ручное действие оператора/клиента (см.
  `proposal.md`, раздел `Migration` требования «Стратегия контекста
  memory_layers»).

## Decisions

### 1. Слой памяти оборачивает результат стратегии, а не подменяет его

`context::assemble` остаётся точкой, которая сначала вызывает
`impl_for(strategy).assemble(...)` как сегодня — без каких-либо условий по
памяти внутри самих реализаций стратегий, — а затем, если слоистая память
включена, добавляет разделы памяти в `assembled.sections` ПЕРЕД сборкой
системного сообщения и обогащает `assembled.context` полями памяти:

```rust
pub async fn assemble(
    state: &AppState, chat: &store::Chat, settings: &ChatSettings,
    owner: &str, strategy: ContextStrategy,
    stored: Vec<store::ChatMessage>, new_message: Message,
) -> Assembled {
    let ctx = StrategyCtx { state, chat, settings, owner };
    let mut assembled = impl_for(strategy).assemble(&ctx, stored, new_message).await;
    if crate::memory::effective_layers_enabled(state, settings) {
        let memory = crate::memory::layers(state, chat, settings, owner).await;
        let mut sections = memory.sections;           // [long_term, working]
        sections.extend(std::mem::take(&mut assembled.sections));
        assembled.sections = sections;
        crate::memory::merge_into_context(&mut assembled.context, &memory, &assembled.history);
        let router = /* как сегодня, из AppState.memory_route_outcomes */;
        crate::memory::merge_router_counters(&mut assembled.context, router);
    }
    let system_message = crate::system_message::build(&state.config.system_prompt, &assembled.sections);
    assembled.history.insert(0, system_message);
    assembled
}
```

`MemoryLayersImpl` и ветка `ContextStrategy::MemoryLayers` в `impl_for`
удаляются целиком — из пяти стратегий `impl_for` снова разбирает четыре.
`memory.rs` больше не реализует `ContextStrategyImpl`: он даёт свободные
функции (`layers`, `merge_into_context`, `merge_router_counters`,
`route_after_exchange`, `effective_*`), вызываемые из `context::assemble` и
`app.rs`, — тот же приём, что уже применяют `window.rs`/`facts.rs`/`branch.rs`
для самих стратегий.

Порядок разделов: память (`[long_term, working]`) идёт ПЕРЕД разделами
стратегии (`facts_section`/`summary_section`, если они есть у действующей
стратегии). Обоснование: память — контекст владельца и задачи, который
предшествует по смыслу деталям текущего диалога (фактам, пересказу); это
согласуется с уже принятым в `add-memory-layers` порядком «долговременная
раньше рабочей» — то же рассуждение «от общего к частному» распространяется
на пару «память — стратегия».

Альтернатива, отклонённая: держать сборку разделов памяти внутри каждой
реализации стратегии (пять мест правки вместо одного) — отклонена по той же
причине, по которой сегодня системное сообщение собирается один раз в
`context::assemble`, а не в каждой `*Impl` — не размножать точку, которую
проще держать одной.

### 2. Краткосрочный слой в `context` — отчёт по факту, не отдельная настройка

`memory_short_term_tail` (`ChatSettings`) и `AGENTD_MEMORY_SHORT_TERM_TAIL_MESSAGES`
удаляются. Поля `memory_short_term_messages`/`memory_short_term_chars`
блока `context` остаются, но считаются не отдельным вызовом усечения, а по
факту — длиной и суммой символов `assembled.history` ПОСЛЕ работы стратегии
(до вставки системного сообщения, чтобы не считать сам системный текст).
Эта пара чисел одинаково осмысленна для любой стратегии: у `sliding_window`
и `facts` это её окно, у `summary` — дословный хвост после компактизации, у
`branching` — длина цепочки активной ветки.

Альтернатива, отклонённая: оставить `memory_short_term_tail` как
независимый размер окна, усекающий уже собранную стратегией историю ещё
раз, — отклонена: два независимых размера окна (стратегии и памяти) над
одной и той же историей не имеют содержательного смысла и означали бы
скрытое, недокументированное двойное усечение.

### 3. Маршрутизатор запускается по флагу, не по стратегии

`app.rs`, вызов `route_after_exchange` после `append_exchange`: условие
`strategy == ContextStrategy::MemoryLayers` заменяется на
`crate::memory::effective_layers_enabled(&state, &settings)`. Сам
маршрутизатор (`route_after_exchange`, валидация, применение операций,
`finish_task`) не меняется — он уже не знал о `ContextStrategy` напрямую,
принимал `chat`/`settings`/`owner`.

### 4. `ContextDto`: обогащение вместо отдельного конструктора

`ContextDto::for_memory_layers(...)` удаляется. Вместо него —
`crate::memory::merge_into_context(&mut ContextDto, &MemoryLayers, &history)`,
заполняющая поля памяти (`memory_long_term_entries`, …) в уже построенном
`ContextDto` действующей стратегии, не трогая её собственные поля и не
трогая `strategy` (он остаётся действующей стратегией, а не
`MemoryLayers` — этого значения перечисления больше нет). `strategy`
`ContextDto` теперь всегда одна из четырёх оставшихся.

### 5. `BREAKING`: `ContextStrategy` теряет вариант, миграция — ручная

Убрать вариант аккуратно (deprecate-then-remove) не имеет смысла: это
единственный владелец репозитория, значение `memory_layers` прожило в
`main` считаные дни, и обратная совместимость с внешними интеграциями не
требуется (см. корневой `CLAUDE.md`). Прямое удаление варианта, с
руководством по миграции в `specs/memory-layers/spec.md` (требование
«Стратегия контекста memory_layers», REMOVED).

## Risks / Trade-offs

- [Риск] Существующие чаты с сохранённым `context_strategy: "memory_layers"`
  в БД перестают разбираться (`ContextStrategy::parse` не узнает значение) →
  Смягчение: `store::parse_settings` уже трактует несовместимое изменение
  `ChatSettings` как повод вернуть умолчания сервиса, а не уронить чат
  (design.md `add-memory-layers`, риск в конце документа) — старый чат не
  падает, а получает операторскую стратегию по умолчанию и выключенную
  память; владелец включает `memory_layers_enabled` вручную, если нужно
  прежнее поведение. Данных это не касается: `chat_working_memory`/
  `owner_long_term_memory` не читаются через `ChatSettings` и не страдают.
- [Риск] Два места, где раньше диагностировалась «стратегия», теперь должны
  диагностировать «стратегия ИЛИ память» — легко забыть про комбинацию при
  правке одного из них позже → Смягчение: тесты `src/tests.rs` покрывают
  все четыре стратегии с включённой и выключенной памятью явно (раздел
  задач), а не только одну стратегию с памятью, как было в
  `add-memory-layers`.

## Migration Plan

1. `agent-cli/crates/core`: убрать `ContextStrategy::MemoryLayers`, убрать
   `ChatSettings.memory_short_term_tail`, добавить
   `ChatSettings.memory_layers_enabled`; прогнать `cargo test`, запушить в
   `main`.
2. `agent-sever`: временно убрать `[patch]`, `cargo generate-lockfile`,
   проверить `Cargo.lock` (коммит, не путь), `cargo test --locked`, вернуть
   `[patch]`.
3. `agent-sever`: `context::assemble` оборачивает результат стратегии
   разделами и полями памяти; `MemoryLayersImpl` и
   `ContextDto::for_memory_layers` удаляются; `app.rs` переключает условие
   запуска маршрутизатора на флаг; `src/config.rs` — новая переменная,
   удаление старой.
4. `agent-cli/crates/cli`: TUI — переключатель памяти отдельно от перебора
   стратегий; README обоих репозиториев.
5. Откат: миграций БД нет (задача уже отмечена в `proposal.md — Impact`),
   откат — возврат предыдущего образа сервиса и предыдущего коммита ядра;
   данные слоёв памяти не теряются при откате в любую сторону.
