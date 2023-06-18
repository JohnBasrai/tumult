The [fly.io distributed systems challenges](https://fly.io/dist-sys/) solved in Rust.

Status is currently have solutions for the following challenges:
[Echo](https://fly.io/dist-sys/1/), 
[Unique ID Generation](https://fly.io/dist-sys/2/), and 
[Broadcast](https://fly.io/dist-sys/3a/)

The next challenge is [Grow-Only Counter](https://fly.io/dist-sys/4/) which apparently
requires go RPC client to talk to the maelstrom server
[seq-kv](https://github.com/jepsen-io/maelstrom/blob/main/doc/services.md#seq-kv) over a
back channel. I'm still thinking about how to solve that issue.  The first thing that
comes to mind is to run go as separate process which rust will call each time it is needed
and make synchronous call to it using Rust's std::process:Command.

