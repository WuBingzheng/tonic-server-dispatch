/// Define the service and build the mapping relationship between tonic
/// network tasks and your business tasks.
///
/// Parameters:
///
/// - `$service` Original service name. Because we need to generate new
///   service name based on this name, so do not give the module prefix.
///
/// - `$hash_by` The field in request types which is used to calculate
///   which business task to dispatched to. All request types should
///   contain this field.
///
/// - `$method` The gRPC method name. You need list all methods.
///
/// - `$request` The gRPC request type.
///
/// - `$reply` The gRPC response type.
///
///
/// This macro defines 3 items:
///
/// - `trait DispatchBackend` This defines all your service's gRPC
///   methods, and you need to implement this trait for your service
///   context.
///
/// - `fn start_simple_dispatch_backend` This starts a simple kind of
///   backend tasks, which just listen on the request channel.
///    If you want more complex backend task (e.g. listen on another
///    channel too), you have to create tasks and channels youself.
///
/// - `struct [<$service DispatchServer>]` This defines the real tonic
///   service, and this macro implement it automatically. If you use
///   the `start_simple_dispatch_backend` which handles this struct
///   already, then you do not need to touch this. But if you need to
///   build backend tasks yourself, then you need to create channels
///   and this struct with their `Sender` ends by its `with_txs()`
///   method. See `start_simple_dispatch_backend()`'s code for example.
///
/// Read the [DictService] example's source code for a better understanding.
///
/// [DictService]: https://github.com/WuBingzheng/tonic-server-dispatch/blob/master/examples/src/server_async.rs
///
#[macro_export]
macro_rules! dispatch_service_async {
    (
        $service:ty,
        $hash_by:ident,
        $(
            $method:ident ($request:ty) -> $reply:ty,
        )*
    ) => {

        paste::paste! {

        // Trait for backend business context.
        //
        // This defines all gRPC methods for this server.
        //
        // The formats of methods are similar to the original tonic ones,
        // except that changes
        //   - self: from `&self` to `mut &self`
        //   - parameter: from `Request<R>` to `R`
        //   - retuen value: from `Response<R>` to `R`
        //
        // For example:
        //
        // ```
        // impl DispatchBackend for MyGreeter {
        //     async fn say_hello(&mut self, req: SayHelloRequest) -> Result<SayHelloReply, Status> {
        //         Ok(SayHelloReply{
        //             say: "hello".into(),
        //         })
        //     }
        // }
        // ```
        trait DispatchBackend {
            $(
                fn $method(&mut self, request: $request)
                -> impl std::future::Future<Output = Result<$reply, tonic::Status>> + Send;
            )*
        }

        // Dispatched request.
        //
        // This is an internal type. You would not need to know this.
        enum DispatchRequest {
            $(
                [<$method:camel>] ($request, tokio::sync::oneshot::Sender<Result<$reply, tonic::Status>>),
            )*
        }

        impl DispatchRequest {
            async fn handle_and_reply<B>(self, ctx: &mut B)
                where B: DispatchBackend + Send + Sync + 'static
            {
                match self {
                    $(
                        DispatchRequest::[<$method:camel>](req, resp_tx) => {
                            let reply = ctx.$method(req).await;
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                }
            }
        }

        // Context for the tonic server, used to dispatch requests.
        pub struct [<$service DispatchServer>] {
            txs: Vec<tokio::sync::mpsc::Sender<DispatchRequest>>,
        }

        impl [<$service DispatchServer>] {
            // create with txs
            fn with_txs(txs: Vec<tokio::sync::mpsc::Sender<DispatchRequest>>) -> Self {
                Self { txs }
            }

            // internal method
            fn calc_hash(item: &impl std::hash::Hash) -> u64 {
                use std::hash::Hasher;
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                item.hash(&mut hasher);
                hasher.finish()
            }
        }

        // The tonic server implementation.
        //
        // Dispatch the request to backend, and wait for the reply.
        #[tonic::async_trait]
        impl $service for [<$service DispatchServer>] {
            $(
                async fn $method(
                    &self,
                    request: tonic::Request<$request>,
                ) -> Result<tonic::Response<$reply>, tonic::Status> {
                    let request = request.into_inner();

                    let shard = Self::calc_hash(&request.$hash_by) as usize % self.txs.len();

                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

                    let biz_req = DispatchRequest::[<$method:camel>](request, resp_tx);

                    match self.txs[shard].try_send(biz_req) {
                        Ok(()) => resp_rx.await.unwrap().map(tonic::Response::new),
                        Err(_) => Err(tonic::Status::unavailable(String::new())),
                    }
                }
            )*
        }

        // Start a simple backend service.
        //
        // You need to write your own code if any more feature, for example
        // the backend task need to listen on another channel.
        #[allow(dead_code)]
        fn start_simple_dispatch_backend<B>(task_num: usize, channel_capacity: usize)
            -> [<$service DispatchServer>]
            where B: Default + DispatchBackend + Send + Sync + 'static
        {
            async fn backend_task<B>(mut req_rx: tokio::sync::mpsc::Receiver<DispatchRequest>)
                where B: Default + DispatchBackend + Send + Sync + 'static
            {
                let mut ctx = B::default();
                while let Some(request) = req_rx.recv().await {
                    request.handle_and_reply(&mut ctx).await;
                }
            }

            let mut req_txs = Vec::new();
            for _ in 0..task_num {
                let (req_tx, req_rx) = tokio::sync::mpsc::channel(channel_capacity);

                tokio::spawn(backend_task::<B>(req_rx));

                req_txs.push(req_tx);
            }

            [<$service DispatchServer>]::with_txs(req_txs)
        }

        } // end of paste!
    }
}
