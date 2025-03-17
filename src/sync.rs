/// Similar to the `dispatch_service_async` but in sync mode.
#[macro_export]
macro_rules! dispatch_service_sync {
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
        //   - `async fn` to `fn`
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
                -> Result<$reply, tonic::Status>;
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
            fn handle_and_reply<B>(self, ctx: &mut B)
                where B: DispatchBackend + Send + Sync + 'static
            {
                match self {
                    $(
                        DispatchRequest::[<$method:camel>](req, resp_tx) => {
                            let reply = ctx.$method(req);
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                }
            }
        }

        // Context for the tonic server, used to dispatch requests.
        pub struct [<$service DispatchServer>] {
            txs: Vec<std::sync::mpsc::SyncSender<DispatchRequest>>,
        }

        impl [<$service DispatchServer>] {
            // create with txs
            fn with_txs(txs: Vec<std::sync::mpsc::SyncSender<DispatchRequest>>) -> Self {
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
            fn backend_task<B>(mut req_rx: std::sync::mpsc::Receiver<DispatchRequest>)
                where B: Default + DispatchBackend + Send + Sync + 'static
            {
                let mut ctx = B::default();
                while let Ok(request) = req_rx.recv() {
                    request.handle_and_reply(&mut ctx);
                }
            }

            let mut req_txs = Vec::new();
            for _ in 0..task_num {
                let (req_tx, req_rx) = std::sync::mpsc::sync_channel(channel_capacity);

                std::thread::spawn(|| backend_task::<B>(req_rx));

                req_txs.push(req_tx);
            }

            [<$service DispatchServer>]::with_txs(req_txs)
        }

        } // end of paste!
    }
}
