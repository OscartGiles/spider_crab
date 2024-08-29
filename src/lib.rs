//! spider_crab is a library for crawling websites.
//!
pub mod client_middleware;
mod crawler;
mod parser;
pub use client_middleware::ClientWithMiddlewareVisitor;
pub use crawler::{Crawler, CrawlerBuilder, PageContent, SiteVisitor, VisitorError};
pub use parser::{parse_links, AllPages, Page};
