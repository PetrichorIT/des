use std::{fmt::Debug, sync::Arc};

use crate::{
    net::{
        IntoModuleTree, ObjectPath, SimBuilder, globals,
        module::ModuleContext,
        processing::{ModuleImpl, ProcessingStack},
        runtime::{AtSimStartEvent, NetEvents},
        schedule_event,
    },
    prelude::{GateRef, Module, ModuleRef},
    time::SimTime,
};

/// A scoped spawner, used to populate a subtree of the total module-tree with nodes.
///
/// Using this spawner, nodes of the subtree with a root at `scope()` can be
/// created. These nodes are created through the spawner's `node()` method and
/// the spawners `root()` method. Spawners are used either in the implmentation
/// or the trait `IntoModuleTree` or at runtime using the `spawner` API of
/// the current module context.
///
/// # Examples - Module Blocks
///
/// When implementing the trait `IntoModuleTree` manually, a spawner at the desired position in the module
/// tree is provided. The following actions may be performed:
///
/// - create a node at the root of the subtree using `Spawner::root` or `Spawner::root_with_context`
/// - create a node under the root of the subtree using `Spawner::node` (which recursively uses another spawner)
/// - create gates & gate-connections between existing nodes in the subtree
///
/// Note that the usual rules about node creation remain, requiring the existence of a root node before any node under root
/// might be created. At runtime however the root of the subtree is already populated.
///
/// ```
/// # use des::prelude::*;
/// # use des::net::{handlers::{ModuleFn, HandlerFn}, IntoModuleTree};
/// struct LAN {}
/// impl IntoModuleTree for LAN {
///     type Ret = ();
///     fn build<A>(self, mut sim: Spawner<'_, A>) {
///         sim.root(HandlerFn::new(|_| {}));
///         let gates = sim.gates("", "port", 5);
///         for i in 0..5 {
///             let host = format!("host-{i}");
///             sim.node(&host, ModuleFn::new(
///                 /* ... */
///                 # || 123, |_, _| {}
///             ));
///             let gate = sim.gate(&host, "port");
///             gate.connect(gates[i].clone());
///         }
///     }
/// }
///
/// let mut sim = Sim::new(());
/// sim.node("google", LAN {});
/// sim.node("microsoft", LAN {});
/// sim.node("aws", HandlerFn::new(|_| {}));
/// sim.node("aws.us-east", LAN {});
///
/// let _ = Builder::new().build(sim.freeze()).run();
/// ```
pub struct Spawner<'a, A> {
    scope: ObjectPath,
    inner: InnerSpawner<'a, A>,
}

/// The kind of spawner.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SpawnerKind {
    /// A spawner that is created at build time. It will add nodes directly to the `SimBuilder` and schedule them all for simultaneous startup.
    BuildtimeSpawner,
    /// A spawner that is created at runtime in relation to a already existing node. It will add nodes
    /// to the module tree and schedule individual startup events for each node.
    RuntimeSpawner,
}

enum InnerSpawner<'a, A> {
    AtBuildtime {
        base: &'a mut SimBuilder<A>,
    },
    AtRuntime {
        stack: Arc<dyn Fn() -> ProcessingStack>,
    },
}

impl<'a, A> InnerSpawner<'a, A> {
    fn get(&self, path: &ObjectPath) -> Option<ModuleRef> {
        match self {
            Self::AtBuildtime { base } => base.get(path),
            Self::AtRuntime { .. } => globals().get(path),
        }
    }

    /// global path, impl -> constructed but not stared
    fn create_node(
        &mut self,
        path: ObjectPath,
        module: impl FnOnce() -> Box<dyn Module>,
    ) -> ModuleRef {
        match self {
            Self::AtBuildtime { base } => crate_node_at_buildtime(path, module, base),
            Self::AtRuntime { stack, .. } => create_node_at_runtime(&path, module, &**stack),
        }
    }

    fn create_gates(&mut self, path: &ObjectPath, gate: &str, size: usize) -> Vec<GateRef> {
        let Some(module) = self.get(path) else {
            panic!("cannot create gate '{path}.{gate}', because node '{path}' does not exist")
        };

        let mut gates = Vec::new();
        for k in 0..size {
            if let Some(gate) = module.gate((gate, k)) {
                gates.push(gate);
            } else {
                break;
            }
        }

        match gates.len() {
            0 => module.create_gate_cluster(gate, size),
            s if s == size => gates,
            _ => panic!("cannot create gate cluster from partial gate cluster"),
        }
    }

    // FIXME: invariants make things annoying, but we could just
    // make a subscope(_, || <do something> bound) that should work without being annoying maybe

    fn as_ref_spawner<'b>(&'b mut self) -> InnerSpawner<'b, A>
    where
        'a: 'b,
    {
        match self {
            Self::AtBuildtime { base } => InnerSpawner::AtBuildtime { base: *base },
            Self::AtRuntime { stack } => InnerSpawner::AtRuntime {
                stack: stack.clone(),
            },
        }
    }

    fn kind(&self) -> SpawnerKind {
        match self {
            Self::AtBuildtime { .. } => SpawnerKind::BuildtimeSpawner,
            Self::AtRuntime { .. } => SpawnerKind::RuntimeSpawner,
        }
    }
}

fn crate_node_at_buildtime<A>(
    path: ObjectPath,
    module: impl FnOnce() -> Box<dyn Module>,
    base: &mut SimBuilder<A>,
) -> ModuleRef {
    // Check dup
    assert!(
        base.get(&path).is_none(),
        "cannot create node '{path}', node allready exists"
    );
    // Check node path location
    let ctx = if let Some(parent) = path.nonzero_parent() {
        // (a) Check that the parent exists
        let Some(parent) = base.get(&parent) else {
            panic!(
                "cannot create node '{path}', since parent node '{parent}' is required, but does not exist"
            );
        };

        ModuleContext::new_child_of(path.name(), parent)
    } else if let Some(zero_parent) = base.get(&ObjectPath::from("")) {
        ModuleContext::new_child_of(path.name(), zero_parent)
    } else {
        ModuleContext::new_standalone(path)
    };
    // read in Props
    let path_parts = ctx.path.as_str().split('.').collect::<Vec<_>>();
    base.globals
        .capture_for(&path_parts, &mut ctx.props.write());

    let _ = ctx.activate();
    let pe = {
        let module = module();
        let stack = (base.stack)();
        ModuleImpl::new(module.stack(stack), module)
    };
    ctx.upgrade_dummy(pe);
    ctx.deactivate();
    base.with_modules_mut(|mods| mods.add(ctx.clone()));
    ctx
}

fn create_node_at_runtime(
    path: &ObjectPath,
    module_creator: impl FnOnce() -> Box<dyn Module>,
    stack: &dyn Fn() -> ProcessingStack,
) -> ModuleRef {
    assert!(
        globals().get(path).is_none(),
        "cannot create node '{path}' that already exists"
    );

    let parent_path = path.nonzero_parent().expect("must have a parent");
    let parent = globals().get(&parent_path).expect("must have a parent");
    let ctx = ModuleContext::new_child_of(path.name(), parent);

    // TODO: CFGs are missing here
    // A) store in globals & pull
    // B) provide custom CFGs API to local spawners ?

    let path_parts = ctx.path.as_str().split('.').collect::<Vec<_>>();
    globals().capture_for(&path_parts, &mut ctx.props.write());

    let prev = ctx.activate();
    let pe = {
        let module = module_creator();
        let stack = stack();
        ModuleImpl::new(module.stack(stack), module)
    };
    ctx.upgrade_dummy(pe);
    ctx.deactivate();

    globals().add_module(ctx.clone());

    if let Some(prev) = prev {
        let _ = prev.me().activate();
    }

    schedule_event(
        NetEvents::AtSimStartEvent(AtSimStartEvent {
            modules: vec![ctx.clone()],
        }),
        SimTime::now(),
    );

    ctx
}

impl<'a, A> Spawner<'a, A> {
    // FIXME: the +'static bound may be relaxed to 'a if we are not using a box
    pub(crate) fn new_at_runtime(
        ctx: &ModuleContext,
        stack: impl Fn() -> ProcessingStack + 'static,
    ) -> Self {
        Self {
            scope: ctx.path(),
            inner: InnerSpawner::AtRuntime {
                stack: Arc::new(stack),
            },
        }
    }

    pub(crate) fn new_at_buildtime(scope: ObjectPath, base: &'a mut SimBuilder<A>) -> Self {
        Self {
            scope,
            inner: InnerSpawner::AtBuildtime { base },
        }
    }

    pub(crate) fn subscope(&mut self, path: impl Into<ObjectPath>) -> Spawner<'_, A> {
        Spawner {
            scope: self.scope.appended(path.into()),
            inner: self.inner.as_ref_spawner(),
        }
    }

    /// The kind of spawner that `self` is.
    ///
    /// There are two options:
    /// - `SpawnerKind::RuntimeSpawner`: The spawner is created at runtime.
    ///   Nodes created through this spawner will start as soon as possible in their own startup cycle.
    /// - `SpawnerKind::BuildtimeSpawner`: The spawner is created at buildtime.
    ///   Nodes created through this spawner will start all at once when `at_sim_start` is called on the application object.
    #[must_use]
    pub fn kind(&self) -> SpawnerKind {
        self.inner.kind()
    }

    /// The scope of the spawner.
    ///
    /// A spawner may only create nodes at or beneath this object path.
    #[must_use]
    pub fn scope(&self) -> &ObjectPath {
        &self.scope
    }

    /// Retrieves a node relative to this spawner's scope.
    ///
    /// Nodes outside the spawners scope are not accessible, to reduce the
    /// risk of unintended interactions.
    pub fn get(&self, path: impl AsRef<str>) -> Option<ModuleRef> {
        self.inner.get(&self.scope.appended(path))
    }

    /// Creates or retrieves a gate at the specified module.
    ///
    /// See [`SimBuilder::gate`] for more information.
    pub fn gate(&mut self, path: impl Into<ObjectPath>, gate: &str) -> GateRef {
        // FIXME: this alloc of vec is unnecessary, but maybe the compiler figures that out
        // TODO: check that
        self.inner
            .create_gates(&self.scope.appended(path.into()), gate, 1)
            .remove(0)
    }

    /// Creates or retrieves a gate cluster at the specified module.
    ///
    /// See [`SimBuilder::gates`] for more information.
    pub fn gates(&mut self, path: impl Into<ObjectPath>, gate: &str, size: usize) -> Vec<GateRef> {
        self.inner
            .create_gates(&self.scope.appended(path.into()), gate, size)
    }

    /// Create a node at the root of the subtree, using the
    /// provided module as the handler for the node.
    pub fn root(&mut self, handler: impl Module) -> ModuleRef {
        self.inner
            .create_node(self.scope.clone(), || Box::new(handler))
    }

    /// Creates a node at the root of the subtree, using the module
    /// implementation provided by the closure as the handler for the node.
    ///
    /// The closure is executed within the `node-context` of the node
    /// that is being created. Accordingly most APIs provided by the
    /// module context can be accessed in the constructor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::{prelude::*, net::{IntoModuleTree, module::Prop}};
    /// struct PropInStruct {
    ///     prop: Prop<String>, // cannot be initialized outside of node-context, since it contains a prop
    /// }
    /// impl Module for PropInStruct {}
    ///
    /// struct PropInStructBuilder;
    /// impl IntoModuleTree for PropInStructBuilder {
    ///     type Ret = ();
    ///     fn build<A>(self, mut ctx: Spawner<'_, A>) -> Self::Ret {
    ///         ctx.root_with_context(|| {
    ///             // can use node-context of the node that is being created
    ///             let prop = current().prop::<String>("key").expect("prop");
    ///             Box::new(PropInStruct { prop })
    ///         });
    ///     }
    /// }
    /// ```
    pub fn root_with_context(
        &mut self,
        handler_creator: impl FnOnce() -> Box<dyn Module>,
    ) -> ModuleRef {
        self.inner.create_node(self.scope.clone(), handler_creator)
    }

    /// Create a node under the root of the subtree, using a module block.
    ///
    /// See [`SimBuilder::node`] for general information about block creation.
    pub fn node<M: IntoModuleTree>(
        &mut self,
        path: impl Into<ObjectPath>,
        module_block: M,
    ) -> M::Ret {
        let scope = self.subscope(path);
        module_block.build(scope)
    }
}

impl<A> Debug for Spawner<'_, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Spawner")
            .field("scope", &self.scope())
            .field("kind", &self.kind())
            .finish()
    }
}
