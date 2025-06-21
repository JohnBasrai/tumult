use tumult::*;

use anyhow::{Context, Result};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::StdoutLock;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload
{
    Broadcast
    {
        message: usize,
    },
    BroadcastOk,
    Read,
    ReadOk
    {
        messages: HashSet<usize>,
    },
    Topology
    {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
    Gossip
    {
        seen: HashSet<usize>,
    },
}

enum InjectedPayload
{
    Gossip,
}

struct BroadcastNode
{
    node:         String,
    id:           usize,
    messages:     HashSet<usize>,
    known:        HashMap<String, HashSet<usize>>,
    neighborhood: Vec<String>,
}

impl Node<(), Payload, InjectedPayload> for BroadcastNode
{
    fn from_init(
        _state: (), init: Init, tx: Sender<Event<Payload, InjectedPayload>>,
    ) -> Result<Self>
    {
        thread::spawn(move || {
            loop
            {
                thread::sleep(Duration::from_millis(250));
                if tx.send(Event::Injected(InjectedPayload::Gossip)).is_err()
                {
                    break;
                }
            }
        });

        Ok(Self {
            node:         init.node_id,
            id:           1,
            messages:     HashSet::new(),
            known:        init
                .node_ids
                .into_iter()
                .map(|nid| (nid, HashSet::new()))
                .collect(),
            neighborhood: Vec::new(),
        })
    }

    fn step(
        &mut self, input: Event<Payload, InjectedPayload>, output: &mut StdoutLock,
    ) -> Result<()>
    {
        match input
        {
            Event::EOF =>
            {}

            Event::Injected(payload) => match payload
            {
                InjectedPayload::Gossip =>
                {
                    for n in &self.neighborhood
                    {
                        let known_to_n = &self.known[n];
                        let (already_known, mut notify_of): (HashSet<_>, HashSet<_>) = self
                            .messages
                            .iter()
                            .copied()
                            .partition(|m| known_to_n.contains(m));
                        let mut rng = rand::thread_rng();
                        let additional_cap = (10 * notify_of.len() / 100) as u32;
                        notify_of.extend(already_known.iter().filter(|_| {
                            rng.gen_ratio(
                                additional_cap.min(already_known.len() as u32),
                                already_known.len() as u32,
                            )
                        }));

                        Message {
                            src:  self.node.clone(),
                            dest: n.clone(),
                            body: Body {
                                msg_id:      None,
                                in_reply_to: None,
                                payload:     Payload::Gossip { seen: notify_of },
                            },
                        }
                        .send(&mut *output)
                        .with_context(|| format!("gossip to {n}"))?;
                    }
                }
            },

            Event::Message(input) =>
            {
                let mut reply = input.into_reply(Some(&mut self.id));
                match reply.body.payload
                {
                    Payload::Gossip { seen } =>
                    {
                        self.known
                            .get_mut(&reply.dest)
                            .expect("got gossip msg from unknown node:{&reply.dest}")
                            .extend(seen.iter().copied());
                        self.messages.extend(seen);
                    }
                    Payload::Broadcast { message } =>
                    {
                        self.messages.insert(message);
                        reply.body.payload = Payload::BroadcastOk;
                        reply
                            .send(&mut *output)
                            .context("Error:sending broadcast")?;
                    }
                    Payload::Read =>
                    {
                        reply.body.payload = Payload::ReadOk {
                            messages: self.messages.clone(),
                        };
                        reply.send(&mut *output).context("read failure")?;
                    }
                    Payload::Topology { mut topology } =>
                    {
                        self.neighborhood = topology
                            .remove(&self.node)
                            .unwrap_or_else(|| panic!("no topology for node {}", self.node));
                        reply.body.payload = Payload::TopologyOk;
                        reply
                            .send(&mut *output)
                            .context("reply to topology error")?;
                    }
                    Payload::BroadcastOk | Payload::ReadOk { .. } | Payload::TopologyOk =>
                    {}
                }
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> { main_loop::<_, BroadcastNode, _, _>(()) }
