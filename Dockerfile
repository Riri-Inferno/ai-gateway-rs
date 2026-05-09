ARG RUST_VERSION=1.95
ARG APP_NAME=server

# cargo-chefを使った依存関係キャッシング
FROM lukemathwalker/cargo-chef:latest-rust-${RUST_VERSION} AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# 依存関係のビルド（キャッシュ可能）
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

# ===================================
# ビルドステージ
# ===================================
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --bin ${APP_NAME} && \
    cp ./target/release/${APP_NAME} /bin/server

# ===================================
# テストステージ
# ===================================
FROM chef AS test
COPY . .

# ===================================
# 実行ステージ（本番）
# ===================================
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /bin/server /app/
WORKDIR /app
EXPOSE 8080
ENTRYPOINT ["/app/server"]
