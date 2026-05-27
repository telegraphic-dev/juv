FROM rust:1.88-bookworm AS builder
WORKDIR /usr/src/juv
COPY . .
RUN cargo build --release --locked --bins

FROM eclipse-temurin:25-jdk-jammy
COPY --from=builder /usr/src/juv/target/release/juv /usr/local/bin/juv
COPY --from=builder /usr/src/juv/target/release/juvx /usr/local/bin/juvx
ENTRYPOINT ["juv"]
CMD ["--help"]