use std::time::Duration;

use tokio::net::IOContext;
use tokio::sim::SimContext;

use super::Plugin;
use crate::net::message::{schedule_at, schedule_in, send, send_in, Message};
use crate::net::message::{
    TYP_IO_TICK, TYP_RESTART, TYP_TCP_CONNECT, TYP_TCP_CONNECT_TIMEOUT, TYP_TCP_PACKET,
    TYP_UDP_PACKET, TYP_WAKEUP,
};

macro_rules! as_option {
    ($e:expr) => {
        match $e {
            Ok(()) => None,
            Err(value) => Some(value),
        }
    };
}

/// Tokio-Intergration
#[derive(Debug)]
pub struct TokioNetPlugin {
    io: Option<IOContext>,
    prev: Option<IOContext>,
}

impl TokioNetPlugin {
    /// Creates a new TokioNetPlugin
    pub fn new() -> Self {
        Self {
            io: Some(IOContext::empty()),
            prev: None,
        }
    }
}

impl Plugin for TokioNetPlugin {
    fn capture_sim_start(&mut self) {
        self.capture(None);
    }

    fn capture_sim_end(&mut self) {
        self.capture(None);
    }

    fn capture(&mut self, msg: Option<Message>) -> Option<Message> {
        let io = self.io.take().expect("Plugin failure");
        self.prev = SimContext::with_current(|ctx| ctx.io.replace(io));

        let Some(msg) = msg else { return None };

        match msg.header().typ {
            TYP_WAKEUP => {
                log::trace!("Wakeup received");
                None
            }
            TYP_RESTART => {
                log::trace!("Module restart complete");
                None
            }
            TYP_IO_TICK => {
                log::trace!("IO tick received");
                IOContext::with_current(|ctx| ctx.io_tick());
                None
            }
            TYP_UDP_PACKET => {
                use tokio::sim::net::UdpMessage;
                let (msg, header) = msg.cast::<UdpMessage>();

                as_option!(IOContext::with_current(|ctx| {
                    ctx.process_udp(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }))
            }
            TYP_TCP_CONNECT => {
                use tokio::sim::net::TcpConnectMessage;
                let (msg, header) = msg.cast::<TcpConnectMessage>();

                as_option!(IOContext::with_current(|ctx| {
                    ctx.process_tcp_connect(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }))
            }
            TYP_TCP_CONNECT_TIMEOUT => {
                use tokio::sim::net::TcpConnectMessage;
                let (msg, header) = msg.cast::<TcpConnectMessage>();

                as_option!(IOContext::with_current(|ctx| {
                    ctx.process_tcp_connect_timeout(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }))
            }
            TYP_TCP_PACKET => {
                use tokio::sim::net::TcpMessage;
                let (msg, header) = msg.cast::<TcpMessage>();

                as_option!(IOContext::with_current(|ctx| {
                    ctx.process_tcp_packet(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }))
            }
            _ => Some(msg),
        }
    }

    fn defer_sim_start(&mut self) {
        self.defer();
    }

    fn defer_sim_end(&mut self) {
        self.defer();
    }

    fn defer(&mut self) {
        let intents = IOContext::with_current(|ctx| ctx.yield_intents());
        for intent in intents {
            use tokio::sim::net::IOIntent;
            match intent {
                IOIntent::UdpSendPacket(pkt) => {
                    log::info!("Sending captured UDP packet: {:?}", pkt);
                    if pkt.dest_addr.ip().is_loopback() {
                        schedule_in(
                            Message::new()
                                // .kind(RT_UDP)
                                .typ(TYP_UDP_PACKET)
                                .dest(pkt.dest_addr)
                                .content(pkt)
                                .build(),
                            Duration::from_nanos(10),
                        );
                    } else {
                        send(
                            Message::new()
                                // .kind(RT_UDP)
                                .typ(TYP_UDP_PACKET)
                                .dest(pkt.dest_addr)
                                .content(pkt)
                                .build(),
                            "out",
                        );
                    }
                }
                IOIntent::TcpConnect(pkt) => {
                    log::info!("Sending captured TCP connect: {:?}", pkt);
                    send(
                        Message::new()
                            // .kind(RT_TCP_CONNECT)
                            .typ(TYP_TCP_CONNECT)
                            .dest(pkt.dest())
                            .content(pkt)
                            .build(),
                        "out",
                    );
                }
                IOIntent::TcpSendPacket(pkt, delay) => {
                    log::info!("Sending captured TCP packet: {:?}", pkt);
                    send_in(
                        Message::new()
                            // .kind(RT_TCP_PACKET)
                            .typ(TYP_TCP_PACKET)
                            .dest(pkt.dest_addr)
                            .content(pkt)
                            .build(),
                        "out",
                        delay,
                    );
                }
                IOIntent::TcpConnectTimeout(pkt, timeout) => {
                    log::info!("Scheduling TCP Connect Timeout: {:?} in {:?}", pkt, timeout);
                    schedule_in(
                        Message::new()
                            // .kind(RT_TCP_CONNECT_TIMEOUT)
                            .typ(TYP_TCP_CONNECT_TIMEOUT)
                            .dest(pkt.src())
                            .content(pkt)
                            .build(),
                        timeout,
                    );
                }
                IOIntent::IoTick(wakeup_time) => {
                    log::info!("Scheduling IO Tick at {}", wakeup_time.as_millis());
                    schedule_at(Message::new().typ(TYP_IO_TICK).build(), wakeup_time);
                }
                _ => {
                    log::warn!("Unkown Intent");
                }
            }
        }

        // Remove IO Context
        let was_activated = self.io.is_none();
        if was_activated {
            self.io = SimContext::with_current(|ctx| {
                let ret = ctx.io.take();
                ctx.io = self.prev.take();
                ret
            });
        }
    }
}
