// NOTE: This code is intentionally simple and is not meant to be a production-ready application.
// The purpose of this repository is to demonstrate optimized vs. unoptimized GitHub Actions CI/CD
// workflows for Rust projects.
// The actual functionality of this load balancer proxy is unimportant.
// Focus on the build configuration and CI workflow files instead.

use async_trait::async_trait;
use pingora::prelude::*;
use rocksdb::DB;
use std::sync::Arc;

const UPSTREAM_HOST: &str = "one.one.one.one";
const REQUEST_COUNT_KEY: &str = "request_count";

pub struct LB {
    upstreams: Arc<LoadBalancer<RoundRobin>>,
    stats: Arc<DB>,
}

fn increment_counter(db: &DB, key: &str) -> u64 {
    let current = db
        .get(key)
        .ok()
        .flatten()
        .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
        .map(u64::from_be_bytes)
        .unwrap_or(0);

    let next = current + 1;
    let _ = db.put(key, next.to_be_bytes());
    next
}

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
            .upstreams
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
        let count = increment_counter(&self.stats, REQUEST_COUNT_KEY);
        println!("proxied request #{count}");

        upstream_request.insert_header("Host", UPSTREAM_HOST)?;
        Ok(())
    }
}

fn main() {
    let mut server = Server::new(None).expect("failed to create server");
    server.bootstrap();

    let upstreams =
        LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).expect("invalid upstreams");
    let stats = DB::open_default("proxy_stats_db").expect("failed to open stats database");

    let lb = LB {
        upstreams: Arc::new(upstreams),
        stats: Arc::new(stats),
    };
    let mut proxy = http_proxy_service(&server.configuration, lb);
    proxy.add_tcp("0.0.0.0:6188");

    server.add_service(proxy);
    server.run_forever();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rust_build_demo_test_db_{nanos}"))
    }

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

    #[test]
    fn increment_counter_persists_and_returns_running_total() {
        // Arrange
        let path = temp_db_path();
        let db = DB::open_default(&path).unwrap();
        increment_counter(&db, "hits");

        // Act
        let second = increment_counter(&db, "hits");

        // Assert
        assert_eq!(second, 2);
        let stored = db.get("hits").unwrap().unwrap();
        assert_eq!(u64::from_be_bytes(stored.try_into().unwrap()), 2);
        drop(db);
        let _ = DB::destroy(&rocksdb::Options::default(), &path);
    }
}
