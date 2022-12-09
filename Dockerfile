FROM rust:1.65 as build
RUN apt update && apt install -y \
    build-essential \
    libhyperscan-dev \
    pkg-config git
WORKDIR "/noseyparker"
COPY . .
RUN cargo build --release

FROM debian:11-slim
COPY --from=build /noseyparker/target/release/noseyparker /usr/bin/noseyparker
RUN apt update && apt install -y \
    libhyperscan-dev

ENTRYPOINT ["noseyparker"]