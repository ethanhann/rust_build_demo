// NOTE: This code is intentionally simple and is not meant to be a production-ready application.
// The purpose of this repository is to demonstrate optimized vs. unoptimized GitHub Actions CI/CD
// workflows for Rust projects.
// The actual functionality of this load balancer proxy is unimportant.
// Focus on the build configuration and CI workflow files instead.

use async_trait::async_trait;
use pingora::prelude::*;
use std::sync::Arc;

const UPSTREAM_HOST: &str = "one.one.one.one";

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

        let peer = HttpPeer::new(upstream, true, UPSTREAM_HOST.to_string());
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request.insert_header("Host", UPSTREAM_HOST)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_selects_a_configured_upstream() {
        // Arrange
        let lb = LoadBalancer::<RoundRobin>::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).unwrap();

        // Act
        let selected = lb.select(b"", 256);

        // Assert
        assert!(selected.is_some());
        let addr = selected.unwrap().addr.to_string();
        assert!(
            addr == "1.1.1.1:443" || addr == "1.0.0.1:443",
            "selected upstream {addr} was not one of the configured backends"
        );
    }
}
