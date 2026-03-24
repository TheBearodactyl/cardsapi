FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

FROM alpine:3.21

RUN adduser -D -h /app appuser
COPY --from=builder /app/target/release/cardsapi /usr/local/bin/cardsapi

USER appuser
EXPOSE 8000

ENV ROCKET_ADDRESS=0.0.0.0

CMD ["cardsapi"]
