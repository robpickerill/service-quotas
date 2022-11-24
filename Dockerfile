FROM rust:1.65 AS builder

RUN update-ca-certificates

ENV USER=service-quotas
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /service-quotas

COPY ./ .

RUN cargo build --release

FROM debian:bullseye-slim

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /service-quotas

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /service-quotas/target/release/service-quotas ./

USER service-quotas:service-quotas

ENTRYPOINT ["/service-quotas/service-quotas"]