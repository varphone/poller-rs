﻿poller-rs
=========

[![Build Status](https://ci.vaxpl.com/api/badges/rdst/poller-rs/status.svg?ref=refs/heads/{{BRANCH_NAME}})](https://ci.vaxpl.com/rdst/poller-rs)

File I/O events library for Rust.

Examples
--------

```rs
use poller::{Events, Poller};

fn main() {
    let mut poller = Poller::new();
    poller.add(0, Events::new().with_read(), None);
    for (fd, events, ctx) in poller.pull_events(1000).unwrap().iter() {
        println!("Fd={}, Events={}, Context={:?}", fd, events, ctx);
    }
}
```
