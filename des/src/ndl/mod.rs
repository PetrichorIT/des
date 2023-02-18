//! NDL intergration.
use des_ndl::{
    error::RootResult,
    ir::{self, ConnectionEndpoint},
    Context,
};
use std::{path::Path, str::FromStr, sync::Arc};

mod registry;

use crate::{
    net::module::ModuleContext,
    prelude::{
        Channel, ChannelMetrics, EventLifecycle, GateServiceType, ModuleRef, NetworkRuntime,
        ObjectPath,
    },
    time::Duration,
};

pub use self::registry::Registry;

/// A application for NDL intergration.
#[derive(Debug)]
pub struct NdlApplication {
    tree: Arc<ir::Module>,
    registry: Registry,
}

impl NdlApplication {
    /// Create a new NdlApplication.
    pub fn new(path: impl AsRef<Path>, registry: Registry) -> RootResult<NdlApplication> {
        let ctx = Context::load(path)?;
        Ok(NdlApplication {
            tree: ctx.entry.unwrap(),
            registry,
        })
    }
}

impl EventLifecycle<NetworkRuntime<Self>> for NdlApplication {
    fn at_sim_start(rt: &mut crate::prelude::Runtime<NetworkRuntime<Self>>) {
        log::info!("building ndl");
        Self::build_at(
            rt,
            rt.app.inner.tree.clone(),
            ObjectPath::root_module("root"),
            None,
        );
    }

    fn at_sim_end(_rt: &mut crate::prelude::Runtime<NetworkRuntime<Self>>) {
        log::info!("closing ndl");
    }
}

impl NdlApplication {
    fn build_at(
        rt: &mut crate::prelude::Runtime<NetworkRuntime<Self>>,
        ir: Arc<ir::Module>,
        path: ObjectPath,
        parent: Option<ModuleRef>,
    ) -> ModuleRef {
        let ident = path.name();
        let ty = &ir.ident.raw;

        let ctx = if let Some(parent) = parent {
            ModuleContext::child_of(ident, parent)
        } else {
            ModuleContext::standalone(path.clone())
        };

        for gate in &ir.gates {
            let _ = ctx.create_gate_cluster(
                &gate.ident.raw,
                gate.cluster.as_size(),
                gate.service_typ.into(),
            );
        }

        for submod in &ir.submodules {
            let sty = submod.typ.as_module_arc().unwrap();
            let new_path = ObjectPath::from_str(&format!("{}.{}", path.path(), submod.ident.raw))
                .expect("failed to create submod path");

            Self::build_at(rt, sty, new_path, Some(ctx.clone()));
            // ctx.add_child(&submod.ident.raw, sub);
        }

        for con in &ir.connections {
            let from = match &con.from {
                ConnectionEndpoint::Local(gate, pos) => ctx.gate(&gate.raw, pos.as_index()),
                ConnectionEndpoint::Nonlocal(submod, pos, gate) => {
                    let c = ctx.child(&format!("{}{}", submod.raw, pos)).unwrap();

                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .unwrap();

            let to = match &con.to {
                ConnectionEndpoint::Local(gate, pos) => ctx.gate(&gate.raw, pos.as_index()),
                ConnectionEndpoint::Nonlocal(submod, pos, gate) => {
                    let c = ctx.child(&format!("{}{}", submod.raw, pos)).unwrap();
                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .unwrap();

            from.set_next_gate(to.clone());

            if let Some(delay) = &con.delay {
                let link = delay.as_link_arc().unwrap();
                let channel = Channel::new(
                    ObjectPath::channel_with("ch", &ctx.path()),
                    ChannelMetrics::from(&*link),
                );

                from.set_channel(channel);
            }
        }

        ctx.activate();
        log_scope!(ctx.path.path());
        let state = rt.app.inner.registry.get(ty).unwrap()();
        ctx.upgrade_dummy(state);

        // TODO: is this still usefull or should we just save the entry point?
        rt.app.create_module(ctx.clone());

        ctx
    }
}

impl From<ir::GateServiceType> for GateServiceType {
    fn from(value: ir::GateServiceType) -> Self {
        match value {
            ir::GateServiceType::Input => GateServiceType::Input,
            ir::GateServiceType::Output => GateServiceType::Output,
            ir::GateServiceType::None => GateServiceType::Undefined,
        }
    }
}

impl From<&ir::Link> for ChannelMetrics {
    fn from(value: &ir::Link) -> Self {
        ChannelMetrics {
            bitrate: value.bitrate as usize,
            jitter: Duration::from_secs_f64(value.jitter),
            latency: Duration::from_secs_f64(value.latency),
            cost: value
                .fields
                .get("cost")
                .map(|l| l.as_float_casted())
                .unwrap_or(0.0),
            queuesize: value
                .fields
                .get("queuesize")
                .map(|l| l.as_integer_casted())
                .unwrap_or(0) as usize,
        }
    }
}
