grpc-router originated as a workaround for
[missing feature](https://github.com/tower-rs/tower-grpc/issues/2) of
[tower-grpc lib](https://github.com/tower-rs/tower-grpc). The library does
not support serving of multiple services on a single endpoint, which mayastor
project desperately needs, because it needs to serve CSI identity and node
services on a single unix domain socket in order to comply with the CSI spec.

Example usage is missing csi module with auto-generated `IdentityServer` and
`NodeServer` and service implementations `Identity` and `Node`. It can't be
used on its own but gives an idea how grpc-router should be used:

```rust
extern crate grpc_router;

use grpc_router::Router2;

fn create_service() {
    let csi_service = Router2::new(
        "/csi.v1.Identity/",
        csi::server::IdentityServer::new(Identity {}),
        csi::server::NodeServer::new(Node {}),
    );

    let mut csi_server = Server::new(csi_service);
    // Bind csi_server to http2 server and continue as normally
    // ...
}
```

All requests with `/csi.v1.Identity/` prefix in URL will be routed to
`csi::server::IdentityServer` and all other requests to
`csi::server::NodeServer`, which is kind of a default grpc service in
this case.
