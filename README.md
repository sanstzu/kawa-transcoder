# kawa-transcoder
> This README file is under construction, expect incomplete information 🚧

## Setup
In order to run this service, make sure you have installed:
- cargo
- ffmpeg
- aws-cli

Once it has been installed, then run the following command to build:

```bash
cargo build --release
```

and this following command to run the service:

```bash
cargo run
```

This service would listen on port `50051` for any gRPC requests from the RTMP ingest service.
