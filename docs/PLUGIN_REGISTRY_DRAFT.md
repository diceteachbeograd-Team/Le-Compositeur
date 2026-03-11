# Plugin Registry Draft

Last updated: 2026-03-11
Status: draft + stage-A scaffold complete and stage-B dual-path CLI integration started

## Problem

Widget logic is currently wired through central branching in CLI/GUI/renderer paths.
Adding a new widget requires touching multiple core files and increases regression risk.

## Goal

Introduce a plugin-style widget registry so new widget types can be added with minimal central orchestration changes.

## Scope (Phase 1-2)

- Runtime registry abstraction for built-in widgets (`quote`, `clock`, `weather`, `news`, `news_ticker2`, `cams`).
- Unified widget metadata contract for config schema + GUI generation.
- Deterministic ordering/layering and performance budget controls per widget instance.
- No dynamic loading (`.so`/`.dll`) in first iteration. Start with compile-time registration.

## Non-Goals (initial)

- Third-party binary plugin loading.
- Sandboxed plugin execution.
- Cross-process plugin API.

## Proposed Core Contracts

```rust
pub type WidgetId = &'static str;

pub struct WidgetInstanceConfig {
    pub id: String,              // stable instance id, e.g. "news_main"
    pub widget_type: String,     // plugin type id, e.g. "news"
    pub enabled: bool,
    pub layer_z: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh_seconds: u64,
    pub fps_cap: f32,
    pub settings: serde_json::Value, // plugin-specific settings payload
}

pub struct WidgetRuntimeContext<'a> {
    pub cycle: u64,
    pub cache_dir: &'a std::path::Path,
    pub now_unix: u64,
}

pub struct WidgetResolvedPayload {
    pub text: String,
    pub image_path: Option<std::path::PathBuf>,
}

pub trait WidgetPlugin: Send + Sync {
    fn type_id(&self) -> WidgetId;
    fn display_name(&self) -> &'static str;
    fn default_instance(&self) -> WidgetInstanceConfig;
    fn schema_fragment_json(&self) -> serde_json::Value;
    fn validate(&self, instance: &WidgetInstanceConfig) -> anyhow::Result<()>;
    fn resolve(
        &self,
        instance: &WidgetInstanceConfig,
        ctx: &WidgetRuntimeContext<'_>,
    ) -> anyhow::Result<WidgetResolvedPayload>;
}
```

## Registry Shape

```rust
pub struct WidgetRegistry {
    plugins: std::collections::BTreeMap<String, Box<dyn WidgetPlugin>>,
}

impl WidgetRegistry {
    pub fn register(&mut self, plugin: Box<dyn WidgetPlugin>) -> anyhow::Result<()>;
    pub fn get(&self, type_id: &str) -> Option<&dyn WidgetPlugin>;
    pub fn all(&self) -> impl Iterator<Item = &dyn WidgetPlugin>;
}
```

Built-in bootstrapping:
- `register_quote_widget`
- `register_clock_widget`
- `register_weather_widget`
- `register_news_widget`
- `register_news_ticker2_widget`
- `register_cams_widget`

## Integration Plan

1. `wc-core`
- Add `widget_instances: Vec<WidgetInstanceConfig>` as normalized config field.
- Keep legacy fields for compatibility.
- Add migration path from legacy fields into default widget instances.

2. `wc-cli`
- Replace central widget-specific branches with registry iteration:
  - load + validate widget instances
  - sort by `layer_z`
  - resolve payload via plugin trait
  - pass resolved payloads into renderer mapping
- Keep fallback mode while migrating.

3. `wc-render`
- Accept a widget-render input list where possible.
- Preserve current text/image rendering behavior while reducing hard-coded widget branches over time.

4. `wc-gui`
- Generate widget settings panes from plugin schema fragments.
- Keep existing tabs as compatibility layer during transition.
- Add per-instance add/remove/duplicate controls later (Phase 3).

## Migration Stages

### Stage A: Registry introduction (no behavior change)
- Add registry types and built-in plugin wrappers around existing logic.
- Existing config and UI remain unchanged.

### Stage B: Dual-path execution
- CLI resolves widgets through registry but keeps legacy fallback.
- Add feature flag or internal toggle for safe rollout.

### Stage C: Config normalization
- Introduce `widget_instances` as canonical model.
- `migrate` command writes canonical format and backup.

### Stage D: UI generation
- GUI reads plugin schema fragments and renders dynamic sections.
- Remove duplicated hard-coded editor branches incrementally.

## Risks and Controls

- Risk: behavioral drift while wrapping existing widgets.
  - Control: keep regression tests for known widget outputs and ordering.
- Risk: schema/UI mismatch.
  - Control: single source of truth from plugin schema fragment.
- Risk: migration complexity for existing user configs.
  - Control: dual-read strategy + explicit `migrate` backups.

## Immediate Next Tasks

1. Create `wc-core::widget_registry` module with the trait and registry container.
2. Implement wrappers for `quote`, `clock`, `weather`, `news`, `news_ticker2`, `cams`.
3. Add CLI integration behind a guarded path and compare output parity in tests.
4. Add docs section to architecture and test matrix for registry migration checks.
