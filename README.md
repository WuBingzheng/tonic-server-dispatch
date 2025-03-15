A typical architecture of network service is that after receiving a
request, the network tasks dispatch it to the business tasks according
to some fields. In this way, requests for the same content can be
dispatched in the same task to avoid shared state or locking.
[This tokio tutorial] gives detailed description.

The same is true in `tonic`'s gRPC server. The dispatch of requests
from network tasks to the business task has a pattern. This crate is
an abstraction of this pattern to simplify the repetitive work in
the application.

Read the [documentation] for more detail.


[This tokio tutorial]: https://tokio.rs/tokio/tutorial/channels
[documentation]: https://docs.rs/tonic-server-dispatch
