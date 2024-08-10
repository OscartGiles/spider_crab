# Monzo Crawler 🕸️

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

Hide links in the output.
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

## Tracing

The CLI can export traces to an [OTLP collector](https://opentelemetry.io/docs/collector/). For example, you could export traces to [Jaeger](https://www.jaegertracing.io/). To try it out start Jaeger with docker:

```bash
docker run --name monzo_crawl_jaeger -e COLLECTOR_OTLP_ENABLED=true  -p6831:6831/udp -p6832:6832/udp -p16686:16686 -p14268:14268 -p 4317:4317 jaegertracing/all-in-one:latest
```

and then run the CLI with the `--otl-endpoint` option.
```bash
monzo_crawler https://oscartgiles.github.io/ --otl-endpoint http://localhost:4317
```

You can then view the logs at [`http://localhost:16686/`](http://localhost:16686/).

Clean up the Jaeger container.
```bash
docker stop monzo_crawl_jaeger; docker rm monzo_crawl_jaeger
```


## Library

Run tests.
```bash
cargo test
```

Run benchmarks (currently only http parsing is benchmarked) and open an html report.
```bash
cargo bench
open ./target/criterion/report/index.html
```

Open library docs.
```bash
cargo doc --open
```

