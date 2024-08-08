use std::collections::HashSet;

use scraper::{Html, Selector};
use url::Url;

/// Get all unique links that are from the same domain as the `page_url`.
/// Excludes any links that do not use http or https scheme.
/// Fragments are not treated as unique links.
pub fn parse_links(html_content: &str, page_url: &Url) -> HashSet<Url> {
    let document = Html::parse_document(html_content);
    let selector = Selector::parse("a").unwrap();

    document
        .select(&selector)
        .filter(|a| a.value().attr("href").is_some())
        .map(|a| {
            a.value()
                .attr("href")
                .expect("href not found. This is a bug. None's should be filtered out.")
        })
        .filter(|href| !href.starts_with('#'))
        .map(|href| {
            if href.starts_with('/') {
                page_url.join(href).unwrap()
            } else {
                Url::parse(href).unwrap()
            }
        })
        .filter(|url| url.domain() == page_url.domain())
        .filter(|url| url.scheme() == "https" || url.scheme() == "http")
        .map(|mut href| {
            href.set_fragment(None);
            href
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_links;
    use std::{collections::HashSet, fs};
    use url::Url;

    #[test]
    fn test_link_parser() {
        let html = r#"
    <!DOCTYPE html>
    <meta charset="utf-8">
    <title>Hello, world!</title>
    <h1 class="foo">Hello, <i>world!</i></h1>
    <a href="https://monzo.com/hi">Monzo https</a>
    <a href="http://monzo.com/hi">Monzo http</a>
    <a href="ftp://monzo.com/hi">Don't include any links that don't use http or https scheme</a>
    <div>
        <p>foo</p>
        <p>bar</p>
        <p><a href="/nested-deeper">baz</a></p>
        <p><a href="/nested-deeper">duplicate</a></p>
        <p><a href="/fragments-not-unique#first">first fragment</a></p>
        <p><a href="/fragments-not-unique#second">second fragment</a></p>
        <a href="https://notmonozo.com/opps">dont include other domains</a>
        <a href="https://sudomain.monzo.com/hi">don't include subdomains</a>
    </div>
"#;

        let links = parse_links(html, &Url::parse("https://monzo.com").unwrap());

        let expected_links: HashSet<Url> = HashSet::from([
            "https://monzo.com/hi",
            "http://monzo.com/hi",
            "https://monzo.com/nested-deeper",
            "https://monzo.com/fragments-not-unique",
        ])
        .iter()
        .map(|&url| Url::parse(url).unwrap())
        .collect();

        assert_eq!(links, expected_links);
    }

    #[test]
    fn test_parse_monzo() {
        let html = fs::read_to_string("./tests/test_data/monzo/home.html").unwrap();

        let links = parse_links(&html, &Url::parse("https://monzo.com").unwrap());

        for element in links {
            println!("{}", element.as_str());
        }
    }
}
