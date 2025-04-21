#[macro_export]
macro_rules! service_method_body {
    ($method:ident, $self:ident, $request:expr, $hash_by:ident) => {
        paste::paste! {
            {
                let request = $request.into_inner();

                let shard = Self::calc_hash(&request.$hash_by) as usize % $self.txs.len();

                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

                let biz_req = DispatchRequest::[<$method:camel>](request, resp_tx);

                match $self.txs[shard].try_send(biz_req) {
                    Ok(()) => resp_rx.await.unwrap().map(tonic::Response::new),
                    Err(_) => Err(tonic::Status::unavailable(String::new())),
                }
            }
        }
    }
}

/// Similar to the `dispatch_service_async` but in sync mode.
#[macro_export]
macro_rules! dispatch_service_sync {
    (
        $service:ty,
        $hash_by:ident : $hash_type:ty,

        [ $(
            $shard_method:ident ($shard_request:ty) -> $shard_reply:ty,
        )* ],

        [ $(
            $mutable_method:ident ($mutable_request:ty) -> $mutable_reply:ty,
        )* ],

        [ $(
            $readonly_method:ident ($readonly_request:ty) -> $readonly_reply:ty,
        )* ]
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
        trait DispatchBackendShard {
            type Item: DispatchBackendItem;
            fn get(&self, key: &$hash_type) -> Result<&Self::Item, Status>;
            fn get_mut(&mut self, key: &$hash_type) -> Result<&mut Self::Item, Status>;

            $(
                fn $shard_method(&mut self, request: $shard_request)
                -> Result<$shard_reply, tonic::Status>;
            )*
        }
        trait DispatchBackendItem {
            $(
                fn $mutable_method(&mut self, request: $mutable_request)
                -> Result<$mutable_reply, tonic::Status>;
            )*
            $(
                fn $readonly_method(&self, request: $readonly_request)
                -> Result<$readonly_reply, tonic::Status>;
            )*
        }

        // Dispatched request.
        //
        // This is an internal type. You would not need to know this.
        enum DispatchRequest {
            $(
                [<$shard_method:camel>] ($shard_request, tokio::sync::oneshot::Sender<Result<$shard_reply, tonic::Status>>),
            )*
            $(
                [<$mutable_method:camel>] ($mutable_request, tokio::sync::oneshot::Sender<Result<$mutable_reply, tonic::Status>>),
            )*
            $(
                [<$readonly_method:camel>] ($readonly_request, tokio::sync::oneshot::Sender<Result<$readonly_reply, tonic::Status>>),
            )*
        }

        impl DispatchRequest {
            fn handle_and_reply<B>(self, ctx: &mut B)
                where B: DispatchBackendShard + Send + Sync + 'static
            {
                match self {
                    $(
                        DispatchRequest::[<$shard_method:camel>](req, resp_tx) => {
                            let reply = ctx.$shard_method(req);
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$mutable_method:camel>](req, resp_tx) => {
                            let reply = match ctx.get_mut(&req.$hash_by) {
                                Ok(i) => i.$mutable_method(req),
                                Err(err) => Err(err),
                            };
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$readonly_method:camel>](req, resp_tx) => {
                            let reply = match ctx.get(&req.$hash_by) {
                                Ok(i) => i.$readonly_method(req),
                                Err(err) => Err(err),
                            };
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
                async fn $shard_method(
                    &self,
                    request: tonic::Request<$shard_request>,
                ) -> Result<tonic::Response<$shard_reply>, tonic::Status> {
                    tonic_server_dispatch::service_method_body!($shard_method, self, request, $hash_by)
                }
             )*
             $(
                async fn $mutable_method(
                    &self,
                    request: tonic::Request<$mutable_request>,
                ) -> Result<tonic::Response<$mutable_reply>, tonic::Status> {
                    tonic_server_dispatch::service_method_body!($mutable_method, self, request, $hash_by)
                }
            )*
            $(
                async fn $readonly_method(
                    &self,
                    request: tonic::Request<$readonly_request>,
                ) -> Result<tonic::Response<$readonly_reply>, tonic::Status> {
                    tonic_server_dispatch::service_method_body!($readonly_method, self, request, $hash_by)
                }
            )*
        }

        // Start a simple backend service.
        //
        // You need to write your own code if any more feature, for example
        // the backend task need to listen on another channel.
        #[allow(dead_code)]
        fn start_simple_dispatch_backend<B>(backend: B, task_num: usize, channel_capacity: usize)
            -> [<$service DispatchServer>]
            where B: Clone + DispatchBackendShard + Send + Sync + 'static
        {
            fn backend_task<B>(mut backend: B, mut req_rx: std::sync::mpsc::Receiver<DispatchRequest>)
                where B: DispatchBackendShard + Send + Sync + 'static
            {
                while let Ok(request) = req_rx.recv() {
                    request.handle_and_reply(&mut backend);
                }
            }

            let mut req_txs = Vec::new();
            for i in 0..task_num {
                let (req_tx, req_rx) = std::sync::mpsc::sync_channel(channel_capacity);

                let backend = backend.clone();
                std::thread::Builder::new()
                    .name(format!("biz-worker-{i}"))
                    .spawn(|| backend_task::<B>(backend, req_rx))
                    .unwrap();

                req_txs.push(req_tx);
            }

            [<$service DispatchServer>]::with_txs(req_txs)
        }

        } // end of paste!
    }
}
