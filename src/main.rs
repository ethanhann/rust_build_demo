use async_trait::async_trait;
use pingora::prelude::*;
use std::sync::Arc;

pub struct LB(Arc<LoadBalancer<RoundRobin>>);

#[async_trait]
impl ProxyHttp for LB {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self
            .0
            .select(b"", 256)
            .ok_or_else(|| Error::new_str("no upstream available"))?;

        let peer = HttpPeer::new(upstream, true, "one.one.one.one".to_string());
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request.insert_header("Host", "one.one.one.one")?;
        Ok(())
    }
}

fn main() {
    let mut server = Server::new(None).expect("failed to create server");
    server.bootstrap();

    let upstreams =
        LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).expect("invalid upstreams");

    let mut proxy = http_proxy_service(&server.configuration, LB(Arc::new(upstreams)));
    proxy.add_tcp("0.0.0.0:6188");

    server.add_service(proxy);
    server.run_forever();
}
