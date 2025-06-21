use tumult::*;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{StdoutLock, Write};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload
{
    Echo
    {
        echo: String
    },
    EchoOk
    {
        echo: String
    },
}

struct EchoNode
{
    id: usize,
}

impl Node<(), Payload> for EchoNode
{
    fn from_init(_state: (), _init: Init, _tx: Sender<Event<Payload>>) -> Result<Self>
    {
        Ok(EchoNode { id: 1 })
    }

    fn step(&mut self, input: Event<Payload>, output: &mut StdoutLock) -> Result<()>
    {
        let Event::Message(input) = input
        else
        {
            panic!("got unexpected injected event: {input:#?}");
        };

        let mut reply = input.into_reply(Some(&mut self.id));
        match reply.body.payload
        {
            Payload::Echo { echo } =>
            {
                reply.body.payload = Payload::EchoOk { echo };
                serde_json::to_writer(&mut *output, &reply)
                    .context("serializing response to init")?;
                output.write_all(b"\n").context("write newline")?;
            }
            Payload::EchoOk { .. } =>
            {}
        }
        Ok(())
    }
}

fn main() -> Result<()> { main_loop::<_, EchoNode, _, _>(()) }
