//! Web source connector, backed by the [MudFish](https://github.com/Mullassery/MudFish)
//! crawling engine.
//!
//! Reads a website (bounded by depth and a hard URL cap) and emits one
//! [`Record`] per crawled page, so a sync can pull web content into any
//! destination the ETL already supports. The crawl is performed by
//! `mudfish_engine::crawl`, which gives the same SSRF-safe (post-resolution
//! address filtering), robots-respecting, budget-bounded behavior the MudFish
//! CLI/Python surface uses — the connector adds no fetching of its own.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{Capability, ConnectionTest, Field, Record, RecordMetadata, RecordOperation, Schema};

/// Configuration for the web source connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Seed URL the crawl starts from (must be `http`/`https`).
    pub seed_url: String,
    /// Maximum link-following depth. `0` fetches only the seed page.
    pub max_depth: usize,
    /// Hard cap on the number of pages fetched.
    pub max_urls: usize,
    /// Restrict the crawl to the seed URL's (resolved) domain.
    pub same_domain: bool,
    /// Render JS-dependent pages in headless Chrome. Off by default: a browser
    /// render is far heavier and needs Chrome on the host.
    pub render_js: bool,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            seed_url: String::new(),
            max_depth: 1,
            max_urls: 25,
            same_domain: true,
            render_js: false,
        }
    }
}

/// Web source connector.
pub struct WebSource {
    pub config: WebConfig,
}

impl WebSource {
    pub fn new(config: WebConfig) -> Self {
        Self { config }
    }

    /// Build the MudFish crawl config this connector always runs with.
    fn crawl_config(&self) -> crate::Result<mudfish_core::CrawlConfig> {
        let seed = url::Url::parse(&self.config.seed_url).map_err(|e| {
            crate::Error::ConfigError(format!(
                "invalid web seed_url '{}': {e}",
                self.config.seed_url
            ))
        })?;
        if seed.scheme() != "http" && seed.scheme() != "https" {
            return Err(crate::Error::ConfigError(format!(
                "web seed_url scheme must be http or https, got '{}'",
                seed.scheme()
            )));
        }

        let cfg = mudfish_core::CrawlConfig {
            seeds: vec![seed],
            max_depth: self.config.max_depth,
            max_urls: Some(self.config.max_urls as u64),
            same_domain: self.config.same_domain,
            render_js: self.config.render_js,
            // Never allow private/link-local targets for a connector that may
            // be pointed at user-supplied URLs (the engine's default, made
            // explicit).
            allow_private_networks: false,
            ..Default::default()
        };
        Ok(cfg)
    }

    /// Run the crawl and convert each page to a `Record`.
    async fn crawl_records(&self) -> crate::Result<Vec<Record>> {
        let cfg = self.crawl_config()?;
        let result = mudfish_engine::crawl(&cfg)
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("web crawl failed: {e}")))?;

        let received_at = chrono::Utc::now().to_rfc3339();
        Ok(result
            .pages
            .into_iter()
            .map(|page| {
                let links: Vec<String> = page.links.iter().map(|l| l.url.to_string()).collect();
                let data = serde_json::json!({
                    "url": page.final_url.to_string(),
                    "status_code": page.status_code,
                    "content_type": page.content_type,
                    "depth": page.depth,
                    "title": page.metadata.title,
                    "meta_description": page.metadata.meta_description,
                    "links": links,
                    "body_bytes": page.body_bytes,
                });
                Record {
                    id: format!("web:{}", page.final_url),
                    data,
                    metadata: RecordMetadata {
                        source: "web".to_string(),
                        source_timestamp: None,
                        received_at: received_at.clone(),
                        operation: RecordOperation::Upsert,
                    },
                }
            })
            .collect())
    }
}

#[async_trait]
impl super::SourceConnector for WebSource {
    fn name(&self) -> &str {
        "Web (MudFish)"
    }

    fn description(&self) -> &str {
        "Crawl a website with the MudFish engine and read one record per page"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        // A real fetch of the seed (no link-following) is the honest
        // connection test for a web source.
        let mut probe = self.config.clone();
        probe.max_depth = 0;
        probe.max_urls = 1;
        let source = WebSource { config: probe };
        match source.crawl_records().await {
            Ok(records) if !records.is_empty() => Ok(ConnectionTest::success(&format!(
                "Fetched {} with MudFish",
                self.config.seed_url
            ))),
            Ok(_) => Ok(ConnectionTest::failure("seed URL returned no pages", None)),
            Err(e) => Ok(ConnectionTest::failure(
                "web crawl failed",
                Some(e.to_string()),
            )),
        }
    }

    async fn detect_schema(&self) -> crate::Result<Schema> {
        let fields = [
            ("url", "string"),
            ("status_code", "integer"),
            ("content_type", "string"),
            ("depth", "integer"),
            ("title", "string"),
            ("meta_description", "string"),
            ("links", "array<string>"),
            ("body_bytes", "integer"),
        ]
        .iter()
        .map(|(name, field_type)| Field {
            name: name.to_string(),
            field_type: field_type.to_string(),
            required: false,
            description: None,
        })
        .collect();

        Ok(Schema {
            fields,
            sample_records: vec![],
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        self.crawl_records().await
    }

    async fn read_batch(&self, offset: u64, limit: u64) -> crate::Result<Vec<Record>> {
        let all = self.crawl_records().await?;
        Ok(all
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn read_incremental(&self, _last_value: &str) -> crate::Result<Vec<Record>> {
        Err(crate::Error::ConfigError(
            "incremental reads are not implemented for the web connector; a crawl \
             returns the current state of the site, not a change feed"
                .to_string(),
        ))
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Read,
            Capability::SchemaDetection,
            Capability::Batch,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::SourceConnector;

    #[test]
    fn rejects_non_http_scheme() {
        let source = WebSource::new(WebConfig {
            seed_url: "ftp://example.com".to_string(),
            ..Default::default()
        });
        assert!(source.crawl_config().is_err());
    }

    #[test]
    fn rejects_invalid_url() {
        let source = WebSource::new(WebConfig {
            seed_url: "not a url".to_string(),
            ..Default::default()
        });
        assert!(source.crawl_config().is_err());
    }

    #[test]
    fn builds_a_budgeted_ssrf_safe_config() {
        let source = WebSource::new(WebConfig {
            seed_url: "https://example.com".to_string(),
            max_depth: 2,
            max_urls: 7,
            same_domain: false,
            render_js: false,
        });
        let cfg = source.crawl_config().unwrap();
        assert_eq!(cfg.seeds.len(), 1);
        assert_eq!(cfg.max_depth, 2);
        assert_eq!(cfg.max_urls, Some(7));
        assert!(!cfg.same_domain);
        assert!(
            !cfg.allow_private_networks,
            "web connector must never permit private/link-local targets"
        );
    }

    #[test]
    fn reports_read_capabilities() {
        let source = WebSource::new(WebConfig::default());
        let caps = source.capabilities();
        assert!(caps.contains(&Capability::Read));
        assert!(caps.contains(&Capability::SchemaDetection));
    }
}
