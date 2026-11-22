//! A compiled Rhai script borrowed together with the engine and scope it runs in.
use rhai::{AST, Engine, Scope};

/// One module's compiled script with the engine and the scope it runs in, as each call into the
/// script needs all three; the Rhai counterpart of the `Lua` state the Lua adapter passes.
pub(super) struct RhaiScript<'a> {
    engine: &'a Engine,
    scope: &'a mut Scope<'static>,
    ast: &'a AST,
}

impl<'a> RhaiScript<'a> {
    pub(super) const fn new(
        engine: &'a Engine,
        scope: &'a mut Scope<'static>,
        ast: &'a AST,
    ) -> Self {
        Self { engine, scope, ast }
    }

    pub(super) const fn engine(&self) -> &Engine {
        self.engine
    }

    pub(super) const fn scope_mut(&mut self) -> &mut Scope<'static> {
        self.scope
    }

    /// The engine, scope and script together, as `Engine::call_fn` takes them.
    pub(super) const fn parts(&mut self) -> (&Engine, &mut Scope<'static>, &AST) {
        (self.engine, self.scope, self.ast)
    }
}
