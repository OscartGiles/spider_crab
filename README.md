
> We'd like you to write a simple web crawler in a programming language you're familiar with. Given a starting URL, the crawler should visit each URL it finds on the same domain. It should print each URL visited, and a list of links found on that page. The crawler should be limited to one subdomain - so when you start with *https://monzo.com/*, it would crawl all pages on the monzo.com website, but not follow external links, for example to facebook.com or community.monzo.com.

> We would like to see your own implementation of a web crawler. Please do not use frameworks like scrapy or go-colly which handle all the crawling behind the scenes or someone else's code. You are welcome to use libraries to handle things like HTML parsing.

> Ideally, write it as you would a production piece of code. This exercise is not meant to show us whether you can write code – we are more interested in how you design software. This means that we care less about a fancy UI or sitemap format, and more about how your program is structured: the trade-offs you've made, what behaviour the program exhibits, and your use of concurrency, test coverage, and so on.


## Scope

A simple webcrawler. I take this to mean:

- [ ] Crawl a website
- [ ] Only follow links in html
- [ ] Do not support SPA/javascript
- [ ] Respect robots.txt
- [ ] Follow links in sitemap

## Performance
It will try to be performant:
    - Use Tokio for I/O
    - Use a threadpool for CPU-bound tasks (parsing html)
    - Do some simple bench marking

## Reliability rate limiting and ethical crawling
- Retry requests on failure (exponential backoff)
- Limit the number of concurrent requests per domain
- Respect Crawl-delay in robots.txt
- Use a randomised delay?
- Handle HTTP status code (e.g. 429/ 503)


## Tests
- Unit tests for parsers etc
- Test failure cases?
- Integration test with a self hosted site...
- Test against another crawler

### Components:
- HTML Parser
