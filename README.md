# Monzo Crawler

A simple web crawler for the Monzo interview process, written in Rust and using the Tokio async runtime.
It consists of a library and a CLI. The library could be reused for uses in different contexts (e.g. a web service).


## Install CLI

Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed and then install with:

```bash
cargo install --path .
```

## Usage

Get help.
```bash
monzo_crawler --help
```

Crawl a website.
```bash
monzo_crawler https://oscartgiles.github.io/
```

### Options
Save the results to file.
```bash
monzo_crawler https://oscartgiles.github.io/ -o crawl_results.txt
```

Hide links in the output (terminal only).
```bash
monzo_crawler https://oscartgiles.github.io/ --hide-links
```

Limit the number of pages visited.
```bash
monzo_crawler https://monzo.com/ --max-pages 5 --hide-links
```

Crawl for a set period of time (in seconds).
```bash
monzo_crawler https://monzo.com/ --max-time 1 --hide-links
```

Limit the number of concurrent requests to a domain.

```bash
monzo_crawler https://monzo.com/ -c 1  
```

Ignore robots.txt. monzo_crawler respects it by default.

```bash
monzo_crawler https://monzo.com/ --ignore-robots
```

## Library

Run tests.
```bash
cargo test
```

Run benchmarks.
```bash
cargo bench
```

Open library docs.
```bash
cargo doc --open
```

