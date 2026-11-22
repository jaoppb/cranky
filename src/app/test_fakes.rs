//! Hand-written stand-ins for the ports `CrankyApp` is driven through in tests.

use crate::app::registry::WatchLayoutSender;
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, ShowFloating};
use crate::features::module_runtime::ports::{
    LayoutEvent, LayoutSender, LazySite, ModuleRegistryPort, ModuleRuntimeDependencies,
    ModuleSetup, RegistryLoadError, SpawnedModule,
};
use crate::features::styling::domain::StyleSheetName;
use crate::features::systray::domain::{SystrayActionCall, SystrayId};
use crate::features::systray::ports::{SniPort, SniPortError};
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use crate::shared::dbus::subscription_manager::DbusSubscriptionManager;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::{ModuleId, ModuleKey, ModuleName};
use crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory;
use crate::shared::wayland::ports::{AppReadModel, DisplayServerError, DisplayServerPort};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

type Deps = ModuleRuntimeDependencies<
    TinySkiaCanvasFactory,
    Sender<LayoutEvent>,
    Sender<DisplayCommand>,
    Sender<UiCommand>,
>;

type Setup<'a> = ModuleSetup<
    'a,
    TinySkiaCanvasFactory,
    Sender<LayoutEvent>,
    Sender<DisplayCommand>,
    Sender<UiCommand>,
>;

/// Which actor, if any, `parent` embeds at a key.
type SiteResolver = Box<dyn Fn(Option<ModuleId>, &ModuleKey) -> Option<ModuleId> + Send + Sync>;

/// The parent a site was spawned under, if any.
type ParentLookup = Box<dyn Fn(ModuleId) -> Option<ModuleId> + Send + Sync>;

/// A registry that answers from the data a test puts in its fields.
///
/// Nothing is loaded or spawned: `spawn_all` hands out `senders` once, and
/// `spawn_module` counts its calls in `spawn_module_calls` and keeps what each
/// received in `spawn_requests`. It fails with `ModuleNotFound` unless
/// `spawned_id` is set, in which case it lists that id in `module_ids` and
/// hands out a sender for it.
pub(super) struct FakeRegistry {
    pub(crate) root_module: Option<ModuleId>,
    pub(crate) module_ids: Vec<ModuleId>,
    pub(crate) module_names: HashMap<ModuleId, ModuleName>,
    pub(crate) name_to_ids: HashMap<ModuleName, Vec<ModuleId>>,
    pub(crate) module_keys: HashMap<ModuleId, ModuleKey>,
    pub(crate) senders: HashMap<ModuleId, WatchLayoutSender>,
    pub(crate) resolve_site: SiteResolver,
    pub(crate) parent_of: ParentLookup,
    pub(crate) spawn_module_calls: Arc<AtomicUsize>,
    pub(crate) spawn_requests: Arc<Mutex<Vec<LazySite>>>,
    pub(crate) spawned_id: Option<ModuleId>,
    active_signals: HashSet<SignalKind>,
}

impl Default for FakeRegistry {
    fn default() -> Self {
        Self {
            root_module: None,
            module_ids: Vec::new(),
            module_names: HashMap::new(),
            name_to_ids: HashMap::new(),
            module_keys: HashMap::new(),
            senders: HashMap::new(),
            resolve_site: Box::new(|_, _| None),
            parent_of: Box::new(|_| None),
            spawn_module_calls: Arc::new(AtomicUsize::new(0)),
            spawn_requests: Arc::new(Mutex::new(Vec::new())),
            spawned_id: None,
            active_signals: HashSet::new(),
        }
    }
}

#[async_trait]
impl
    ModuleRegistryPort<
        TinySkiaCanvasFactory,
        Sender<LayoutEvent>,
        Sender<DisplayCommand>,
        Sender<UiCommand>,
        WatchLayoutSender,
    > for FakeRegistry
{
    fn load(&mut self, _config: &Config) -> Result<(), RegistryLoadError> {
        Ok(())
    }

    fn spawn_all(&mut self, _deps: &Deps) -> HashMap<ModuleId, WatchLayoutSender> {
        std::mem::take(&mut self.senders)
    }

    fn reload_module(
        &mut self,
        _name: &ModuleName,
        _setup: Setup<'_>,
    ) -> Result<HashMap<ModuleId, WatchLayoutSender>, RegistryLoadError> {
        Ok(HashMap::new())
    }

    fn root_module(&self) -> Option<ModuleId> {
        self.root_module
    }

    fn module_ids(&self) -> &[ModuleId] {
        &self.module_ids
    }

    fn module_names(&self) -> HashMap<ModuleId, ModuleName> {
        self.module_names.clone()
    }

    fn name_to_ids(&self) -> &HashMap<ModuleName, Vec<ModuleId>> {
        &self.name_to_ids
    }

    fn module_keys(&self) -> HashMap<ModuleId, ModuleKey> {
        self.module_keys.clone()
    }

    fn resolve_site(&self, parent: Option<ModuleId>, key: &ModuleKey) -> Option<ModuleId> {
        (self.resolve_site)(parent, key)
    }

    fn parent_of(&self, id: ModuleId) -> Option<ModuleId> {
        (self.parent_of)(id)
    }

    fn spawn_module(
        &mut self,
        site: LazySite,
        _setup: Setup<'_>,
    ) -> Result<SpawnedModule<WatchLayoutSender>, RegistryLoadError> {
        self.spawn_module_calls.fetch_add(1, Ordering::SeqCst);
        let error = RegistryLoadError::ModuleNotFound(site.key().name().clone());
        if let Ok(mut requests) = self.spawn_requests.lock() {
            requests.push(site);
        }
        let id = self.spawned_id.ok_or(error)?;
        self.module_ids.push(id);
        let (layout_tx, _layout_rx) = tokio::sync::watch::channel(HashMap::new());
        Ok(SpawnedModule::new(id, WatchLayoutSender::new(layout_tx)))
    }

    fn modules_using_style(&self, _sheet: &StyleSheetName) -> Vec<ModuleName> {
        Vec::new()
    }

    fn clear(&mut self) {}

    async fn register_dbus_subscriptions(&self, _dbus: &mut DbusSubscriptionManager) {}

    fn active_signal_subscriptions(&self) -> &HashSet<SignalKind> {
        &self.active_signals
    }
}

/// What one `wait_for_events` call resolves to.
type WaitOutcome = Pin<Box<dyn Future<Output = Result<(), DisplayServerError>> + Send>>;

/// A display server whose only scripted behaviour is `wait_for_events`, which
/// is what ends `CrankyApp::run`; every other request succeeds, and
/// `render_all` calls are counted.
pub(super) struct FakeDisplayServer {
    pub(crate) wait_for_events: Box<dyn FnMut() -> WaitOutcome + Send + Sync>,
    /// How many times `render_all` was called.
    pub(crate) renders: Arc<AtomicUsize>,
}

impl FakeDisplayServer {
    /// A server whose first `wait_for_events` fails with `reason`.
    pub(crate) fn failing(reason: &'static str) -> Self {
        Self {
            wait_for_events: Box::new(move || {
                Box::pin(std::future::ready(Err(DisplayServerError::Internal(
                    reason.to_string(),
                ))))
            }),
            renders: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl DisplayServerPort for FakeDisplayServer {
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError> {
        (self.wait_for_events)().await
    }

    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), DisplayServerError> {
        Ok(())
    }

    fn render_all<Snd: LayoutSender + 'static>(
        &mut self,
        _read_model: &AppReadModel,
        _layout_senders: &HashMap<ModuleId, Snd>,
    ) -> Result<(), DisplayServerError> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn show_floating_surface(&mut self, _floating: ShowFloating) -> Result<(), DisplayServerError> {
        Ok(())
    }

    fn hide_floating_surface(&mut self, _kind: &FloatingKind) -> Result<(), DisplayServerError> {
        Ok(())
    }
}

/// A systray host that accepts every request.
pub(super) struct FakeSni;

#[async_trait]
impl SniPort for FakeSni {
    async fn start(&mut self) -> Result<(), SniPortError> {
        Ok(())
    }

    async fn trigger_action(
        &self,
        _id: &SystrayId,
        _call: &SystrayActionCall,
    ) -> Result<(), SniPortError> {
        Ok(())
    }
}
