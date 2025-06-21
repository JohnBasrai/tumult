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
    Generate,
    GenerateOk
    {
        #[serde(rename = "id")]
        uuid: String,
    },
}

struct UniqueNode
{
    node: String,
    id:   usize,
}

impl Node<(), Payload> for UniqueNode
{
    fn from_init(_state: (), init: Init, _tx: Sender<Event<Payload>>) -> Result<Self>
    {
        Ok(UniqueNode {
            node: init.node_id,
            id:   1,
        })
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
            Payload::Generate =>
            {
                let uuid = format!("{}/{}", self.node, self.id);
                reply.body.payload = Payload::GenerateOk { uuid };
                serde_json::to_writer(&mut *output, &reply)
                    .context("serialize response to generate")?;
                output.write_all(b"\n").context("write newline")?;
            }
            Payload::GenerateOk { .. } =>
            {}
        }
        Ok(())
    }
}

fn main() -> Result<()> { main_loop::<_, UniqueNode, _, _>(()) }
