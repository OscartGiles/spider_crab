# spider_crab

A simple web crawler written in Rust and using the Tokio async runtime.
It consists of a library and a CLI. The library could be reused for uses in different contexts (e.g. a web service).

I wrote this as part of a coding challenge. Read more about the design decisions I made in the [design overview](https://github.com/OscartGiles/spider_crab/issues/1)

## Install CLI

Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed and then install with:

```bash
cargo install --git https://github.com/OscartGiles/spider_crab
```

## Usage

Get help.
```bash
spider_crab --help
```

Crawl a website.
```bash
spider_crab https://oscartgiles.github.io/
```

### Options
Save the results to file.
```bash
spider_crab https://oscartgiles.github.io/ -o crawl_results.txt
```

Hide links in the output.
```bash
spider_crab https://oscartgiles.github.io/ --hide-links
```

Limit the number of pages visited.
```bash
spider_crab https://docs.rs/ --max-pages 5 --hide-links
```

Crawl for a set period of time (in seconds).
```bash
spider_crab https://docs.rs/ --max-time 1 --hide-links
```

Limit the number of concurrent requests to a domain.

```bash
spider_crab https://docs.rs/ -c 1 --max-time 10  
```

Ignore robots.txt. spider_crab respects it by default.

```bash
spider_crab https://docs.rs/ --ignore-robots --max-time 10
```

## Tracing

The CLI can export traces to an [OTLP collector](https://opentelemetry.io/docs/collector/). For example, you could export traces to [Jaeger](https://www.jaegertracing.io/). To try it out start Jaeger with docker:

```bash
docker run --name spider_crab_jaeger -e COLLECTOR_OTLP_ENABLED=true  -p6831:6831/udp -p6832:6832/udp -p16686:16686 -p14268:14268 -p 4317:4317 jaegertracing/all-in-one:latest
```

and then run the CLI with the `--otl-endpoint` option.
```bash
spider_crab https://oscartgiles.github.io/ --otl-endpoint http://localhost:4317
```

You can then view the logs at [`http://localhost:16686/`](http://localhost:16686/).

Clean up the Jaeger container.
```bash
docker stop spider_crab_jaeger; docker rm spider_crab_jaeger
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

