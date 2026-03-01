// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::time::Duration;

use tokio::{
    sync::{broadcast, mpsc},
    time::sleep,
};
use tracing::error;

use crate::events::{NetCommand, NetEvent};

#[derive(Debug)]
pub struct NetInterfaceHandle {
    tx: mpsc::Sender<NetCommand>,
    rx: broadcast::Receiver<NetEvent>,
}
impl NetInterfaceHandle {
    pub fn new(tx: mpsc::Sender<NetCommand>, rx: broadcast::Receiver<NetEvent>) -> Self {
        Self { tx, rx }
    }
}

pub trait NetInterface: Sized {
    fn tx(&self) -> mpsc::Sender<NetCommand>;
    fn rx(&self) -> broadcast::Receiver<NetEvent>;
    fn handle(&self) -> NetInterfaceHandle {
        NetInterfaceHandle::from(self)
    }
}

#[derive(Debug, Clone)]
pub struct NetInterfaceInvertedHandle {
    cmd_tx: broadcast::Sender<NetCommand>,
    tx: mpsc::Sender<NetCommand>,
    event_tx: broadcast::Sender<NetEvent>,
}

impl NetInterfaceHandle {
    pub fn from(interface: &impl NetInterface) -> Self {
        Self {
            tx: interface.tx(),
            rx: interface.rx(),
        }
    }
}
impl NetInterface for NetInterfaceHandle {
    fn rx(&self) -> broadcast::Receiver<NetEvent> {
        self.rx.resubscribe()
    }

    fn tx(&self) -> mpsc::Sender<NetCommand> {
        self.tx.clone()
    }
}
pub fn create_test_net_interface() -> (NetInterfaceHandle, NetInterfaceInvertedHandle) {
    let (m_cmd_tx, mut m_cmd_rx) = mpsc::channel::<NetCommand>(1000);
    let (b_evt_tx, _) = broadcast::channel(1000);
    let (b_cmd_tx, _) = broadcast::channel(1000);

    let tx = b_cmd_tx.clone();
    let startup_event_tx = b_evt_tx.clone();
    let keep_alive = b_cmd_tx.subscribe();

    tokio::spawn(async move {
        let _rx_guard = keep_alive;
        sleep(Duration::from_millis(100)).await;
        let _ = startup_event_tx.send(NetEvent::AllPeersDialed);
        while let Some(cmd) = m_cmd_rx.recv().await {
            let _ = tx.send(cmd);
        }
    });

    let handle = NetInterfaceHandle {
        tx: m_cmd_tx.clone(),
        rx: b_evt_tx.subscribe(),
    };

    let inverted = NetInterfaceInvertedHandle {
        tx: m_cmd_tx,
        cmd_tx: b_cmd_tx,
        event_tx: b_evt_tx,
    };

    (handle, inverted)
}

#[derive(Clone)]
pub struct TestNetInterface {
    m_cmd_tx: mpsc::Sender<NetCommand>,
    b_cmd_tx: broadcast::Sender<NetCommand>,
    b_evt_tx: broadcast::Sender<NetEvent>,
}

impl TestNetInterface {
    pub fn new() -> Self {
        let (m_cmd_tx, mut m_cmd_rx) = mpsc::channel::<NetCommand>(1000);
        let (b_evt_tx, _) = broadcast::channel(1000);
        let (b_cmd_tx, _) = broadcast::channel(1000);

        // Bridge mpsc commands to broadcast so the mock can subscribe
        let tx = b_cmd_tx.clone();
        let startup_event_tx = b_evt_tx.clone();
        tokio::spawn(async move {
            // Simulate dial-in delay like TestNetInterface
            sleep(Duration::from_millis(100)).await;
            let _ = startup_event_tx.send(NetEvent::AllPeersDialed);

            while let Some(cmd) = m_cmd_rx.recv().await {
                if let Err(e) = tx.send(cmd.clone()) {
                    error!("Error sending on channel. cmd={cmd:?} with error={e}");
                }
            }
            println!("***** ERROR CLOSING CHANNEL!!!! ****");
        });

        Self {
            m_cmd_tx,
            b_evt_tx,
            b_cmd_tx,
        }
    }
}

impl NetInterface for TestNetInterface {
    fn tx(&self) -> mpsc::Sender<NetCommand> {
        self.m_cmd_tx.clone()
    }

    fn rx(&self) -> broadcast::Receiver<NetEvent> {
        self.b_evt_tx.subscribe()
    }
}

impl NetInterfaceInverted for TestNetInterface {
    fn tx(&self) -> mpsc::Sender<NetCommand> {
        self.m_cmd_tx.clone()
    }
    fn cmd_tx(&self) -> broadcast::Sender<NetCommand> {
        self.b_cmd_tx.clone()
    }

    fn cmd_rx(&self) -> broadcast::Receiver<NetCommand> {
        self.b_cmd_tx.subscribe()
    }

    fn event_tx(&self) -> broadcast::Sender<NetEvent> {
        self.b_evt_tx.clone()
    }

    fn event_rx(&self) -> broadcast::Receiver<NetEvent> {
        self.b_evt_tx.subscribe()
    }
}

pub trait NetInterfaceInverted: Sized {
    fn tx(&self) -> mpsc::Sender<NetCommand>;
    fn event_tx(&self) -> broadcast::Sender<NetEvent>;
    fn event_rx(&self) -> broadcast::Receiver<NetEvent>;
    fn cmd_tx(&self) -> broadcast::Sender<NetCommand>;
    fn cmd_rx(&self) -> broadcast::Receiver<NetCommand>;

    fn into_handle_inverted(self) -> NetInterfaceInvertedHandle {
        NetInterfaceInvertedHandle {
            tx: self.tx(),
            event_tx: self.event_tx(),
            cmd_tx: self.cmd_tx(),
        }
    }
}

impl NetInterfaceInverted for NetInterfaceInvertedHandle {
    fn tx(&self) -> mpsc::Sender<NetCommand> {
        self.tx.clone()
    }

    fn cmd_rx(&self) -> broadcast::Receiver<NetCommand> {
        self.cmd_tx.subscribe()
    }
    fn event_tx(&self) -> broadcast::Sender<NetEvent> {
        self.event_tx.clone()
    }
    fn cmd_tx(&self) -> broadcast::Sender<NetCommand> {
        self.cmd_tx.clone()
    }
    fn event_rx(&self) -> broadcast::Receiver<NetEvent> {
        self.event_tx.subscribe()
    }
}
