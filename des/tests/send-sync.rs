fn require_send<T: Send>(_: &T) {}
fn require_sync<T: Sync>(_: &T) {}

macro_rules! require {
    ($t:ty: Send ) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        const _: fn() = || {
            let f: $t = todo!();
            require_send(&f);
        };
    };
    ($t:ty: Send + Sync) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        const _: fn() = || {
            let f: $t = todo!();
            require_send(&f);
            require_sync(&f);
        };
    };
    ($t:ty: Sync) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        const _: fn() = || {
            let f: $t = todo!();
            require_sync(&f);
        };
    };
}

require!(des::Error: Send);
require!(des::Failure: Send);
require!(des::ObjectPath: Send + Sync);

require!(des::gate::Gate: Send + Sync);
require!(des::gate::GateCluster: Send + Sync);

require!(des::message::Message: Send);
require!(des::message::Body: Send);
require!(des::message::Extensions: Send);
require!(des::message::Header: Send + Sync);
require!(des::message::MessageId: Send + Sync);
require!(des::message::MessageKind: Send + Sync);

require!(des::channel::ChannelRef: Send + Sync);
require!(des::channel::DatarateChannel: Send + Sync);
require!(des::channel::DatarateChannelMetrics: Send + Sync);
require!(des::channel::DelayChannel: Send + Sync);
require!(des::channel::ChannelDropBehaviour: Send + Sync);

require!(des::module::ModuleContext: Send + Sync);
require!(des::module::ModuleRef: Send + Sync);
require!(des::module::RawProp: Send + Sync);
require!(des::module::Signal: Send);
require!(des::module::UnwindBehaviour: Send + Sync);
require!(des::module::SignalCode: Send + Sync);

require!(des::time::SimTime: Send + Sync);
require!(des::time::Duration: Send + Sync);
require!(des::time::MissedTickBehavior: Send + Sync);
require!(des::time::Sleep: Send + Sync);
require!(des::time::Timeout<()>: Send + Sync);
require!(des::time::Interval: Send + Sync);

require!(des::runtime::NetEvents: Send);
require!(des::runtime::Globals: Send + Sync);

#[test]
fn passed() {}
