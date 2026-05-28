FROM rust:1.88-bookworm AS builder
WORKDIR /usr/src/jbx
COPY . .
RUN cargo build --release --locked --bin jbx

FROM eclipse-temurin:25-jdk-jammy
COPY --from=builder /usr/src/jbx/target/release/jbx /usr/local/bin/jbx
ENTRYPOINT ["jbx"]
CMD ["--help"]
