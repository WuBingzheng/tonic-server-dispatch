/// internal macro
///
// define tonic Server: [<$service DispatchServer>]
//
// used by sync and async modes both
#[macro_export]
macro_rules! _define_dispatch_server {
    (
        $service:ty,
        $hash_by:ident : $hash_type:ty,

        $mpsc_sender_type:ty,

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

        // Context for the tonic server, used to dispatch requests.
        pub struct [<$service DispatchServer>] {
            txs: Vec<$mpsc_sender_type<DispatchRequest>>,
        }

        impl [<$service DispatchServer>] {
            // create with txs
            fn with_txs(txs: Vec<$mpsc_sender_type<DispatchRequest>>) -> Self {
                Self { txs }
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
                    tonic_server_dispatch::_service_method_body!($shard_method, self, request, $hash_by)
                }
             )*
             $(
                async fn $mutable_method(
                    &self,
                    request: tonic::Request<$mutable_request>,
                ) -> Result<tonic::Response<$mutable_reply>, tonic::Status> {
                    tonic_server_dispatch::_service_method_body!($mutable_method, self, request, $hash_by)
                }
            )*
            $(
                async fn $readonly_method(
                    &self,
                    request: tonic::Request<$readonly_request>,
                ) -> Result<tonic::Response<$readonly_reply>, tonic::Status> {
                    tonic_server_dispatch::_service_method_body!($readonly_method, self, request, $hash_by)
                }
            )*
        }

        }
    }
}

/// internal macro
///
// service method body, for shard_method, mutable_method and readonly_method all.
//
// dispatch requests to backends and wait for repsponse.
#[macro_export]
macro_rules! _service_method_body {
    ($method:ident, $self:ident, $request:expr, $hash_by:ident) => {
        paste::paste! {
            {
                fn calc_hash(item: &impl std::hash::Hash) -> u64 {
                    use std::hash::Hasher;
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    item.hash(&mut hasher);
                    hasher.finish()
                }

                let request = $request.into_inner();

                let shard = calc_hash(&request.$hash_by) as usize % $self.txs.len();

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
