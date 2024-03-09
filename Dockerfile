FROM rust:slim-buster AS rust
WORKDIR /usr/src/katbin
COPY . .
RUN cargo install --path .
RUN cp target/release/katbin ./katbin

# build tailwind
FROM node:20-slim AS tailwind
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable
WORKDIR /usr/src/katbin
COPY --from=rust /usr/src/katbin/api/templates ./api/templates
COPY --from=rust /usr/src/katbin/api/static ./api/static
COPY --from=rust /usr/src/katbin/tailwind ./tailwind
WORKDIR /usr/src/katbin/tailwind
RUN pnpm install --frozen-lockfile
RUN pnpm run build-css-prod

# final image
FROM debian:buster-slim AS final
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /usr/src/katbin
COPY --from=rust /usr/src/katbin/katbin ./
COPY --from=rust /usr/src/katbin/api/templates ./api/templates
COPY --from=tailwind /usr/src/katbin/api/static ./api/static

CMD [ "/usr/src/katbin/katbin" ]