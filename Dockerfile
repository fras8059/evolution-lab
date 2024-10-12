# Stage 1: Build
FROM rust:1.81.0-alpine as base
# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev
# Install cargo-chef globally
RUN cargo install cargo-chef
# Set the working directory inside the container
WORKDIR /app

FROM base AS recipe
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder 
COPY --from=recipe /app/recipe.json recipe.json
# Build dependencies (this is the caching Docker layer)
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM alpine:3.20

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/evolution-api /usr/local/bin/evolution-api

# Expose the application port
EXPOSE 80

# Run the application
CMD ["/usr/local/bin/evolution-api"]