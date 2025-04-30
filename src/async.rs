/// Define the service and build the mapping relationship between tonic
/// network tasks and your asynchronous business tasks.
///
/// Use `dispatch_service_sync!` instead for synchronous mode.
/// See [the module-level documentation](super) for more information
/// about the 2 modes.
///
/// Parameters:
///
/// - `$service` Original service name. Because we need to generate new
///   service name based on this name, so do not give the module prefix.
///
/// - `$hash_by: $hash_type` The field in request types which is used
///   to calculate which business task to dispatched to. All request
///   types should contain this field.
///
/// - `$shard_mutable_method ($shard_mutable_request) -> $shard_mutable_reply`
///   gRPC methods that work on mutable shard (but not on item). E.g. create
///   or remove items on the shard.
///
/// - `$shard_readonly_method ($shard_readonly_request) -> $shard_readonly_reply`
///   gRPC methods that work on readonly shard (but not on item). E.g. list
///   items on the shard.
///
/// - `$item_mutable_method ($item_mutable_request) -> $item_mutable_reply`
///   gRPC mutable methods that work on item. E.g. update item itself.
///
/// - `$item_readonly_method ($item_readonly_request) -> $item_readonly_reply`
///   gRPC readonly methods that work on item. E.g. query item itself.
///
///
/// This macro defines 4 items:
///
/// - `trait DispatchBackendShard` is for each backend shard. You
///   need to implement this trait for your shard context. It has 2 parts:
///    1. associated type Item, and get_item/get_item_mut methods;
///    2. gRPC methods that works at shard (but not item), e.g. create/delete.
///
/// - `trait DispatchBackendItem` is for each backend item. It has
///   mutable and readonly gRPC methods that works at item. You
///   need to implement this trait for your item.
///
///   The formats of all methods are similar to the original tonic ones,
///   except that changes
///     - self: from `&self` to `&mut self` (for mutable methods)
///     - parameter: from `Request<R>` to `R`
///     - retuen value: from `Response<R>` to `R`
///
///   However the meaning of `self` changes. For the original tonic methods,
///   the `self` points to a global service context. While here, for shard
///   methods the `self` points to a context for each shard, and for
///   item mutable/readonly methods the `self` points to the item.
///
/// - `fn start_simple_dispatch_backend` This starts a simple kind of
///   backend tasks, which just listen on the request channel.
///   If you want more complex backend task (e.g. listen on another
///   channel too), you have to create tasks and channels youself.
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

            tokio::sync::mpsc::Sender,

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
        //   - self: from `&self` to `mut &self` (mutable methods)
        //   - parameter: from `Request<R>` to `R`
        //   - retuen value: from `Response<R>` to `R`
        // ```
        trait DispatchBackendShard {
            // part-1
            type Item: DispatchBackendItem + Send + Sync;
            fn get_item(&self, key: &$hash_type) -> Result<&Self::Item, Status>;
            fn get_item_mut(&mut self, key: &$hash_type) -> Result<&mut Self::Item, Status>;

            // part-2
            $(
                fn $shard_mutable_method(&mut self, request: $shard_mutable_request)
                -> impl std::future::Future<Output = Result<$shard_mutable_reply, tonic::Status>> + Send;
            )*
            $(
                fn $shard_readonly_method(&self, request: $shard_readonly_request)
                -> impl std::future::Future<Output = Result<$shard_readonly_reply, tonic::Status>> + Send;
            )*
        }
        trait DispatchBackendItem {
            $(
                fn $item_mutable_method(&mut self, request: $item_mutable_request)
                -> impl std::future::Future<Output = Result<$item_mutable_reply, tonic::Status>> + Send;
            )*
            $(
                fn $item_readonly_method(&self, request: $item_readonly_request)
                -> impl std::future::Future<Output = Result<$item_readonly_reply, tonic::Status>> + Send;
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
            async fn handle_and_reply<B>(self, ctx: &mut B)
                where B: DispatchBackendShard + Send + Sync + 'static
            {
                match self {
                    $(
                        DispatchRequest::[<$shard_mutable_method:camel>](req, resp_tx) => {
                            let reply = ctx.$shard_mutable_method(req).await;
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$shard_readonly_method:camel>](req, resp_tx) => {
                            let reply = ctx.$shard_readonly_method(req).await;
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$item_mutable_method:camel>](req, resp_tx) => {
                            let reply = match ctx.get_item_mut(&req.$hash_by) {
                                Ok(i) => i.$item_mutable_method(req).await,
                                Err(err) => Err(err),
                            };
                            resp_tx.send(reply).unwrap();
                        }
                    )*
                    $(
                        DispatchRequest::[<$item_readonly_method:camel>](req, resp_tx) => {
                            let reply = match ctx.get_item(&req.$hash_by) {
                                Ok(i) => i.$item_readonly_method(req).await,
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
            async fn backend_task<B>(mut backend: B, mut req_rx: tokio::sync::mpsc::Receiver<DispatchRequest>)
                where B: DispatchBackendShard + Send + Sync + 'static
            {
                while let Some(request) = req_rx.recv().await {
                    request.handle_and_reply(&mut backend).await;
                }
            }

            let mut req_txs = Vec::new();
            for _ in 0..task_num {
                let (req_tx, req_rx) = tokio::sync::mpsc::channel(channel_capacity);

                tokio::spawn(backend_task(backend.clone(), req_rx));

                req_txs.push(req_tx);
            }

            [<$service DispatchServer>]::with_txs(req_txs)
        }

        } // end of paste!
    }
}
