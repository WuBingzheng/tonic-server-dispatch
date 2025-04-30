/// Similar to the `dispatch_service_async` but in sync mode.
///
/// See [the module-level documentation](super) for more information
/// about the 2 modes.
///
/// The only API difference is that the methods in `DispatchBackendShard`
/// and `DispatchBackendItem` are sync but not `async fn`.
///
/// And there is also a sync mode [DictService] example.
///
/// [DictService]: https://github.com/WuBingzheng/tonic-server-dispatch/blob/master/examples/src/server_sync.rs
#[macro_export]
macro_rules! dispatch_service_sync {
    (
        $service:ty,
        $hash_by:ident : $hash_type:ty,

        [ $(
            $shard_mutable_method:ident ($shard_mutable_request:ty) -> $shard_mutable_reply:ty,
        )* ],

        [ $(
            $shard_readonly_method:ident ($shard_readonly_request:ty) -> $shard_readonly_reply:ty,
        )* ],

        [ $(
            $item_mutable_method:ident ($item_mutable_request:ty) -> $item_mutable_reply:ty,
        )* ],

        [ $(
            $item_readonly_method:ident ($item_readonly_request:ty) -> $item_readonly_reply:ty,
        )* ]
    ) => {

        // define tonic Server: [<$service DispatchServer>]
        //
        // this part is same for sync and async modes.
        tonic_server_dispatch::_define_dispatch_server!(
            $service,
            $hash_by: $hash_type,

            std::sync::mpsc::SyncSender,

            [ $(
                $shard_mutable_method ($shard_mutable_request) -> $shard_mutable_reply,
            )* ],

            [ $(
                $shard_readonly_method ($shard_readonly_request) -> $shard_readonly_reply,
            )* ],

            [ $(
                $item_mutable_method ($item_mutable_request) -> $item_mutable_reply,
            )* ],

            [ $(
                $item_readonly_method ($item_readonly_request) -> $item_readonly_reply,
            )* ]
        );

        paste::paste! {

        // 2 traits for backend business context: Shard and Item.
        //
        // DispatchBackendShard is for each backend shard. It has 2 parts:
        // 1. associated type Item, and get_item/get_item_mut methods;
        // 2. gRPC methods that works at shard (but not item), e.g. create/delete/list.
        //
        // DispatchBackendItem is for each backend item. It only has
        // gRPC methods that works at item.
        //
        // The formats of all methods are similar to the original tonic ones,
        // except that changes
        //   - `async fn` to `fn`
        //   - self: from `&self` to `mut &self` (mutable methods)
        //   - parameter: from `Request<R>` to `R`
        //   - retuen value: from `Response<R>` to `R`
        // ```
        trait DispatchBackendShard {
            // part-1
            type Item: DispatchBackendItem;
            fn get_item(&self, key: &$hash_type) -> Result<&Self::Item, Status>;
            fn get_item_mut(&mut self, key: &$hash_type) -> Result<&mut Self::Item, Status>;

            // part-2
            $(
                fn $shard_mutable_method(&mut self, request: $shard_mutable_request)
                -> Result<$shard_mutable_reply, tonic::Status>;
            )*
            $(
                fn $shard_readonly_method(&self, request: $shard_readonly_request)
                -> Result<$shard_readonly_reply, tonic::Status>;
            )*
        }
        trait DispatchBackendItem {
            $(
                fn $item_mutable_method(&mut self, request: $item_mutable_request)
                -> Result<$item_mutable_reply, tonic::Status>;
            )*
            $(
                fn $item_readonly_method(&self, request: $item_readonly_request)
                -> Result<$item_readonly_reply, tonic::Status>;
            )*
        }

        // Dispatched request.
        //
        // This is an internal type. You would not need to know this.
        enum DispatchRequest {
            $(
                [<$shard_mutable_method:camel>] ($shard_mutable_request, tokio::sync::oneshot::Sender<Result<$shard_mutable_reply, tonic::Status>>),
            )*
            $(
                [<$shard_readonly_method:camel>] ($shard_readonly_request, tokio::sync::oneshot::Sender<Result<$shard_readonly_reply, tonic::Status>>),
            )*
            $(
                [<$item_mutable_method:camel>] ($item_mutable_request, tokio::sync::oneshot::Sender<Result<$item_mutable_reply, tonic::Status>>),
            )*
            $(
                [<$item_readonly_method:camel>] ($item_readonly_request, tokio::sync::oneshot::Sender<Result<$item_readonly_reply, tonic::Status>>),
            )*
        }

        impl DispatchRequest {
            fn handle_and_reply<B>(self, ctx: &mut B)
                where B: DispatchBackendShard + Send + Sync + 'static
            {
                match self {
                    $(
                        DispatchRequest::[<$shard_mutable_method:camel>](req, resp_tx) => {
                            let reply = ctx.$shard_mutable_method(req);
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$shard_readonly_method:camel>](req, resp_tx) => {
                            let reply = ctx.$shard_readonly_method(req);
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$item_mutable_method:camel>](req, resp_tx) => {
                            let reply = match ctx.get_item_mut(&req.$hash_by) {
                                Ok(i) => i.$item_mutable_method(req),
                                Err(err) => Err(err),
                            };
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$item_readonly_method:camel>](req, resp_tx) => {
                            let reply = match ctx.get_item(&req.$hash_by) {
                                Ok(i) => i.$item_readonly_method(req),
                                Err(err) => Err(err),
                            };
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                }
            }
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
