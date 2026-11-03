# 11 — API для аддонов

Требование заказчика: возможность поддержки аддонов должна быть заложена
изначально, чтобы любой мог расширять мод своими идеями.

**Главный критерий успеха API:** аддон, добавляющий новый ген, новый симптом,
новый профиль хоста и новые правила порчи мира, должен быть **датапаком без
единой строчки Java**. Если это невозможно — API спроектирован неправильно.

---

## 1. Три уровня расширения

| Уровень | Кто это делает | Что может | Чем |
|---------|----------------|-----------|-----|
| **1. Датапак** | Автор модпака, игрок | Новые гены из существующих типов, симптомы, профили хостов, правила порчи, узлы исследований, рецепты, конфликты генов, шасси | JSON. Ноль кода |
| **2. Скрипты** | Автор модпака | То же + условная логика | KubeJS / CraftTweaker |
| **3. Java-аддон** | Моддер | Новые **типы** механик, новые машины, новые эффекты, подписка на события | `biohazard-api` |

Принцип: **90% расширений должны быть возможны на уровне 1.**

---

## 2. Уровень 1: датапак-реестры

Регистрируются через `DataPackRegistryEvent.NewRegistry`. Все с `Codec`,
синхронизируются клиенту по необходимости.

| Реестр | Что описывает |
|--------|---------------|
| `biohazard:gene` | Ген: категория, size, instability, эффекты, где встречается |
| `biohazard:gene_interaction` | Конфликт/синергия пары генов, уровень, радиус |
| `biohazard:chassis_layout` | Раскладка цепи: промоторы, хрупкие зоны, терминальные области |
| `biohazard:symptom` | Симптом: тип + параметры + кривая тяжести + визуал |
| `biohazard:transmission_vector` | Путь передачи: радиус, условия, что ему мешает |
| `biohazard:receptor` | Рецептор и карта соседства (куда может мутировать) |
| `biohazard:host_profile` | Профиль хоста: тег сущностей, рецепторы, иммунитет, поведение |
| `biohazard:cell_line` | Клеточная линия: вид, рецепторы, предел пассажей, требования |
| `biohazard:hazard_class` | Класс опасности: условия, требуемый BSL |
| `biohazard:mutation_rule` | Тип мутации: вес, условия, что меняет |
| `biohazard:corruption_stage` | Стадия порчи мира: порог, правила трансформации блоков |
| `biohazard:decontamination_method` | Метод деконтаминации: эффективность, против чего не работает |
| `biohazard:research_node` | Узел дерева: стоимость, требования, что открывает |
| `biohazard:research_trigger` | За что дают очки и сколько |
| `biohazard:protection_profile` | Защита экипировки по векторам |
| `biohazard:lab_procedure` | Операция: тир, параметры, диапазоны, риск |
| `biohazard:natural_pathogen` | Шаблон природного патогена: где генерируется, какие гены |
| `biohazard:strain_name_pool` | Пул имён для автогенерации названий штаммов |

### Пример: новый ген датапаком

`data/myaddon/biohazard/gene/bone_density.json`

```json
{
  "category": "biohazard:symptom",
  "size": 3,
  "instability": 6,
  "effects": [
    {
      "type": "biohazard:attribute",
      "attribute": "minecraft:generic.armor",
      "operation": "add_value",
      "amount": 3.0,
      "scale_with_expression": true
    },
    {
      "type": "biohazard:attribute",
      "attribute": "minecraft:generic.movement_speed",
      "operation": "add_multiplied_base",
      "amount": -0.10
    },
    {
      "type": "biohazard:visual_layer",
      "texture": "myaddon:textures/infection/bone_plates.png",
      "emissive": false,
      "applies_to": "#biohazard:host/mammal"
    }
  ],
  "requires_active_load": true,
  "found_in": [
    { "source": "biohazard:deep_cave_spores", "weight": 3 }
  ]
}
```

### Пример: новое взаимодействие генов

```json
{
  "gene_a": "myaddon:bone_density",
  "gene_b": "biohazard:symptom/myostatin_block",
  "level": "synergy",
  "stability_modifier": 10,
  "interference_range": 2,
  "discovery_hint": "Структурные и мышечные адаптации усиливают друг друга"
}
```

`discovery_hint` — текст, который попадает в Кодекс **после того, как игрок
откроет правило экспериментом**. Аддон участвует в системе знания наравне с
базовым модом.

### Пример: свои мобы как хосты — вообще без файла реестра

Достаточно тега:

`data/myaddon/tags/entity_type/biohazard/host/mammal.json`
```json
{ "values": ["myaddon:giant_rat", "myaddon:cave_bear"] }
```

Всё: мобы аддона болеют, передают инфекцию, получают визуал заражения,
попадают в матрицу хостов и в симуляцию очагов. **Ноль кода, ноль знаний об
устройстве мода.** Это главный показатель того, что архитектура правильная.

---

## 3. Уровень 3: Java-аддон

### Структура артефактов

```
dev.projectbiohazard:biohazard-api:<version>     ← публикуем, стабильное
dev.projectbiohazard:biohazard:<version>         ← реализация, НЕ API
```

`biohazard-api` не зависит от реализации и содержит только интерфейсы, реестры,
типы данных, события. Аддон компилируется против API, в рантайме работает с
реализацией.

### Реестры типов — паттерн «тип в коде, экземпляр в JSON»

Это ключевой паттерн всего API. Аналог `LootItemFunctionType` / `BlockEntityType`
в ванили.

```java
// Аддон регистрирует ТИП (код)
public final class ChillingSymptom implements SymptomBehavior {
    public static final MapCodec<ChillingSymptom> CODEC = ...;

    @Override
    public void onTick(HostContext host, float severity) { ... }

    @Override
    public void onApply(HostContext host) { ... }

    @Override
    public void onRemove(HostContext host) { ... }
}

// Регистрация
DeferredRegister<SymptomType<?>> TYPES =
    DeferredRegister.create(BiohazardRegistries.SYMPTOM_TYPE, "myaddon");

TYPES.register("chilling", () -> new SymptomType<>(ChillingSymptom.CODEC));
```

Дальше **десятки симптомов** создаются датапаком на базе одного типа. Именно это
делает уровень 1 мощным.

### Реестры типов в API

```
BiohazardRegistries.SYMPTOM_TYPE           // поведение симптома
BiohazardRegistries.GENE_EFFECT_TYPE       // эффект гена
BiohazardRegistries.TRANSMISSION_TYPE      // логика пути передачи
BiohazardRegistries.MUTATION_TYPE          // логика мутации
BiohazardRegistries.CORRUPTION_ACTION_TYPE // действие порчи мира
BiohazardRegistries.PROCEDURE_TYPE         // лабораторная операция
BiohazardRegistries.ASSAY_TYPE             // анализ для дедукции
BiohazardRegistries.DECONTAMINATION_TYPE   // метод очистки
BiohazardRegistries.RESEARCH_TRIGGER_TYPE  // источник очков
BiohazardRegistries.FITNESS_FACTOR_TYPE    // фактор приспособленности для отбора
```

Последний особенно важен: аддон может **влиять на эволюцию**, добавив свой
фактор приспособленности (например, «в биоме аддона отбор благоприятствует
холодостойким штаммам»).

### События (NeoForge event bus)

```java
// --- Заражение ---
InfectionAttemptEvent          // cancellable, можно менять дозу и порог
InfectionStageChangeEvent      // cancellable
InfectionCuredEvent
ExposureCalculationEvent       // модификация защиты/экспозиции

// --- Геном и эволюция ---
MutationEvent                  // cancellable, можно подменить результат
StrainCreatedEvent
RecombinationEvent
GenomeValidationEvent          // можно запретить сборку или разрешить запрещённое
PhenotypeComputeEvent          // ⚠ пост-обработка фенотипа. Кэшируется!

// --- Мир ---
ContaminationChangeEvent       // cancellable
BlockCorruptionEvent           // cancellable — главное для совместимости защит
OutbreakRegisteredEvent
OutbreakSimulationEvent        // можно вмешаться в SIR-шаг

// --- Сдерживание ---
ContainmentIntegrityEvent      // модификация расчёта герметичности
ContainmentBreachEvent
RoomScanEvent                  // можно добавить свои блоки как часть помещения

// --- Медицина и исследования ---
VaccinationEvent
ImmunityCalculationEvent
ResearchPointsEvent            // cancellable, можно менять количество
ResearchUnlockEvent

// --- Процедуры ---
ProcedureStartEvent
ProcedureResultEvent           // можно менять выход и исход
```

Правило: **каждое событие, которое меняет мир, — отменяемое.** Это то, что
позволяет клейм-модам и серверным плагинам интегрироваться без миксинов.

### Интерфейсы для чужих модов

```java
// Чужая броня заявляет защиту
public interface IProtectiveEquipment {
    float getProtection(ItemStack stack, ResourceKey<TransmissionVector> vector);
    boolean isSealed(ItemStack stack);
    void onContaminated(ItemStack stack, float amount);
}

// Чужой блок участвует в сдерживании
public interface IContainmentComponent {
    ContainmentRole role();        // SEALING | FILTRATION | AIRLOCK | PRESSURE | DECON
    float contribution(BlockState state, Level level, BlockPos pos);
}

// Чужой блок поглощает/источает загрязнение
public interface IContaminationSource {
    float emission(Level level, BlockPos pos);
}

// Чужая сущность со своей иммунной моделью
public interface ICustomHost {
    HostProfile resolveProfile(LivingEntity entity);
}
```

### Публичный фасад (единственная точка входа в реализацию)

```java
public interface BiohazardAPI {
    static BiohazardAPI get();

    StrainLookup      strains();        // чтение штаммов мира
    GenomeTools       genomes();        // валидация, фенотип, укладка
    InfectionTools    infections();     // заразить, вылечить, запросить состояние
    ContaminationView contamination();  // чтение и изменение загрязнения
    ResearchTools     research();       // выдать очки, проверить разблокировку
    KnowledgeTools    knowledge();      // что игрок знает (важно для UI аддонов!)
}
```

`KnowledgeTools` обязателен: аддон, делающий свой UI, **не должен** иметь доступ
к данным, которых игрок не открыл. Иначе аддон сломает ядро дизайна.

---

## 4. KubeJS / CraftTweaker

Отдельный compat-модуль. Что должно быть доступно скриптами:

```js
// Создание гена
BiohazardEvents.genes(event => {
    event.create('myaddon:frost_adaptation')
         .category('env')
         .size(2)
         .instability(4)
         .attributeEffect('minecraft:generic.movement_speed', 0.1)
         .foundIn('biohazard:cold_biome_samples', 5)
})

// Вмешательство в заражение
BiohazardEvents.infectionAttempt(event => {
    if (event.host.type === 'minecraft:villager' && event.strain.hazardClass >= 4) {
        event.multiplyDose(0.5)   // в этом модпаке жители крепче
    }
})

// Правило порчи
BiohazardEvents.corruption(event => {
    event.stage(3).replace('minecraft:stone', 'myaddon:infected_stone', 5)
})
```

---

## 5. Политика стабильности API

| Правило | |
|---------|--|
| Версионирование | Семантическое. `biohazard-api` версионируется **отдельно** от мода |
| Аннотации | `@ApiStatus.Experimental` / `@Internal` / `@NonExtendable`. Всё без аннотации — стабильно |
| Удаление | Только в major-версии, с `@Deprecated(forRemoval = true)` минимум на одну minor-версию раньше |
| Внутреннее | Пакет `dev.projectbiohazard.impl` — **никогда** не API, даже если публичный класс |
| Changelog | Отдельный `API-CHANGELOG.md`, обязателен к обновлению в каждом PR, меняющем API |
| Тесты | API покрыт тестами на совместимость: пример-аддон собирается в CI |

---

## 6. Пример-аддон в репозитории

Обязателен. Модуль `biohazard-addon-example`, который:

- добавляет 2 гена **датапаком** (демонстрация уровня 1);
- добавляет 1 новый `SymptomType` **кодом** (демонстрация уровня 3);
- добавляет свой моб как хоста **одним тегом**;
- подписывается на одно событие;
- добавляет своё правило порчи мира;
- собирается в CI как проверка, что API не сломан.

Это одновременно и документация, и регрессионный тест. Самый эффективный способ
удержать API пригодным.

---

## 7. Чек-лист для ревью любой новой механики

Перед мерджем любой фичи в мод:

- [ ] Может ли аддон добавить свой вариант этой механики датапаком?
- [ ] Если нет — есть ли реестр типов для расширения кодом?
- [ ] Есть ли событие для вмешательства?
- [ ] Если механика меняет мир — отменяемое ли событие?
- [ ] Работает ли механика с сущностями/блоками из чужих модов (через теги)?
- [ ] Не утекает ли через API знание, которого у игрока нет?
- [ ] Обновлён ли `API-CHANGELOG.md`?
- [ ] Покрыто ли примером в `biohazard-addon-example`, если это новая точка расширения?
