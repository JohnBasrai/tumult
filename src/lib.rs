use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{BufRead, StdoutLock, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Payload> {
    pub src: String,
    pub dest: String,
    pub body: Body<Payload>,
}

impl<Payload> Message<Payload> {
    pub fn into_reply(self, id: Option<&mut usize>) -> Self {
        let msg_id = id.map(|id| {
            let msg_id = *id;
            *id += 1;
            msg_id
        });
        Self {
            src: self.dest,
            dest: self.src,
            body: Body {
                msg_id,
                in_reply_to: self.body.msg_id,
                payload: self.body.payload,
            },
        }
    }

    pub fn send(&self, output: &mut impl Write) -> Result<()>
    where
        Payload: Serialize,
    {
        serde_json::to_writer(&mut *output, self).context("serialize response message")?;
        output.write_all(b"\n").context("write newline")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Event<Payload, InjectedPayload = ()> {
    Message(Message<Payload>),
    Injected(InjectedPayload),
    EOF,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Payload> {
    pub msg_id: Option<usize>,
    pub in_reply_to: Option<usize>,
    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum InitPayload {
    Init(Init),
    InitOk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

pub trait Node<S, Payload, InjectedPayload = ()> {
    fn from_init(
        state: S,
        init: Init,
        inject: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> Result<Self>
    where
        Self: Sized;

    fn step(
        &mut self,
        input: Event<Payload, InjectedPayload>,
        output: &mut StdoutLock,
    ) -> Result<()>;
}

pub fn main_loop<S, N, P, IP>(init_state: S) -> Result<()>
where
    P: DeserializeOwned + Send + 'static,
    N: Node<S, P, IP>,
    IP: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();

    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();
    let mut stdout = std::io::stdout().lock();

    let init_msg: Message<InitPayload> = serde_json::from_str(
        &stdin
            .next()
            .expect("EOF before init message-1?")
            .context("EOF before init message-2")?,
    )
    .context("Error deserializing init message")?;
    let InitPayload::Init(init) = init_msg.body.payload else {
        panic!("Expected init message first");
    };
    let mut node: N =
        Node::from_init(init_state, init, tx.clone()).context("initilization failed")?;

    let reply = Message {
        src: init_msg.dest,
        dest: init_msg.src,
        body: Body {
            msg_id: Some(0),
            in_reply_to: init_msg.body.msg_id,
            payload: InitPayload::InitOk,
        },
    };
    serde_json::to_writer(&mut stdout, &reply).context("serialize init response")?;
    stdout.write_all(b"\n").context("write newline")?;

    drop(stdin);
    let jh = std::thread::spawn(move || {
        let stdin = std::io::stdin().lock();
        for line in stdin.lines() {
            let line = line.context("Maelstrom input from STDIN could not be read")?;
            let input: Message<P> = serde_json::from_str(&line)
                .context("Maelstrom input from STDIN could not be deserialized")?;
            if tx.send(Event::Message(input)).is_err() {
                return Ok::<_, anyhow::Error>(());
            }
        }
        let _ = tx.send(Event::EOF);
        Ok(())
    });

    for input in rx {
        node.step(input, &mut stdout)
            .context("Node step function failed")?;
    }

    jh.join()
        .expect("stdin thread panic")
        .context("stdin thread error")?;

    Ok(())
}
