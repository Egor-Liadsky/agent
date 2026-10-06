**repeatOnLifecycle(STARTED) — ключевые моменты**

- `lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) { flow.collect {} }` — suspend API для lifecycle-aware сбора. При входе в STARTED блок запускается в новой корутине, при уходе ниже STARTED (onStop) отменяется, при возврате — заново; это полный **cancel+restart** (`...docx:287`, `:306`).
- Отличие от `launchWhenStarted` (deprecated): тот лишь приостанавливал корутину, upstream продолжал эмитить и расти в буферах (`:287`, `:306`).
- `flowWithLifecycle(lifecycle, STARTED)` — оператор над одним Flow (можно продолжать цепочку и собрать один раз), внутри `callbackFlow`. Правило: несколько Flow — `repeatOnLifecycle` с общим collect, одиночный — любой вариант (`:288`).
- Зачем перезапуск: в фоне upstream (Room, сеть) полностью останавливается — нет батареи и устаревших эмиссий; цена — повторный запуск, дорогие источники кэшируют через `stateIn`/`shareIn` во ViewModel (`:290`, `:318`).
- Контекст: гарантированы только onStop и onSaveInstanceState; сохранение черновика — в onStop; регистрация результата после STARTED запрещена; неотменённые корутины вне `lifecycleScope` — утечка (`:44`, `:52`, `:50`).

Источник: конспект Android Middle Interview, разделы 2.1.1, 2.1.2, 8.4.1, 8.5, 9.1.2
