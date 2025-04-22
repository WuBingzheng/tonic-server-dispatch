//! A typical architecture of network service is that after receiving a
//! request, the network tasks dispatch it to the business tasks according
//! to some fields. In this way, requests for the same content can be
//! dispatched in the same task to avoid shared state or locking.
//! This [tokio tutorial] gives detailed description.
//!
//! The same is true in `tonic`'s gRPC server. The dispatch of requests
//! from network tasks to the business task has a pattern. This crate is
//! an abstraction of this pattern to simplify the repetitive work in
//! the application.
//!
//!
//! # Examples
//!
//! This library is a bit difficult to get started with.
//! See the DictService service examples in [async mode] or [sync mode],
//! before the API documentation.
//!
//!
//! # Async vs Sync
//!
//! The business jobs can run in async or sync mode.
//!
//! The [tokio tutorial] talks about the async mode. The business jobs run
//! as tokio-tasks above the tokio runtime:
//!
//!     network  +--+ +--+ +--+   channels   +--+ +--+ +--+  business
//!       tasks  |  | |  | |  | <----------> |  | |  | |  |  tasks*
//!              +--+ +--+ +--+              +--+ +--+ +--+
//!       tokio  +----------------------------------------+
//!     runtime  |                                        |
//!              +----------------------------------------+
//!              +---+ +---+                          +---+
//!     threads  |   | |   |       ...                |   |
//!              +---+ +---+                          +---+
//!
//! If your business jobs contains no async code, then they can also
//! run as native threads:
//!
//!     network  +--+ +--+ +--+              +---+ +---+ +---+ business
//!       tasks  |  | |  | |  |              |   | |   | |   | threads*
//!              +--+ +--+ +--+              |   | |   | |   |
//!       tokio  +------------+   channels   |   | |   | |   |
//!     runtime  |            | <----------> |   | |   | |   |
//!              +------------+              |   | |   | |   |
//!              +---+    +---+              |   | |   | |   |
//!     threads  |   |... |   |              |   | |   | |   |
//!              +---+    +---+              +---+ +---+ +---+
//!
//! This crate supports both modes.
//!
//! [tokio tutorial]: https://tokio.rs/tokio/tutorial/channels
//! [async mode]: https://github.com/WuBingzheng/tonic-server-dispatch/blob/master/examples/src/server_async.rs
//! [sync mode]: https://github.com/WuBingzheng/tonic-server-dispatch/blob/master/examples/src/server_sync.rs


mod sync;
mod r#async;

mod common;
