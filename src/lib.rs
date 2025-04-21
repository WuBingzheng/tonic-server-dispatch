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
//!
//! # Usage
//!
//! Let's take the [DictService in async-mode] as example. The [sync-mode]
//! is similar.
//!
//! We assume that you are familiar with how to implement the original
//! tonic server. Here we just talk about the parts related to this
//! crate.
//!
//! 0. Add this Crate
//!
//!    Add this crate and `paste` to your Cargo.toml.
//!
//!     ``` toml
//!     tonic-server-dispatch = "*"
//!     paste = "1.0"
//!     ```
//!
//! 1. Define your Service
//!
//!    This macro builds the mapping relationship between tonic
//!    network tasks and your business tasks.
//!
//!     ``` rust
//!     tonic_server_dispatch::dispatch_service_async! {
//!         DictService, // original service name
//!         key, // hash by this request field
//!
//!         // service methods
//!         set(SetRequest) -> SetReply,
//!         get(Key) -> Value,
//!         delete(Key) -> Value,
//!     }
//!     ```
//!
//!    This macro is the main part of this crate. Go to its
//!    [doc page] for more detail.
//!
//!    For sync-mode, use `dispatch_service_sync!` instead.
//!
//! 2. Implement your Service
//!
//!    Define your business context for each task, and implement
//!    `DispatchBackend` for it. `DispatchBackend` defines all service
//!    methods, similar to the original tonic ones.
//! 
//!     ``` rust
//!     #[derive(Default)]
//!     struct DictCtx (HashMap<String, f64>);
//!
//!     impl DispatchBackend for DictCtx {
//!         async fn get(&mut self, req: Key) -> Result<Value, Status> {
//!             match self.0.get(&req.key) {
//!                 Some(value) => Ok(Value { value: *value }),
//!                 None => Err(Status::not_found(String::new())),
//!             }
//!         }
//!
//!         // all other methods ...
//!     }
//!     ```
//!
//!    Compare to the original tonic prototype:
//!
//!    ```
//!    async fn get(&self, req: tonic::Request<Key>)
//!        -> Result<tonic::Response<Value>, tonic::Status>
//!    ```
//!
//!    the difference:
//!
//!    - `&self` -> `&mut self`
//!    - `tonic::Request<Key>` -> `Key`
//!    - `tonic::Response<Value>` -> `Value`
//!
//!    For sync-mode, remove the `async` before `fn`.
//!
//! 3. Start your Service
//!
//!    This starts backend tasks and creates channels.
//!    The requests are dispatched from network tasks to backend
//!    tasks by the channels, and the response are sent back by
//!    oneshot channels.
//!
//!     ```
//!     let svc = start_simple_dispatch_backend::<DictCtx>(16, 10);
//!     ```
//!
//!    As the function's name suggests, it just starts the simple
//!    kind of backend task, which just listen on the request channel.
//!    If you want more complex backend task (e.g. listen on another
//!    channel too), you have to create tasks and channels youself.
//!    However, the implementation of this function can also be used
//!    as your reference.
//!
//! Now we have finished the dispatch level. It is very simple, isn't it?
//! Go [DictService in async-mode] and [sync-mode] for the full source code.
//!
//! [tokio tutorial]: https://tokio.rs/tokio/tutorial/channels
//! [DictService in async-mode]: https://github.com/WuBingzheng/tonic-server-dispatch/blob/master/examples/src/server_async.rs
//! [sync-mode]: https://github.com/WuBingzheng/tonic-server-dispatch/blob/master/examples/src/server_sync.rs
//! [doc page]: macro.dispatch_service.html


mod sync;
mod r#async;

mod common;
