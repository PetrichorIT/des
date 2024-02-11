#![allow(missing_docs, missing_debug_implementations, unreachable_pub)]

use std::{future::Future, io, panic};

use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    net::module::ModuleContext,
    prelude::{
        AsyncModule, Channel, ChannelMetrics, Message, Module, ModuleRef,
        ObjectPath,
    },
};

use super::NetworkApplication;

/// A builder for an async network runtime.
///
/// A builder can be used to quickly create a simple async simulation
/// without having to define modules manualy. It provides a strong
/// abstraction over the underyling realities of a `NetworkApplication`
///
#[derive(Debug)]
pub struct AsyncBuilder {
    app: NetworkApplication<()>,
    default_cfg: NodeCfg,
    mapping: FxHashMap<String, NodeInfo>,
}

/// Configuration options for nodes of an `AsyncBuilder`
#[derive(Debug, Clone, Default)]
pub struct NodeCfg {
    /// A flag, whether to join the spawned future one the simulation
    /// terminates. Setting this flag to true is helpful on clients,
    /// to confirm that they completed their work, but servers
    /// may still run beyond that point, thus may never join.  
    pub join: bool,
}

#[derive(Debug)]
struct NodeInfo {
    #[allow(unused)]
    kind: NodeKind,
    node: ModuleRef,
    c: usize,
    connections: FxHashSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum NodeKind {
    BuilderAsync,
    ExternalT,
}

impl AsyncBuilder {
    /// Creates a fresh buider.
    ///
    /// A builder uses a `NetworkApplication<()>` to create the
    /// networking abstractions needed. Builders use a default configuration
    /// for all nodes if not specifed otherwise. Use
    /// [`set_default_cfg`](Self::set_default_cfg)
    /// to change the default configuration. Use the various `node*`
    /// functions to create async nodes. [`connect`](Self::connect)
    ///  can then be used to connect nodes
    ///
    #[must_use]
    pub fn new() -> AsyncBuilder {
        AsyncBuilder {
            app: NetworkApplication::new(()),
            mapping: FxHashMap::with_hasher(FxBuildHasher::default()),
            default_cfg: NodeCfg::default(),
        }
    }

    /// Sets the default configuration used for all nodes.
    ///
    /// Note that this operation does not change any prior configuration
    /// derived from the replaced default cfg, but will only change
    /// apply this change in the future.
    pub fn set_default_cfg(&mut self, cfg: NodeCfg) {
        self.default_cfg = cfg;
    }

    /// Finishes the building process and returns a fully configured
    /// network application, to be passed to a [`Runtime`](crate::runtime::Runtime).
    ///
    /// This operation never fails.
    #[must_use]
    pub fn build(self) -> NetworkApplication<()> {
        self.app
    }

    /// Adds a module to builder, that is not restricted by
    /// the limitations for other nodes.
    ///
    /// Note that this may lead to unexpected interactions
    /// since the builder creates modules incrementaly to connect
    /// nodes, while the input type T may have created its own gates.
    /// This may lead to collisions.
    ///
    /// # Panics
    ///
    /// Panics if the provided parent does not exist.
    pub fn external<T: Module>(&mut self, name: &str, parent: Option<&str>) {
        let ctx = if let Some(parent) = parent {
            let parent = self
                .mapping
                .get(parent)
                .expect("no node found under parents name");

            ModuleContext::child_of(name, parent.node.clone())
        } else {
            ModuleContext::standalone(ObjectPath::from(name))
        };

        ctx.activate();

        let state = T::new().to_processing_chain();
        ctx.upgrade_dummy(state);

        self.mapping.insert(
            ctx.path.as_str().to_string(),
            NodeInfo {
                node: ctx.clone(),
                c: 0,
                connections: FxHashSet::with_hasher(FxBuildHasher::default()),
                kind: NodeKind::ExternalT,
            },
        );
        self.app.register_module(ctx);
    }

    /// Adds a normal network node with the given name.
    ///
    /// The created node will have no parent and use
    /// the builders default configuration.
    ///
    /// See [`node_with_cfg_and_parent`](Self::node_with_cfg_and_parent) for more information.
    pub fn node<F, Fut>(&mut self, name: impl AsRef<str>, software: F)
    where
        F: Fn(Receiver<Message>) -> Fut + Send + 'static,
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        self.node_with_cfg_and_parent(name.as_ref(), None, self.default_cfg.clone(), software);
    }

    /// Adds a normal network node with the given name and cfg.
    ///
    /// The created node will have no parent,
    /// but will use a custom provided config.
    ///
    /// See [`node_with_cfg_and_parent`](Self::node_with_cfg_and_parent) for more information.
    pub fn node_with_cfg<F, Fut>(&mut self, name: impl AsRef<str>, cfg: NodeCfg, software: F)
    where
        F: Fn(Receiver<Message>) -> Fut + Send + 'static,
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        self.node_with_cfg_and_parent(name.as_ref(), None, cfg, software);
    }

    /// Adds a normal network node with the given name and parent.
    ///
    /// The node will be the child of the given parent node
    /// and use the builders default configuration.
    ///
    /// See [`node_with_cfg_and_parent`](Self::node_with_cfg_and_parent) for more information.
    pub fn node_with_parent<F, Fut>(
        &mut self,
        name: impl AsRef<str>,
        parent: impl AsRef<str>,
        software: F,
    ) where
        F: Fn(Receiver<Message>) -> Fut + Send + 'static,
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        self.node_with_cfg_and_parent(
            name.as_ref(),
            Some(parent.as_ref()),
            self.default_cfg.clone(),
            software,
        );
    }

    /// Adds a normal network node to the simulation.
    ///
    /// The node will be uniquely identifed by its name
    /// in combination with its parent, if existent. The node
    /// will be created with the object path `<parent>.<name>`
    /// and will follow the associated unload order accordingly.
    ///
    /// The cfg parameter will be used to define internal workings
    /// of the node, not exposed to the software.
    ///
    /// The software is a creation function that returns a
    /// future, when provided with a receiver of messages.
    /// The returned future is to be considered the
    /// `#[tokio::main]` body, that defines the nodes runtime
    /// behaviour. The provided channel receiver represents
    /// an incoming stream of messages. The software
    /// is a creation fn to enable the simulation to
    /// recreate the main task, when the module was shut down
    /// and restarted.
    ///
    /// # Panics
    ///
    /// Panics if the provided parent does not exist.
    pub fn node_with_cfg_and_parent<F, Fut>(
        &mut self,
        name: &str,
        parent: Option<&str>,
        cfg: NodeCfg,
        software: F,
    ) where
        F: Fn(Receiver<Message>) -> Fut + Send + 'static,
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        let ctx = if let Some(parent) = parent {
            let parent = self
                .mapping
                .get(parent)
                .expect("no node found under parents name");

            ModuleContext::child_of(name, parent.node.clone())
        } else {
            ModuleContext::standalone(ObjectPath::from(name))
        };

        ctx.activate();

        let (tx, rx) = channel(8);
        let state = FutModule {
            software,
            cfg,
            handle: None,
            tx,
            rx: Some(rx),
        }.to_processing_chain();
        ctx.upgrade_dummy(state);

        self.mapping.insert(
            ctx.path.as_str().to_string(),
            NodeInfo {
                node: ctx.clone(),
                c: 0,
                connections: FxHashSet::with_hasher(FxBuildHasher::default()),
                kind: NodeKind::BuilderAsync,
            },
        );
        self.app.register_module(ctx);
    }

    /// Connects two nodes with a duplex channel.
    ///
    /// Nodes are identifed by their names, and connected by a set of
    /// nondelayed gate-chains. To facilitate this operation,
    /// the builder creates two new gates per node `in[k]` and `out[k]`,
    /// where k is a incrementing index based on the number of connections per
    /// node.
    ///
    /// # Panics
    ///
    /// This operation panic if the provided node names do
    /// not map to existing nodes.
    pub fn connect(&mut self, lhs: impl AsRef<str>, rhs: impl AsRef<str>) {
        self.connect_with(lhs, rhs, None);
    }

    /// Connects two nodes with a duplex channel.
    ///
    /// Nodes are identifed by their names, and connected by a set of
    /// delayed gate-chains. To facilitate this operation,
    /// the builder creates two new gates per node `in[k]` and `out[k]`,
    /// where k is a incrementing index based on the number of connections per
    /// node.
    ///
    /// The provided channel metrics are used to create
    /// a channel if nessecary, to facilitate delayed message forwarding
    ///
    ///  # Panics
    ///
    /// This operation panic if the provided node names do
    /// not map to existing nodes.
    pub fn connect_with(
        &mut self,
        lhs: impl AsRef<str>,
        rhs: impl AsRef<str>,
        chan: Option<ChannelMetrics>,
    ) {
        let lhs = lhs.as_ref();
        let rhs = rhs.as_ref();

        let lhs_info = self.mapping.get(lhs).expect("failed to resolve");
        let rhs_info = self.mapping.get(rhs).expect("failed to resolve");

        #[cfg(feature = "tracing")]
        if lhs_info.kind == NodeKind::ExternalT || rhs_info.kind == NodeKind::ExternalT {
            tracing::warn!(
                "using AsyncBuilder::connect with external modules may create unexpected gates"
            )
        }

        assert!(lhs_info.connections.get(rhs).is_none());
        assert!(rhs_info.connections.get(lhs).is_none());

        let chan = chan.map(|chan| {
            Channel::new(
                rhs_info.node.path.appended_channel(format!("{lhs}-{rhs}")),
                chan,
            )
        });

        let lhs_gate = lhs_info.node.create_raw_gate("port", lhs_info.c + 1, lhs_info.c);
        let rhs_gate = rhs_info.node.create_raw_gate("port", rhs_info.c + 1, rhs_info.c);

        self.mapping.get_mut(lhs).unwrap().c += 1;
        self.mapping.get_mut(rhs).unwrap().c += 1;


        lhs_gate.connect(rhs_gate, chan);
    }
}

impl Default for AsyncBuilder {
    fn default() -> Self {
        AsyncBuilder::new()
    }
}

struct FutModule<F, Fut>
where
    F: Fn(Receiver<Message>) -> Fut,
    Fut: Future<Output = io::Result<()>>,
{
    software: F,
    cfg: NodeCfg,
    handle: Option<JoinHandle<io::Result<()>>>,
    tx: Sender<Message>,
    rx: Option<Receiver<Message>>,
}

impl<F, Fut> AsyncModule for FutModule<F, Fut>
where
    F: Fn(Receiver<Message>) -> Fut + Send,
    Fut: Future<Output = io::Result<()>> + Send + 'static,
{
    fn new() -> Self {
        unimplemented!("FutModule should not be initalized using Module::new")
    }

    fn reset(&mut self) {}

    async fn at_sim_start(&mut self, _: usize) {
        let rx = self.rx.take().unwrap_or_else(|| {
            let (tx, rx) = channel(8);
            self.tx = tx;
            rx
        });

        let f = &self.software;
        let fut = f(rx);
        self.handle = Some(tokio::spawn(fut));
    }

    async fn handle_message(&mut self, msg: Message) {
        self.tx.send(msg).await.expect("Failed to send to channel");
    }

    async fn at_sim_end(&mut self) {
        if self.cfg.join {
            let handle = self
                .handle
                .take()
                .expect("not join handle found, thats wierd");

            let finished = match handle.await {
                Ok(fin) => fin,
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("builder node paniced: {e}");
                    panic::resume_unwind(e.into_panic());
                }
            };

            if let Err(e) = finished {
                #[cfg(feature = "tracing")]
                tracing::error!("builder node failed: {e}");
                panic!("{e}")
            }
        }
    }
}
