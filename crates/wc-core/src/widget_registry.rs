use anyhow::{Result, bail};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub type WidgetTypeId = &'static str;

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetInstanceConfig {
    pub id: String,
    pub widget_type: String,
    pub enabled: bool,
    pub layer_z: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh_seconds: u64,
    pub fps_cap: f32,
    pub settings: BTreeMap<String, String>,
}

impl WidgetInstanceConfig {
    pub fn new(widget_type: &str, id: &str) -> Self {
        Self {
            id: id.to_string(),
            widget_type: widget_type.to_string(),
            enabled: true,
            layer_z: 10,
            pos_x: 0,
            pos_y: 0,
            width: 640,
            height: 360,
            refresh_seconds: 60,
            fps_cap: 1.0,
            settings: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetRuntimeContext {
    pub cycle: u64,
    pub cache_dir: PathBuf,
    pub now_unix: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetResolvedPayload {
    pub text: String,
    pub image_path: Option<PathBuf>,
}

pub trait WidgetPlugin: Send + Sync {
    fn type_id(&self) -> WidgetTypeId;
    fn display_name(&self) -> &'static str;
    fn default_instance(&self) -> WidgetInstanceConfig;
    fn validate(&self, instance: &WidgetInstanceConfig) -> Result<()>;
    fn resolve(
        &self,
        instance: &WidgetInstanceConfig,
        ctx: &WidgetRuntimeContext,
    ) -> Result<WidgetResolvedPayload>;
}

#[derive(Default)]
pub struct WidgetRegistry {
    plugins: BTreeMap<String, Box<dyn WidgetPlugin>>,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: Box<dyn WidgetPlugin>) -> Result<()> {
        let id = plugin.type_id();
        if id.trim().is_empty() {
            bail!("widget type id cannot be empty");
        }
        if self.plugins.contains_key(id) {
            bail!("widget type '{id}' already registered");
        }
        self.plugins.insert(id.to_string(), plugin);
        Ok(())
    }

    pub fn get(&self, type_id: &str) -> Option<&dyn WidgetPlugin> {
        self.plugins.get(type_id).map(|p| p.as_ref())
    }

    pub fn all(&self) -> impl Iterator<Item = &dyn WidgetPlugin> {
        self.plugins.values().map(|p| p.as_ref())
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

pub const BUILTIN_WIDGET_TYPE_IDS: &[&str] =
    &["quote", "clock", "weather", "news", "news_ticker2", "cams"];

#[cfg(test)]
mod tests {
    use super::{
        WidgetInstanceConfig, WidgetPlugin, WidgetRegistry, WidgetResolvedPayload,
        WidgetRuntimeContext,
    };
    use anyhow::Result;
    use std::path::PathBuf;

    struct StubPlugin {
        id: &'static str,
        label: &'static str,
    }

    impl WidgetPlugin for StubPlugin {
        fn type_id(&self) -> &'static str {
            self.id
        }

        fn display_name(&self) -> &'static str {
            self.label
        }

        fn default_instance(&self) -> WidgetInstanceConfig {
            WidgetInstanceConfig::new(self.id, &format!("{}_main", self.id))
        }

        fn validate(&self, _instance: &WidgetInstanceConfig) -> Result<()> {
            Ok(())
        }

        fn resolve(
            &self,
            _instance: &WidgetInstanceConfig,
            _ctx: &WidgetRuntimeContext,
        ) -> Result<WidgetResolvedPayload> {
            Ok(WidgetResolvedPayload {
                text: format!("{} payload", self.id),
                image_path: Some(PathBuf::from(format!("/tmp/{}.png", self.id))),
            })
        }
    }

    #[test]
    fn registry_rejects_duplicate_type_id() {
        let mut registry = WidgetRegistry::new();
        registry
            .register(Box::new(StubPlugin {
                id: "news",
                label: "News",
            }))
            .expect("first register should succeed");
        let second = registry.register(Box::new(StubPlugin {
            id: "news",
            label: "News duplicate",
        }));
        assert!(second.is_err(), "duplicate ids must be rejected");
    }

    #[test]
    fn registry_iterates_plugins_in_stable_order() {
        let mut registry = WidgetRegistry::new();
        registry
            .register(Box::new(StubPlugin {
                id: "weather",
                label: "Weather",
            }))
            .expect("register weather");
        registry
            .register(Box::new(StubPlugin {
                id: "news",
                label: "News",
            }))
            .expect("register news");
        registry
            .register(Box::new(StubPlugin {
                id: "quote",
                label: "Quote",
            }))
            .expect("register quote");

        let ids = registry
            .all()
            .map(|plugin| plugin.type_id().to_string())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["news", "quote", "weather"]);
    }

    #[test]
    fn plugin_contract_resolve_can_be_called_through_registry() {
        let mut registry = WidgetRegistry::new();
        registry
            .register(Box::new(StubPlugin {
                id: "cams",
                label: "Cams",
            }))
            .expect("register cams");
        let plugin = registry.get("cams").expect("cams plugin should exist");
        let instance = plugin.default_instance();
        let payload = plugin
            .resolve(
                &instance,
                &WidgetRuntimeContext {
                    cycle: 9,
                    cache_dir: PathBuf::from("/tmp"),
                    now_unix: 123,
                },
            )
            .expect("resolve should work");
        assert_eq!(payload.text, "cams payload");
        assert_eq!(payload.image_path, Some(PathBuf::from("/tmp/cams.png")));
    }
}
